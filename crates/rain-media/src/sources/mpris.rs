use crate::session::MediaSession;
use crate::source::{MediaSource, SourceUpdate};
use crate::timeline::Timeline;
use crate::track::{Capabilities, MediaTrack, PlaybackState};
use futures::StreamExt;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, mpsc, watch};
use zbus::Connection;
use zbus::fdo::DBusProxy;
use zbus_mpris::{MPRIS_PREFIX, MprisMetadata, MprisPlayerProxy};

#[derive(Clone, Default)]
pub struct MprisSource {
    monitors: Arc<Mutex<HashMap<String, watch::Sender<bool>>>>,
}

impl MprisSource {
    pub fn new() -> Self {
        Self::default()
    }
}

impl MediaSource for MprisSource {
    fn name(&self) -> &'static str {
        "mpris"
    }

    fn start(&self, sink: mpsc::Sender<SourceUpdate>) -> tokio::task::JoinHandle<()> {
        let monitors = Arc::clone(&self.monitors);

        tokio::spawn(async move {
            let connection = match Connection::session().await {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("[MPRIS] Failed to connect to D-Bus session: {e}");
                    return;
                }
            };

            let dbus = match DBusProxy::new(&connection).await {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("[MPRIS] Failed to create DBusProxy: {e}");
                    return;
                }
            };

            let mut owner_changes = match dbus.receive_name_owner_changed().await {
                Ok(stream) => stream,
                Err(e) => {
                    eprintln!("[MPRIS] Failed listening for D-Bus owner changes: {e}");
                    return;
                }
            };

            // Discover currently active players
            if let Ok(names) = dbus.list_names().await {
                let mut map = monitors.lock().await;
                for name in names {
                    if name.starts_with(MPRIS_PREFIX) {
                        let (shutdown_tx, shutdown_rx) = watch::channel(false);
                        map.insert(name.to_string(), shutdown_tx);
                        spawn_monitor(
                            connection.clone(),
                            name.to_string(),
                            sink.clone(),
                            shutdown_rx,
                        );
                    }
                }
            }

            // Supervise player lifecycles (joins/exits)
            while let Some(signal) = owner_changes.next().await {
                let Ok(args) = signal.args() else { continue };
                let name = args.name();

                if !name.starts_with(MPRIS_PREFIX) {
                    continue;
                }

                let name_str = name.to_string();
                let mut map = monitors.lock().await;

                if args.new_owner().is_none() {
                    // Player disconnected from D-Bus
                    if let Some(shutdown_tx) = map.remove(&name_str) {
                        let _ = shutdown_tx.send(true);
                    }
                    let _ = sink.send(SourceUpdate::Remove(name_str)).await;
                } else if args.old_owner().is_none() && !map.contains_key(&name_str) {
                    // New player joined D-Bus
                    let (shutdown_tx, shutdown_rx) = watch::channel(false);
                    map.insert(name_str.clone(), shutdown_tx);
                    spawn_monitor(connection.clone(), name_str, sink.clone(), shutdown_rx);
                }
            }
        })
    }
}

fn spawn_monitor(
    connection: Connection,
    name: String,
    sink: mpsc::Sender<SourceUpdate>,
    shutdown_rx: watch::Receiver<bool>,
) {
    tokio::spawn(async move {
        let _ = monitor_player_lifecycle(connection, name.clone(), sink.clone(), shutdown_rx).await;
        let _ = sink.send(SourceUpdate::Remove(name)).await;
    });
}

async fn monitor_player_lifecycle(
    connection: Connection,
    name: String,
    sink: mpsc::Sender<SourceUpdate>,
    mut shutdown_rx: watch::Receiver<bool>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let player = MprisPlayerProxy::builder(&connection)
        .destination(name.clone())?
        .build()
        .await?;

    let mut metadata_stream = player.receive_metadata_changed().await;
    let mut status_stream = player.receive_playback_status_changed().await;
    let mut seeked_stream = player.receive_seeked().await?;

    // Initial snapshot
    if let Ok(update) = fetch_player_session(&player).await {
        let _ = sink.send(update).await;
    }

    loop {
        tokio::select! {
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() { break; }
            }
            Some(_) = metadata_stream.next() => {
                if let Ok(update) = fetch_player_session(&player).await {
                    let _ = sink.send(update).await;
                }
            }
            Some(_) = status_stream.next() => {
                if let Ok(update) = fetch_player_session(&player).await {
                    let _ = sink.send(update).await;
                }
            }
            Some(signal) = seeked_stream.next() => {
                if let Ok(args) = signal.args()
                    && let Some(pos) = micros_to_duration(args.position)
                    && let Ok(SourceUpdate::Upsert(mut session)) = fetch_player_session(&player).await
                {
                    session.timeline.position = pos;
                    session.timeline.updated_at = Instant::now();
                    session.last_interaction = Instant::now();
                    let _ = sink.send(SourceUpdate::Upsert(session)).await;
                }
            }
            else => break,
        }
    }

    Ok(())
}

async fn fetch_player_session(
    player: &MprisPlayerProxy<'_>,
) -> Result<SourceUpdate, Box<dyn Error + Send + Sync>> {
    let destination = player.inner().destination().to_string();
    let raw_meta = player.metadata().await.unwrap_or_default();
    let metadata = MprisMetadata::from_metadata(&raw_meta);

    let state = match player.playback_status().await.as_deref() {
        Ok("Playing") => PlaybackState::Playing,
        Ok("Paused") => PlaybackState::Paused,
        _ => PlaybackState::Stopped,
    };

    if state == PlaybackState::Stopped || metadata.is_empty() {
        return Ok(SourceUpdate::Remove(destination));
    }

    let position = player
        .position()
        .await
        .ok()
        .and_then(micros_to_duration)
        .unwrap_or(Duration::ZERO);

    let rate = player.rate().await.unwrap_or(1.0);
    let is_playing = state == PlaybackState::Playing;

    let source_name = extract_mpris_source_name(&destination, metadata.url.as_str());

    let track = MediaTrack {
        title: metadata.title,
        artists: metadata.artist,
        album: (!metadata.album.trim().is_empty()).then_some(metadata.album),
        duration: metadata.length,
        artwork_url: (!metadata.art_url.trim().is_empty()).then_some(metadata.art_url),
        source_url: (!metadata.url.trim().is_empty()).then_some(metadata.url),
    };

    let capabilities = Capabilities {
        can_play: player.can_play().await.unwrap_or(true),
        can_pause: player.can_pause().await.unwrap_or(true),
        can_next: player.can_go_next().await.unwrap_or(false),
        can_previous: player.can_go_previous().await.unwrap_or(false),
        can_seek: player.can_seek().await.unwrap_or(false),
    };

    let session = MediaSession {
        id: destination,
        source_name,
        track,
        state,
        timeline: Timeline::new(position, metadata.length, rate, is_playing),
        capabilities,
        last_interaction: Instant::now(),
    };

    Ok(SourceUpdate::Upsert(session))
}

fn extract_mpris_source_name(destination: &str, url: &str) -> String {
    if !url.trim().is_empty()
        && let Ok(parsed) = url::Url::parse(url)
        && let Some(host) = parsed.host_str()
    {
        return host.trim_start_matches("www.").to_string();
    }

    if let Some(stripped) = destination.strip_prefix(MPRIS_PREFIX) {
        stripped.split('.').next().unwrap_or(stripped).to_string()
    } else {
        destination.to_string()
    }
}

fn micros_to_duration(micros: i64) -> Option<Duration> {
    u64::try_from(micros.max(0)).ok().map(Duration::from_micros)
}
