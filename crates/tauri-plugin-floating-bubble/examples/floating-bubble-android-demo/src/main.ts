import {
  showBubble,
  hideBubble,
  isBubbleVisible,
  hasOverlayPermission,
  requestOverlayPermission,
  setBubbleState,
  onBubbleClick,
  type BubbleClickEvent,
} from "tauri-plugin-floating-bubble";

// DOM Elements
let permissionStatusEl: HTMLElement | null;
let bubbleStatusEl: HTMLElement | null;
let logEl: HTMLElement | null;

// Log helper
function log(message: string) {
  const time = new Date().toLocaleTimeString();
  const entry = document.createElement("p");
  entry.className = "log-entry";
  entry.textContent = `[${time}] ${message}`;
  logEl?.appendChild(entry);
  logEl?.scrollTo(0, logEl.scrollHeight);
}

// Update status displays
async function updateStatus() {
  try {
    const { granted } = await hasOverlayPermission();
    if (permissionStatusEl) {
      permissionStatusEl.textContent = granted ? "Granted" : "Not Granted";
      permissionStatusEl.className = `status-value ${granted ? "status-granted" : "status-denied"}`;
    }
  } catch (e) {
    if (permissionStatusEl) {
      permissionStatusEl.textContent = "N/A (Desktop)";
      permissionStatusEl.className = "status-value";
    }
  }

  try {
    const { visible } = await isBubbleVisible();
    if (bubbleStatusEl) {
      bubbleStatusEl.textContent = visible ? "Yes" : "No";
      bubbleStatusEl.className = `status-value ${visible ? "status-granted" : ""}`;
    }
  } catch (e) {
    if (bubbleStatusEl) {
      bubbleStatusEl.textContent = "N/A";
    }
  }
}

// Event handlers
async function handleRequestPermission() {
  log("Requesting overlay permission...");
  try {
    const { granted } = await requestOverlayPermission();
    log(granted ? "Permission granted!" : "Permission denied or pending");
    await updateStatus();
  } catch (e) {
    log(`Error: ${e}`);
  }
}

async function handleShowBubble() {
  log("Showing bubble...");
  try {
    await showBubble({
      size: 60,
      startX: 0,
      startY: 200,
      background: "#1C1C1C",
      states: {
        idle: {},
        recording: {},
        processing: {},
      },
    });
    log("Bubble shown!");
    await updateStatus();
  } catch (e) {
    log(`Error: ${e}`);
  }
}

async function handleHideBubble() {
  log("Hiding bubble...");
  try {
    await hideBubble();
    log("Bubble hidden!");
    await updateStatus();
  } catch (e) {
    log(`Error: ${e}`);
  }
}

async function handleSetState(state: string) {
  log(`Setting state to: ${state}`);
  try {
    await setBubbleState(state);
    log(`State set to: ${state}`);
  } catch (e) {
    log(`Error: ${e}`);
  }
}

// Initialize
window.addEventListener("DOMContentLoaded", async () => {
  // Get DOM elements
  permissionStatusEl = document.querySelector("#permission-status");
  bubbleStatusEl = document.querySelector("#bubble-status");
  logEl = document.querySelector("#log");

  // Set up button handlers
  document.querySelector("#btn-permission")?.addEventListener("click", handleRequestPermission);
  document.querySelector("#btn-show")?.addEventListener("click", handleShowBubble);
  document.querySelector("#btn-hide")?.addEventListener("click", handleHideBubble);

  // State buttons
  document.querySelector("#btn-state-idle")?.addEventListener("click", () => handleSetState("idle"));
  document.querySelector("#btn-state-recording")?.addEventListener("click", () => handleSetState("recording"));
  document.querySelector("#btn-state-processing")?.addEventListener("click", () => handleSetState("processing"));

  // Listen for bubble click events
  try {
    await onBubbleClick((event: BubbleClickEvent) => {
      log(`Bubble clicked! Action: ${event.action}`);
    });
    log("Bubble click listener registered");
  } catch (e) {
    log(`Note: Click listener setup - ${e}`);
  }

  // Initial status update
  log("App initialized");
  await updateStatus();
});
