use crate::session::{MediaEvent, MediaSession};
use crate::source::{MediaSource, SourceUpdate};
use crate::track::PlaybackState;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast, mpsc};

pub struct MediaHub {
    sessions: Arc<Mutex<HashMap<String, MediaSession>>>,
    event_tx: broadcast::Sender<MediaEvent>,
}

impl MediaHub {
    pub fn new(sources: Vec<Arc<dyn MediaSource>>) -> Self {
        let (event_tx, _) = broadcast::channel(64);
        let (update_tx, mut update_rx) = mpsc::channel::<SourceUpdate>(128);
        let sessions = Arc::new(Mutex::new(HashMap::new()));

        for source in &sources {
            source.start(update_tx.clone());
        }

        let sessions_clone = Arc::clone(&sessions);
        let event_tx_clone = event_tx.clone();

        tokio::spawn(async move {
            let mut last_emitted_id: Option<String> = None;

            while let Some(update) = update_rx.recv().await {
                let mut map = sessions_clone.lock().await;

                match update {
                    SourceUpdate::Upsert(session) => {
                        map.insert(session.id.clone(), session);
                    }
                    SourceUpdate::Remove(id) => {
                        map.remove(&id);
                    }
                }

                let active = Self::select_active_session(&map, |_| true);
                let current_id = active.as_ref().map(|s| s.id.clone());

                if current_id != last_emitted_id {
                    last_emitted_id = current_id;
                    let _ = event_tx_clone.send(MediaEvent::SessionChanged(active));
                } else if let Some(ref current) = active {
                    let _ = event_tx_clone.send(MediaEvent::MetadataChanged(current.clone()));
                }
            }
        });

        Self { sessions, event_tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<MediaEvent> {
        self.event_tx.subscribe()
    }

    pub async fn active_session(&self) -> Option<MediaSession> {
        let map = self.sessions.lock().await;
        Self::select_active_session(&map, |_| true)
    }

    /// Returns the most relevant active session matching a specific classification filter.
    pub async fn active_by_kind<F>(&self, filter: F) -> Option<MediaSession>
    where
        F: Fn(&MediaSession) -> bool,
    {
        let map = self.sessions.lock().await;
        Self::select_active_session(&map, filter)
    }

    fn select_active_session<F>(
        map: &HashMap<String, MediaSession>,
        filter: F,
    ) -> Option<MediaSession>
    where
        F: Fn(&MediaSession) -> bool,
    {
        let has_extension_active = map
            .values()
            .any(|s| s.id.starts_with("extension:") && s.state.is_active());

        map.values()
            .filter(|s| s.state.is_active() && !s.is_empty())
            // If extension is providing rich data, ignore duplicate raw browser MPRIS sessions
            .filter(|s| {
                if has_extension_active && s.id.starts_with("org.mpris.MediaPlayer2.") {
                    let id_lower = s.id.to_ascii_lowercase();
                    let is_browser = id_lower.contains("chromium")
                        || id_lower.contains("chrome")
                        || id_lower.contains("firefox")
                        || id_lower.contains("brave")
                        || id_lower.contains("edge");

                    if is_browser {
                        return false;
                    }
                }
                true
            })
            .filter(|s| filter(s))
            .max_by_key(|s| (s.state == PlaybackState::Playing, s.last_interaction))
            .cloned()
    }
}
