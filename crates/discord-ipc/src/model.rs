// Copyright (C) 2026 rain (despondence) <308736589+despondence@users.noreply.github.com>
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use serde_with::{serde_as, skip_serializing_none, DisplayFromStr};
use uuid::Uuid;

#[serde_as]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Handshake {
    pub v: u8,
    #[serde_as(as = "DisplayFromStr")]
    pub client_id: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Cmd {
    SetActivity,
}

#[derive(Clone, Copy, Debug, Default, Deserialize_repr, Eq, PartialEq, Serialize_repr)]
#[repr(u8)]
pub enum ActivityType {
    #[default]
    Game = 0,
    Streaming = 1,
    Listening = 2,
    Watching = 3,
    Custom = 4,
    Competing = 5,
}

#[skip_serializing_none]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ActivityTimestamps {
    pub start: Option<u64>,
    pub end: Option<u64>,
}

#[skip_serializing_none]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ActivityAssets {
    pub large_image: Option<String>,
    pub large_text: Option<String>,
    pub small_image: Option<String>,
    pub small_text: Option<String>,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Activity {
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub application_id: Option<u64>,
    pub platform: Option<String>,
    pub details: Option<String>,
    pub state: Option<String>,
    #[serde(rename = "type")]
    pub activity_type: ActivityType,
    pub status_display_type: Option<u8>,
    pub timestamps: Option<ActivityTimestamps>,
    pub assets: Option<ActivityAssets>,
}

#[skip_serializing_none]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Args {
    pub pid: u32,
    pub activity: Option<Activity>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SetActivity {
    pub cmd: Cmd,
    pub args: Args,
    pub nonce: Uuid,
}
