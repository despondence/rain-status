const DAEMON_URL = "http://127.0.0.1:6767/update-status";

console.log("[Rain BG Worker] Service worker initialized.");

chrome.runtime.onMessage.addListener((message, _sender, _sendResponse) => {
  if (message.type === "STATUS_UPDATE" && message.payload) {
    send(message.payload);
  }
});

async function send(payload) {
  try {
    const res = await fetch(DAEMON_URL, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });

    if (!res.ok) {
      console.warn(`[Rain BG Worker] Daemon HTTP error: ${res.status}`);
    }
  } catch (err) {
    console.error("[Rain BG Worker] Failed to reach Rust daemon:", err);
  }
}
