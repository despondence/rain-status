use crate::classifier::{ArtworkResolver, MediaKind};
use discord_ipc::IpcStream;
use discord_ipc::model::{Activity, ActivityAssets, ActivityTimestamps, ActivityType};
use rain_media::session::MediaSession;
use rain_media::track::PlaybackState;
use std::error::Error;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_ARTWORK: &str = "https://placehold.co/512x512/1a1b26/ffffff.png?text=Media";

#[derive(Clone, Debug, Default, PartialEq)]
struct ActivityCache {
    title: String,
    state: String,
    artwork: String,
    is_paused: bool,
    anchor_start_ms: Option<u64>,
}

pub struct PresenceSlot {
    kind: MediaKind,
    ipc: Option<IpcStream>,
    pid: u32,
    has_active_presence: bool,
    last_activity: Option<ActivityCache>,
    resolver: ArtworkResolver,
}

impl PresenceSlot {
    pub fn new(kind: MediaKind, resolver: ArtworkResolver) -> Self {
        Self {
            kind,
            ipc: None,
            pid: process::id(),
            has_active_presence: false,
            last_activity: None,
            resolver,
        }
    }

    pub async fn sync(
        &mut self,
        session: Option<&MediaSession>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let Some(session) = session else {
            return self.clear().await;
        };

        if session.state == PlaybackState::Stopped || session.is_empty() {
            return self.clear().await;
        }

        let is_paused = session.state == PlaybackState::Paused;
        let artist_display = session
            .track
            .display_artist()
            .unwrap_or_else(|| session.source_name.clone());

        let raw_state = if is_paused {
            format!("(paused) {artist_display}")
        } else {
            artist_display
        };

        let details = sanitize_string(&session.track.title, "Unknown Title");
        let state_text = sanitize_string(&raw_state, "Unknown Artist");

        // Dynamically resolve HTTPS artwork (handles file:// from MPRIS)
        let image_url = self
            .resolver
            .resolve(session)
            .await
            .unwrap_or_else(|| DEFAULT_ARTWORK.to_string());

        // Accurate live timeline calculation: start_time = now - position
        let now_ms = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;
        let current_pos_ms = session.timeline.current_position().as_millis() as u64;
        let start_ms = if is_paused {
            None
        } else {
            Some(now_ms.saturating_sub(current_pos_ms))
        };

        // Deduplication Check
        if let Some(ref last) = self.last_activity {
            let is_same_metadata = last.title == details
                && last.state == state_text
                && last.artwork == image_url
                && last.is_paused == is_paused;

            let is_same_timeline = match (last.anchor_start_ms, start_ms) {
                (Some(last_start), Some(curr_start)) => curr_start.abs_diff(last_start) < 2000,
                (None, None) => true,
                _ => false,
            };

            if is_same_metadata && is_same_timeline {
                return Ok(());
            }
        }

        self.ensure_connected().await?;

        let timestamps = start_ms.map(|start| {
            let end = session
                .track
                .duration
                .map(|dur| start + dur.as_millis() as u64);
            ActivityTimestamps {
                start: Some(start),
                end,
            }
        });

        let activity = Activity {
            application_id: Some(self.kind.discord_app_id()),
            platform: Some("desktop".to_string()),
            details: Some(details.clone()),
            state: Some(state_text.clone()),
            activity_type: match self.kind {
                MediaKind::Music => ActivityType::Listening,
                MediaKind::Video => ActivityType::Watching,
            },
            status_display_type: Some(2),
            timestamps,
            assets: Some(ActivityAssets {
                large_image: Some(image_url.clone()),
                large_text: Some(details.clone()),
                small_image: None,
                small_text: None,
            }),
        };

        if let Some(ipc) = self.ipc.as_mut() {
            match ipc.set_activity(self.pid, Some(activity)).await {
                Ok(_) => {
                    self.has_active_presence = true;
                    self.last_activity = Some(ActivityCache {
                        title: details,
                        state: state_text,
                        artwork: image_url,
                        is_paused,
                        anchor_start_ms: start_ms,
                    });
                }
                Err(e) => {
                    eprintln!(
                        "[Discord:{:?}] SetActivity failed: {e}. Resetting socket...",
                        self.kind
                    );
                    self.ipc = None;
                    self.has_active_presence = false;
                    self.last_activity = None;
                }
            }
        }

        Ok(())
    }

    pub async fn clear(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        if !self.has_active_presence {
            return Ok(());
        }

        if let Some(ipc) = self.ipc.as_mut() {
            let _ = ipc.clear_activity(self.pid).await;
            self.has_active_presence = false;
            self.last_activity = None;
        }
        Ok(())
    }

    async fn ensure_connected(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        if self.ipc.is_some() {
            return Ok(());
        }

        let app_id = self.kind.discord_app_id();
        let socket_path = discord_ipc::env::ipc_path();

        let mut ipc = IpcStream::connect(&socket_path).await.map_err(|e| {
            format!(
                "[Discord:{:?}] Socket connect failed at {socket_path:?}: {e}",
                self.kind
            )
        })?;

        ipc.handshake(app_id).await.map_err(|e| {
            format!(
                "[Discord:{:?}] Handshake rejected for App ID {app_id}: {e}",
                self.kind
            )
        })?;

        println!(
            "[Discord:{:?}] Connected & Handshake OK (App ID: {app_id})",
            self.kind
        );
        self.ipc = Some(ipc);
        Ok(())
    }
}

pub struct DiscordBridge {
    music_slot: PresenceSlot,
    video_slot: PresenceSlot,
}

impl DiscordBridge {
    pub fn new(resolver: ArtworkResolver) -> Self {
        Self {
            music_slot: PresenceSlot::new(MediaKind::Music, resolver.clone()),
            video_slot: PresenceSlot::new(MediaKind::Video, resolver),
        }
    }

    pub async fn sync_music(
        &mut self,
        session: Option<&MediaSession>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.music_slot.sync(session).await
    }

    pub async fn sync_video(
        &mut self,
        session: Option<&MediaSession>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.video_slot.sync(session).await
    }
}

/// Enforces Discord RPC 2–128 character constraints
fn sanitize_string(s: &str, default: &str) -> String {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return default.to_string();
    }
    if trimmed.len() < 2 {
        return format!("{trimmed} ");
    }
    if trimmed.len() > 128 {
        return trimmed.chars().take(128).collect();
    }
    trimmed.to_string()
}
