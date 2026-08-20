// Copyright (C) 2026 rain (despondence) <308736589+despondence@users.noreply.github.com>
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;
use std::time::Duration;
use zbus::zvariant::Value;

pub type Metadata = HashMap<String, Value<'static>>;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MprisMetadata {
    pub track_id: Option<String>,
    pub title: String,
    pub artist: Vec<String>,
    pub album: String,
    pub album_artist: Vec<String>,
    pub art_url: String,
    pub url: String,
    pub length: Option<Duration>,
    pub track_number: Option<i32>,
    pub disc_number: Option<i32>,
    pub genre: Vec<String>,
    pub lyrics: Option<String>,
    pub user_rating: Option<f64>,
}

impl MprisMetadata {
    pub fn from_metadata(metadata: &Metadata) -> Self {
        Self {
            track_id: get_string(metadata, "mpris:trackid"),
            title: get_string(metadata, "xesam:title").unwrap_or_default(),
            artist: get_string_array(metadata, "xesam:artist").unwrap_or_default(),
            album: get_string(metadata, "xesam:album").unwrap_or_default(),
            album_artist: get_string_array(metadata, "xesam:albumArtist").unwrap_or_default(),
            art_url: get_string(metadata, "mpris:artUrl").unwrap_or_default(),
            url: get_string(metadata, "xesam:url").unwrap_or_default(),
            length: get_duration(metadata, "mpris:length"),
            track_number: get_i32(metadata, "xesam:trackNumber"),
            disc_number: get_i32(metadata, "xesam:discNumber"),
            genre: get_string_array(metadata, "xesam:genre").unwrap_or_default(),
            lyrics: get_string(metadata, "xesam:asText")
                .or_else(|| get_string(metadata, "xesam:lyrics")),
            user_rating: get_f64(metadata, "xesam:userRating"),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.title.trim().is_empty() && self.artist.is_empty() && self.url.is_empty()
    }
}

impl From<&Metadata> for MprisMetadata {
    fn from(metadata: &Metadata) -> Self {
        Self::from_metadata(metadata)
    }
}

fn unwrap_value<'a>(value: &'a Value<'a>) -> &'a Value<'a> {
    match value {
        Value::Value(nested) => unwrap_value(nested),
        other => other,
    }
}

fn get_string(metadata: &Metadata, key: &str) -> Option<String> {
    metadata
        .get(key)
        .and_then(|value| match unwrap_value(value) {
            Value::Str(string) => Some(string.to_string()),
            Value::ObjectPath(path) => Some(path.to_string()),
            _ => None,
        })
}

fn get_string_array(metadata: &Metadata, key: &str) -> Option<Vec<String>> {
    metadata
        .get(key)
        .and_then(|value| match unwrap_value(value) {
            Value::Array(array) => {
                let list: Vec<String> = array
                    .iter()
                    .filter_map(|item| match unwrap_value(item) {
                        Value::Str(string) => Some(string.to_string()),
                        Value::ObjectPath(path) => Some(path.to_string()),
                        _ => None,
                    })
                    .collect();

                if list.is_empty() { None } else { Some(list) }
            }
            Value::Str(string) if !string.trim().is_empty() => Some(vec![string.to_string()]),
            _ => None,
        })
}

fn get_duration(metadata: &Metadata, key: &str) -> Option<Duration> {
    metadata
        .get(key)
        .and_then(|value| match unwrap_value(value) {
            Value::I64(micros) => u64::try_from(*micros).ok().map(Duration::from_micros),
            Value::U64(micros) => Some(Duration::from_micros(*micros)),
            Value::I32(micros) => u64::try_from(*micros).ok().map(Duration::from_micros),
            Value::U32(micros) => Some(Duration::from_micros(*micros as u64)),
            Value::F64(micros) if *micros >= 0.0 => Some(Duration::from_micros(*micros as u64)),
            _ => None,
        })
}

fn get_i32(metadata: &Metadata, key: &str) -> Option<i32> {
    metadata
        .get(key)
        .and_then(|value| match unwrap_value(value) {
            Value::I32(value) => Some(*value),
            Value::U32(value) => i32::try_from(*value).ok(),
            Value::I64(value) => i32::try_from(*value).ok(),
            Value::U64(value) => i32::try_from(*value).ok(),
            Value::I16(value) => Some(*value as i32),
            Value::U16(value) => Some(*value as i32),
            _ => None,
        })
}

fn get_f64(metadata: &Metadata, key: &str) -> Option<f64> {
    metadata
        .get(key)
        .and_then(|value| match unwrap_value(value) {
            Value::F64(value) => Some(*value),
            Value::I32(value) => Some(*value as f64),
            Value::I64(value) => Some(*value as f64),
            _ => None,
        })
}
