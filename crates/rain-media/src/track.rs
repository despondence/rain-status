use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct MediaTrack {
    pub title: String,
    pub artists: Vec<String>,
    pub album: Option<String>,
    pub duration: Option<Duration>,
    pub artwork_url: Option<String>,
    pub source_url: Option<String>,
}

impl MediaTrack {
    pub fn display_artist(&self) -> Option<String> {
        if self.artists.is_empty() {
            None
        } else {
            Some(self.artists.join(", "))
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PlaybackState {
    #[default]
    Stopped,
    Playing,
    Paused,
}

impl PlaybackState {
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Playing | Self::Paused)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Capabilities {
    pub can_play: bool,
    pub can_pause: bool,
    pub can_next: bool,
    pub can_previous: bool,
    pub can_seek: bool,
}
