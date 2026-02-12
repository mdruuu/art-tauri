<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  let hotkey = $state("");
  let recording = $state(false);
  let saved = $state(false);
  let error = $state("");

  import { onMount } from "svelte";

  onMount(() => {
    invoke<string>("get_hotkey").then((hk) => {
      hotkey = hk;
    });
  });

  function startRecording() {
    recording = true;
    error = "";
    saved = false;

    function onKeyDown(e: KeyboardEvent) {
      e.preventDefault();

      // Ignore bare modifier keys
      if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) return;

      const parts: string[] = [];
      if (e.metaKey || e.ctrlKey) parts.push("CmdOrCtrl");
      if (e.shiftKey) parts.push("Shift");
      if (e.altKey) parts.push("Alt");

      // Map key to Tauri format
      let key = e.key;
      if (key === " ") key = "Space";
      else if (key.length === 1) key = key.toUpperCase();

      parts.push(key);
      hotkey = parts.join("+");
      recording = false;

      window.removeEventListener("keydown", onKeyDown);
    }

    window.addEventListener("keydown", onKeyDown);
  }

  async function saveHotkey() {
    try {
      await invoke("set_hotkey", { hotkey });
      saved = true;
      error = "";
      setTimeout(() => (saved = false), 2000);
    } catch (e) {
      error = String(e);
    }
  }
</script>

<div class="settings">
  <h2>Art Settings</h2>

  <div class="field">
    <!-- svelte-ignore a11y_label_has_associated_control -->
    <label>Global Hotkey</label>
    <div class="hotkey-row">
      <button class="hotkey-display" onclick={startRecording}>
        {#if recording}
          <span class="recording">Press a key combo...</span>
        {:else}
          {hotkey || "Not set"}
        {/if}
      </button>
      <button class="save-btn" onclick={saveHotkey} disabled={recording || !hotkey}>
        {saved ? "Saved!" : "Save"}
      </button>
    </div>
    {#if error}
      <p class="error">{error}</p>
    {/if}
    <p class="help">Click the box, then press your desired key combination.</p>
  </div>

  <div class="info">
    <p>Press the hotkey to show random artwork fullscreen on all monitors.</p>
    <p>Use arrow keys to browse, Escape to dismiss.</p>
  </div>
</div>

<style>
  .settings {
    padding: 24px;
    background: var(--bg);
    min-height: 100vh;
  }

  h2 {
    font-size: 1.2rem;
    font-weight: 600;
    margin-bottom: 20px;
    color: var(--text);
  }

  .field {
    margin-bottom: 20px;
  }

  label {
    display: block;
    font-size: 0.85rem;
    color: var(--text-muted);
    margin-bottom: 8px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .hotkey-row {
    display: flex;
    gap: 8px;
  }

  .hotkey-display {
    flex: 1;
    padding: 10px 14px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text);
    font-family: monospace;
    font-size: 0.95rem;
    cursor: pointer;
    text-align: left;
  }

  .hotkey-display:hover {
    border-color: var(--accent);
  }

  .recording {
    color: var(--accent);
    animation: pulse 1s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }

  .save-btn {
    padding: 10px 18px;
    background: var(--accent);
    color: #fff;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.9rem;
  }

  .save-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .save-btn:hover:not(:disabled) {
    filter: brightness(1.1);
  }

  .error {
    margin-top: 6px;
    color: #e55;
    font-size: 0.8rem;
  }

  .help {
    margin-top: 6px;
    font-size: 0.8rem;
    color: var(--text-muted);
  }

  .info {
    margin-top: 24px;
    padding: 14px;
    background: var(--surface);
    border-radius: 6px;
    font-size: 0.85rem;
    color: var(--text-muted);
    line-height: 1.5;
  }

  .info p + p {
    margin-top: 4px;
  }
</style>
