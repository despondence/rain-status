use crate::timeline::Timeline;
use crate::track::{Capabilities, MediaTrack, PlaybackState};
use std::time::Instant;

#[derive(Clone, Debug, PartialEq)]
pub struct MediaSession {
    pub id: String,
    pub source_name: String,
    pub track: MediaTrack,
    pub state: PlaybackState,
    pub timeline: Timeline,
    pub capabilities: Capabilities,
    pub last_interaction: Instant,
}

impl MediaSession {
    pub fn is_empty(&self) -> bool {
        self.track.title.trim().is_empty() && self.state == PlaybackState::Stopped
    }
}

#[derive(Clone, Debug)]
pub enum MediaEvent {
    SessionChanged(Option<MediaSession>),
    MetadataChanged(MediaSession),
    Seeked { session: MediaSession },
}
