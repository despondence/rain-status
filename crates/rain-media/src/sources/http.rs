use crate::session::MediaSession;
use crate::source::{MediaSource, SourceUpdate};
use crate::timeline::Timeline;
use crate::track::{Capabilities, MediaTrack, PlaybackState};
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ExtensionPayload {
    pub source_hint: Option<String>,
    pub title: String,
    pub artist: Option<String>,
    pub artwork: Option<String>,
    pub url: Option<String>,
    pub duration_secs: Option<f64>,
    pub position_secs: Option<f64>,
    pub state: Option<PlaybackState>,
    pub tab_id: Option<i64>,
}

#[derive(Clone)]
pub struct HttpSource {
    port: u16,
}

impl HttpSource {
    pub const fn new(port: u16) -> Self {
        Self { port }
    }
}

impl MediaSource for HttpSource {
    fn name(&self) -> &'static str {
        "http-extension"
    }

    fn start(&self, sink: mpsc::Sender<SourceUpdate>) -> tokio::task::JoinHandle<()> {
        let port = self.port;
        tokio::spawn(async move {
            let app = Router::new()
                .route("/update-status", post(handle_update_status))
                .with_state(sink);

            match tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, port)).await {
                Ok(listener) => {
                    println!("[HTTP Source] Listening for browser extension on port {port}");
                    let _ = axum::serve(listener, app).await;
                }
                Err(e) => {
                    eprintln!("[HTTP Source] Failed to bind port {port}: {e}");
                }
            }
        })
    }
}

async fn handle_update_status(
    State(sink): State<mpsc::Sender<SourceUpdate>>,
    Json(payload): Json<ExtensionPayload>,
) -> StatusCode {
    // Ensure extension session IDs always start with "extension:"
    let session_id = payload
        .source_hint
        .as_deref()
        .map(|h| format!("extension:{h}"))
        .or_else(|| payload.tab_id.map(|id| format!("extension:tab:{id}")))
        .unwrap_or_else(|| "extension:browser".to_string());

    let state = payload.state.unwrap_or(PlaybackState::Playing);

    if state == PlaybackState::Stopped || payload.title.trim().is_empty() {
        let _ = sink.send(SourceUpdate::Remove(session_id)).await;
        return StatusCode::OK;
    }

    let source_name = extract_friendly_name(payload.url.as_deref());
    let duration = payload.duration_secs.map(Duration::from_secs_f64);
    let position = payload
        .position_secs
        .map(Duration::from_secs_f64)
        .unwrap_or(Duration::ZERO);

    let is_playing = state == PlaybackState::Playing;

    let track = MediaTrack {
        title: payload.title,
        artists: payload.artist.into_iter().collect(),
        album: None,
        duration,
        artwork_url: payload.artwork,
        source_url: payload.url,
    };

    let session = MediaSession {
        id: session_id,
        source_name,
        track,
        state,
        timeline: Timeline::new(position, duration, 1.0, is_playing),
        capabilities: Capabilities {
            can_play: true,
            can_pause: true,
            can_next: true,
            can_previous: true,
            can_seek: true,
        },
        last_interaction: Instant::now(),
    };

    if sink.send(SourceUpdate::Upsert(session)).await.is_ok() {
        StatusCode::OK
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

fn extract_friendly_name(url: Option<&str>) -> String {
    if let Some(u) = url
        && let Ok(parsed) = url::Url::parse(u)
        && let Some(host) = parsed.host_str()
    {
        return host.trim_start_matches("www.").to_string();
    }
    "Browser Extension".to_string()
}
