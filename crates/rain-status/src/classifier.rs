use rain_media::session::MediaSession;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MediaKind {
    Music,
    Video,
}

impl MediaKind {
    pub const fn discord_app_id(self) -> u64 {
        match self {
            Self::Music => 1538463485157974026,
            Self::Video => 1538492070769987634,
        }
    }

    pub fn classify_session(session: &MediaSession) -> Self {
        const MUSIC_DOMAINS: &[&str] = &[
            "music.youtube.com",
            "soundcloud.com",
            "spotify.com",
            "music.apple.com",
            "tidal.com",
            "deezer.com",
            "bandcamp.com",
            "pandora.com",
            "qobuz.com",
        ];

        const MUSIC_PLAYERS: &[&str] = &[
            "spotify",
            "cider",
            "rhythmbox",
            "feishin",
            "amberol",
            "cmus",
            "foobar2000",
            "audacious",
            "strawberry",
            "clementine",
            "mpd",
            "lollypop",
        ];

        const VIDEO_DOMAINS: &[&str] = &[
            "twitch.tv",
            "netflix.com",
            "bilibili.com",
            "crunchyroll.com",
            "vimeo.com",
            "dailymotion.com",
            "disneyplus.com",
            "primevideo.com",
        ];

        const VIDEO_PLAYERS: &[&str] = &[
            "mpv",
            "vlc",
            "celluloid",
            "totem",
            "freetube",
            "clapper",
            "haruna",
        ];

        let url = session
            .track
            .source_url
            .as_deref()
            .unwrap_or("")
            .to_ascii_lowercase();
        let source = session.source_name.to_ascii_lowercase();
        let id = session.id.to_ascii_lowercase();

        // 1. Explicit Music matches
        if MUSIC_DOMAINS
            .iter()
            .any(|d| url.contains(d) || source.contains(d))
            || MUSIC_PLAYERS
                .iter()
                .any(|p| source.contains(p) || id.contains(p))
        {
            return Self::Music;
        }

        // 2. Explicit Video matches
        if VIDEO_DOMAINS
            .iter()
            .any(|d| url.contains(d) || source.contains(d))
            || VIDEO_PLAYERS
                .iter()
                .any(|p| source.contains(p) || id.contains(p))
        {
            return Self::Video;
        }

        // 3. YouTube handling: music.youtube.com vs standard youtube.com
        if url.contains("music.youtube.com") {
            return Self::Music;
        }
        if url.contains("youtube.com") || url.contains("youtu.be") {
            return Self::Video;
        }

        // 4. Metadata heuristics (e.g. Chromium MPRIS with omitted URL)
        if session
            .track
            .album
            .as_ref()
            .is_some_and(|a| !a.trim().is_empty())
        {
            return Self::Music;
        }

        Self::Video
    }
}

#[derive(Deserialize)]
struct ItunesResponse {
    results: Vec<ItunesResult>,
}

#[derive(Deserialize)]
struct ItunesResult {
    #[serde(rename = "artworkUrl100")]
    artwork_url_100: Option<String>,
}

#[derive(Deserialize)]
struct DeezerResponse {
    data: Vec<DeezerTrack>,
}

#[derive(Deserialize)]
struct DeezerTrack {
    album: Option<DeezerAlbum>,
}

#[derive(Deserialize)]
struct DeezerAlbum {
    cover_xl: Option<String>,
    cover_big: Option<String>,
}

#[derive(Clone)]
pub struct ArtworkResolver {
    client: reqwest::Client,
    cache: Arc<RwLock<HashMap<String, Option<String>>>>,
}

impl Default for ArtworkResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ArtworkResolver {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(4))
                .user_agent("rain-status/1.0 (Discord Rich Presence Daemon)")
                .build()
                .unwrap_or_default(),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn resolve(&self, session: &MediaSession) -> Option<String> {
        let artwork_url = session.track.artwork_url.as_deref();
        let source_url = session.track.source_url.as_deref();

        // 1. Direct public HTTPS artwork URL (from Extension or remote MPRIS)
        if let Some(art) = artwork_url
            && (art.starts_with("https://") || art.starts_with("http://"))
        {
            return Some(art.to_string());
        }

        // 2. Direct YouTube video thumbnail (if source URL has video ID)
        if let Some(url) = source_url
            && let Some(video_id) = extract_youtube_id(url)
        {
            return Some(format!("https://i.ytimg.com/vi/{video_id}/hqdefault.jpg"));
        }

        // 3. Fallback online lookup using Artist + Title
        let title = session.track.title.trim();
        let artist = session.track.display_artist().unwrap_or_default();

        if title.is_empty() {
            return None;
        }

        let cache_key = format!("{artist}:{title}");

        // Check in-memory cache
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(&cache_key) {
                return cached.clone();
            }
        }

        // Fetch online (iTunes -> Deezer)
        let fetched = self.fetch_online_artwork(&artist, title).await;

        // Negative caching: caches None as well so failing lookups don't spam APIs
        let mut cache = self.cache.write().await;
        cache.insert(cache_key, fetched.clone());
        fetched
    }

    async fn fetch_online_artwork(&self, artist: &str, title: &str) -> Option<String> {
        let query = format!("{artist} {title}").trim().to_string();
        if query.is_empty() {
            return None;
        }

        // 1. Try iTunes Search API (fast, high-res 512x512 cover art, no auth required)
        let itunes_url = format!(
            "https://itunes.apple.com/search?term={}&entity=song&limit=1",
            urlencoding::encode(&query)
        );

        if let Ok(res) = self.client.get(&itunes_url).send().await
            && let Ok(data) = res.json::<ItunesResponse>().await
            && let Some(first) = data.results.into_iter().next()
            && let Some(art) = first.artwork_url_100
        {
            return Some(art.replace("100x100bb.jpg", "512x512bb.jpg"));
        }

        // 2. Fallback to Deezer Search API
        let deezer_url = format!(
            "https://api.deezer.com/search?q={}",
            urlencoding::encode(&query)
        );

        if let Ok(res) = self.client.get(&deezer_url).send().await
            && let Ok(data) = res.json::<DeezerResponse>().await
            && let Some(first) = data.data.into_iter().next()
            && let Some(album) = first.album
        {
            return album.cover_xl.or(album.cover_big);
        }

        None
    }
}

fn extract_youtube_id(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    if parsed.host_str()?.contains("youtube.com") {
        parsed
            .query_pairs()
            .find(|(k, _)| k == "v")
            .map(|(_, v)| v.into_owned())
    } else if parsed.host_str()? == "youtu.be" {
        parsed.path_segments()?.next().map(|s| s.to_string())
    } else {
        None
    }
}
