use rain_media::hub::MediaHub;
use rain_media::session::MediaSession;
use rain_media::sources::{HttpSource, MprisSource};
use std::error::Error;
use std::sync::Arc;

pub mod bridge;
pub mod classifier;

use bridge::DiscordBridge;
use classifier::{ArtworkResolver, MediaKind};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("🚀 Starting Rain Status Multi-Presence Daemon...");

    let mpris = Arc::new(MprisSource::new());
    let http = Arc::new(HttpSource::new(6767));

    let hub = Arc::new(MediaHub::new(vec![mpris, http]));
    let mut events = hub.subscribe();

    let resolver = ArtworkResolver::new();
    let mut discord = DiscordBridge::new(resolver);

    println!("🎧 Media Hub active. Listening for events...");

    while let Ok(_event) = events.recv().await {
        let active_music = hub
            .active_by_kind(|s| MediaKind::classify_session(s) == MediaKind::Music)
            .await;

        let mut active_video = hub
            .active_by_kind(|s| MediaKind::classify_session(s) == MediaKind::Video)
            .await;

        // If both music and video point to the SAME track, suppress the duplicate video presence
        if let (Some(music), Some(video)) = (&active_music, &active_video) {
            if is_same_track(music, video) {
                active_video = None;
            }
        }

        // Synchronize both presences independently
        if let Err(e) = discord.sync_music(active_music.as_ref()).await {
            eprintln!("[Discord Error] Sync music failed: {e}");
        }

        if let Err(e) = discord.sync_video(active_video.as_ref()).await {
            eprintln!("[Discord Error] Sync video failed: {e}");
        }
    }

    Ok(())
}

/// Checks if two media sessions refer to the same media item (from different providers/tabs)
fn is_same_track(a: &MediaSession, b: &MediaSession) -> bool {
    if a.id == b.id {
        return true;
    }

    let title_a = a.track.title.trim().to_ascii_lowercase();
    let title_b = b.track.title.trim().to_ascii_lowercase();

    if title_a.is_empty() || title_b.is_empty() {
        return false;
    }

    let title_matches =
        title_a == title_b || title_a.contains(&title_b) || title_b.contains(&title_a);

    let artist_a = a
        .track
        .display_artist()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let artist_b = b
        .track
        .display_artist()
        .unwrap_or_default()
        .to_ascii_lowercase();

    if !artist_a.is_empty() && !artist_b.is_empty() {
        let artist_matches =
            artist_a == artist_b || artist_a.contains(&artist_b) || artist_b.contains(&artist_a);
        return title_matches && artist_matches;
    }

    title_matches
}
