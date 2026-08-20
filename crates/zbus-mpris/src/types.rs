// Copyright (C) 2026 rain (despondence) <308736589+despondence@users.noreply.github.com>
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fmt;
use std::str::FromStr;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum PlaybackStatus {
    #[default]
    Stopped,
    Playing,
    Paused,
}

impl PlaybackStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Playing => "Playing",
            Self::Paused => "Paused",
            Self::Stopped => "Stopped",
        }
    }
}

impl fmt::Display for PlaybackStatus {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{}", self.as_str())
    }
}

impl FromStr for PlaybackStatus {
    type Err = std::convert::Infallible;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let value = match string {
            "Playing" => Self::Playing,
            "Paused" => Self::Paused,
            _ => Self::Stopped,
        };

        Ok(value)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum LoopStatus {
    #[default]
    None,
    Track,
    Playlist,
}

impl LoopStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Track => "Track",
            Self::Playlist => "Playlist",
        }
    }
}

impl fmt::Display for LoopStatus {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{}", self.as_str())
    }
}

impl FromStr for LoopStatus {
    type Err = std::convert::Infallible;

    fn from_str(fmt: &str) -> Result<Self, Self::Err> {
        let value = match fmt {
            "Track" => Self::Track,
            "Playlist" => Self::Playlist,
            _ => Self::None,
        };

        Ok(value)
    }
}
