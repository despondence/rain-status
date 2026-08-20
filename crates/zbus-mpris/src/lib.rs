// Copyright (C) 2026 rain (despondence) <308736589+despondence@users.noreply.github.com>
// SPDX-License-Identifier: AGPL-3.0-or-later

pub use self::metadata::{Metadata, MprisMetadata};
pub use self::player::MprisPlayerProxy;
pub use self::root::MprisRootProxy;
pub use self::types::{LoopStatus, PlaybackStatus};

pub mod metadata;
pub mod player;
pub mod root;
pub mod types;

pub const MPRIS_PREFIX: &str = "org.mpris.MediaPlayer2.";
pub const MPRIS_PATH: &str = "/org/mpris/MediaPlayer2";
