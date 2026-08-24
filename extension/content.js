/**
 * Simplified Media Metadata Extractor
 */

// Lightweight site-specific overrides for missing MediaSession info
const SITE_ADAPTERS = [
  {
    match: () => location.hostname.includes("youtube.com"),
    getAuthor: () =>
      document
        .querySelector(".ytmusic-player-bar .byline")
        ?.textContent?.trim() ||
      document
        .querySelector("#owner #channel-name a, #upload-info #channel-name")
        ?.textContent?.trim() ||
      "",
    getArtwork: () => {
      const videoId = new URLSearchParams(location.search).get("v");
      return videoId ? `https://i.ytimg.com/vi/${videoId}/hqdefault.jpg` : "";
    },
  },
  {
    match: () => location.hostname.includes("soundcloud.com"),
    getAuthor: () =>
      document
        .querySelector(".playbackSoundBadge__titleContextContainer a")
        ?.textContent?.trim() ||
      document
        .querySelector(".playbackSoundBadge__lightLink")
        ?.textContent?.trim() ||
      "",
    getArtwork: () => {
      const el = document.querySelector(
        ".playbackSoundBadge__avatar span.sc-artwork",
      );
      const bg = el?.style?.backgroundImage;
      if (bg) {
        const match = bg.match(/url\(["']?([^"']+)["']?\)/);
        if (match) return match[1].replace("-t50x50.", "-t500x500.");
      }
      return "";
    },
  },
  {
    match: () => location.hostname.includes("spotify.com"),
    getAuthor: () =>
      Array.from(
        document.querySelectorAll("[data-testid='context-item-info-artist']"),
      )
        .map((el) => el.textContent)
        .join(", ") || "",
    getArtwork: () =>
      document.querySelector("[data-testid='cover-art-image']")?.src || "",
  },
  {
    match: () => location.hostname.includes("bandcamp.com"),
    getAuthor: () =>
      document.querySelector("#name-section h3 span a")?.textContent?.trim() ||
      "",
    getArtwork: () => document.querySelector("#tralbumArt img")?.src || "",
  },
];

class SimpleMediaTracker {
  constructor() {
    this.lastPayloadJson = "";
    this.adapter = SITE_ADAPTERS.find((a) => a.match()) || null;
    this.init();
  }

  init() {
    // Check state periodically or on navigation events
    setInterval(() => this.checkAndSend(), 2000);

    if (window.navigation) {
      window.navigation.addEventListener("navigate", () => {
        setTimeout(() => this.checkAndSend(), 500);
      });
    }
  }

  getArtwork(metadata) {
    if (metadata?.artwork?.length) {
      const sorted = [...metadata.artwork].sort((a, b) => {
        const dimA = parseInt(a.sizes?.split("x")[0] || "0", 10);
        const dimB = parseInt(b.sizes?.split("x")[0] || "0", 10);
        return dimB - dimA;
      });
      if (sorted[0]?.src) return sorted[0].src;
    }

    if (this.adapter?.getArtwork) {
      const adapterArt = this.adapter.getArtwork();
      if (adapterArt) return adapterArt;
    }

    return (
      document.querySelector("meta[property='og:image']")?.content ||
      document.querySelector("link[rel='image_src']")?.href ||
      ""
    );
  }

  getAuthor(metadata) {
    if (metadata?.artist) return metadata.artist;

    if (this.adapter?.getAuthor) {
      const adapterAuthor = this.adapter.getAuthor();
      if (adapterAuthor) return adapterAuthor;
    }

    return "";
  }

  getTitle(metadata) {
    if (metadata?.title) return metadata.title;

    return (
      document.title
        .replace(/^[\(\[\u25b6\u25b8\u25fa\u220f\s\d:\.]+(?=\w)/, "")
        .replace(
          /\s*[\-\|]\s*(YouTube|YouTube Music|SoundCloud|Twitch|Bandcamp|Spotify|Apple Music|Tidal)$/i,
          "",
        )
        .trim() || ""
    );
  }

  checkAndSend() {
    const metadata = navigator.mediaSession?.metadata;
    const mediaList = Array.from(document.querySelectorAll("video, audio"));
    const activeMedia = mediaList.find(
      (el) => !el.paused && !el.ended && el.readyState > 0,
    );

    // If there is no active playback or valid title, don't update
    const title = this.getTitle(metadata);
    if (!activeMedia && !metadata?.title) return;

    const payload = {
      title: title,
      author: this.getAuthor(metadata),
      artwork: this.getArtwork(metadata),
      url: location.href,
    };

    const currentJson = JSON.stringify(payload);
    if (currentJson === this.lastPayloadJson) return;

    this.lastPayloadJson = currentJson;

    if (chrome?.runtime?.id) {
      chrome.runtime.sendMessage({ type: "STATUS_UPDATE", payload });
    }
  }
}

new SimpleMediaTracker();
