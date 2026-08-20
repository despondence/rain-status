// Copyright (C) 2026 rain (despondence) <308736589+despondence@users.noreply.github.com>
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;
use zbus::zvariant::{ObjectPath, Value};

#[zbus::proxy(
    interface = "org.mpris.MediaPlayer2.Player",
    default_path = "/org/mpris/MediaPlayer2"
)]
pub trait MprisPlayer {
    async fn next(&self) -> zbus::Result<()>;
    async fn previous(&self) -> zbus::Result<()>;
    async fn pause(&self) -> zbus::Result<()>;
    async fn play_pause(&self) -> zbus::Result<()>;
    async fn stop(&self) -> zbus::Result<()>;
    async fn play(&self) -> zbus::Result<()>;
    async fn seek(&self, offset: i64) -> zbus::Result<()>;
    async fn set_position(&self, track_id: ObjectPath<'_>, position: i64) -> zbus::Result<()>;
    async fn open_uri(&self, uri: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    fn seeked(&self, position: i64) -> zbus::Result<()>;

    #[zbus(property)]
    fn playback_status(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn loop_status(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn set_loop_status(&self, status: &str) -> zbus::Result<()>;

    #[zbus(property)]
    fn rate(&self) -> zbus::Result<f64>;

    #[zbus(property)]
    fn set_rate(&self, rate: f64) -> zbus::Result<()>;

    #[zbus(property)]
    fn shuffle(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn set_shuffle(&self, shuffle: bool) -> zbus::Result<()>;

    #[zbus(property)]
    fn metadata(&self) -> zbus::Result<HashMap<String, Value<'static>>>;

    #[zbus(property)]
    fn volume(&self) -> zbus::Result<f64>;

    #[zbus(property)]
    fn set_volume(&self, volume: f64) -> zbus::Result<()>;

    #[zbus(property)]
    fn position(&self) -> zbus::Result<i64>;

    #[zbus(property)]
    fn minimum_rate(&self) -> zbus::Result<f64>;

    #[zbus(property)]
    fn maximum_rate(&self) -> zbus::Result<f64>;

    #[zbus(property)]
    fn can_go_next(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn can_go_previous(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn can_play(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn can_pause(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn can_seek(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn can_control(&self) -> zbus::Result<bool>;
}
