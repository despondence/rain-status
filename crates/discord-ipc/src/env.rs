// Copyright (C) 2026 rain (despondence) <308736589+despondence@users.noreply.github.com>
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::env;
use std::path::PathBuf;

pub fn ipc_dir() -> PathBuf {
    xdg::BaseDirectories::new()
        .get_runtime_directory()
        .ok()
        .cloned()
        .unwrap_or_else(env::temp_dir)
}

pub fn ipc_path() -> PathBuf {
    ipc_dir().join("discord-ipc-0")
}
