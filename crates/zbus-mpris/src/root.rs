// Copyright (C) 2026 rain (despondence) <308736589+despondence@users.noreply.github.com>
// SPDX-License-Identifier: AGPL-3.0-or-later

#[zbus::proxy(
    interface = "org.mpris.MediaPlayer2",
    default_path = "/org/mpris/MediaPlayer2"
)]
pub trait MprisRoot {
    async fn raise(&self) -> zbus::Result<()>;
    async fn quit(&self) -> zbus::Result<()>;

    #[zbus(property)]
    fn can_quit(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn fullscreen(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn set_fullscreen(&self, fullscreen: bool) -> zbus::Result<()>;

    #[zbus(property)]
    fn can_set_fullscreen(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn can_raise(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn has_track_list(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn identity(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn desktop_entry(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn supported_uri_schemes(&self) -> zbus::Result<Vec<String>>;

    #[zbus(property)]
    fn supported_mime_types(&self) -> zbus::Result<Vec<String>>;
}
