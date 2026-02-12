<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import type { Artwork } from "../types";

  let displayedArtwork: Artwork | null = $state(null);
  let nextArtwork: Artwork | null = $state(null);
  let showInfo = $state(true);
  let infoTimer: ReturnType<typeof setTimeout> | null = null;
  let loading = $state(true);
  let shown = false;

  // Derived: show the info bar based on whichever artwork is current
  let artwork = $derived(displayedArtwork);

  function onImageLoaded() {
    displayedArtwork = nextArtwork;
    nextArtwork = null;
    loading = false;

    // Signal the backend to show overlay windows once the first image is ready
    if (!shown) {
      shown = true;
      invoke("overlay_ready");
    }
  }

  function resetInfoTimer() {
    showInfo = true;
    if (infoTimer) clearTimeout(infoTimer);
    infoTimer = setTimeout(() => {
      showInfo = false;
    }, 4000);
  }

  $effect(() => {
    const unlisten = listen<Artwork>("artwork-changed", (event) => {
      nextArtwork = event.payload;
      resetInfoTimer();
    });

    // Try to get current artwork on mount, with retry logic
    async function loadArtwork(retries = 3) {
      for (let attempt = 0; attempt < retries; attempt++) {
        try {
          const art = await invoke<Artwork | null>("get_current_artwork");
          if (art) {
            nextArtwork = art;
            resetInfoTimer();
            return;
          }
        } catch (e) {
          console.warn(`get_current_artwork attempt ${attempt + 1} failed:`, e);
        }
        if (attempt < retries - 1) {
          await new Promise((r) => setTimeout(r, 200));
        }
      }
    }
    loadArtwork();

    function onKeyDown(e: KeyboardEvent) {
      switch (e.key) {
        case "Escape":
          invoke("dismiss_overlays");
          break;
        case "ArrowRight":
        case " ":
          loading = true;
          invoke("next_artwork");
          break;
        case "ArrowLeft":
          loading = true;
          invoke("prev_artwork");
          break;
      }
    }

    function onMouseMove() {
      resetInfoTimer();
    }

    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("mousemove", onMouseMove);

    // Safety timeout: if image hasn't loaded within 3s, show windows anyway
    const safetyTimeout = setTimeout(() => {
      if (!shown) {
        shown = true;
        invoke("overlay_ready");
      }
    }, 3000);

    return () => {
      unlisten.then((fn) => fn());
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("mousemove", onMouseMove);
      if (infoTimer) clearTimeout(infoTimer);
      clearTimeout(safetyTimeout);
    };
  });
</script>

<div class="overlay">
  {#if displayedArtwork}
    <img src={displayedArtwork.image_base64} alt={displayedArtwork.title} class="artwork-image" />
  {/if}

  {#if nextArtwork && nextArtwork !== displayedArtwork}
    <img
      src={nextArtwork.image_base64}
      alt={nextArtwork.title}
      class="artwork-image next"
      onload={onImageLoaded}
    />
  {/if}

  {#if artwork}
    <div class="info-bar" class:visible={showInfo}>
      <div class="info-content">
        <h1>{artwork.title}</h1>
        <p class="artist">{artwork.artist}{artwork.date ? `, ${artwork.date}` : ""}</p>
        <p class="source">{artwork.source}</p>
      </div>
      <div class="controls">
        <span class="hint">← → navigate &nbsp; Esc close</span>
      </div>
    </div>
  {/if}

  {#if loading}
    <div class="loading">
      <div class="spinner"></div>
    </div>
  {/if}
</div>

<style>
  .overlay {
    width: 100vw;
    height: 100vh;
    background: #000;
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
    cursor: none;
  }

  .overlay:hover {
    cursor: default;
  }

  .artwork-image {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: contain;
  }

  .artwork-image.next {
    opacity: 0;
    transition: opacity 0.3s ease;
  }

  .info-bar {
    position: fixed;
    bottom: 0;
    left: 0;
    right: 0;
    background: linear-gradient(transparent, rgba(0, 0, 0, 0.85));
    padding: 40px 32px 24px;
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    opacity: 0;
    transition: opacity 0.3s ease;
    pointer-events: none;
  }

  .info-bar.visible {
    opacity: 1;
  }

  .info-content h1 {
    font-size: 1.3rem;
    font-weight: 500;
    color: #fff;
    margin-bottom: 4px;
  }

  .info-content .artist {
    font-size: 0.95rem;
    color: #ccc;
  }

  .info-content .source {
    font-size: 0.8rem;
    color: #888;
    margin-top: 2px;
  }

  .controls .hint {
    font-size: 0.75rem;
    color: #666;
  }

  .loading {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.5);
  }

  .spinner {
    width: 40px;
    height: 40px;
    border: 3px solid rgba(255, 255, 255, 0.15);
    border-top-color: #fff;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
