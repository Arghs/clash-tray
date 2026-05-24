<script lang="ts">
  import { onMount } from "svelte";
  import { polyfillCountryFlagEmojis } from "country-flag-emoji-polyfill";
  import Popup from "$lib/views/Popup.svelte";
  import Settings from "$lib/views/Settings.svelte";

  let label: string | undefined = $state(undefined);

  onMount(async () => {
    // Windows' Segoe UI Emoji renders regional-indicator pairs as letters, not flags.
    // The polyfill injects a Twemoji web font that the font-family stacks below use.
    polyfillCountryFlagEmojis();
    try {
      const mod = await import("@tauri-apps/api/webviewWindow");
      label = mod.getCurrentWebviewWindow().label;
    } catch {
      label = "popup";
    }
  });

  // The popup needs html/body locked at 100% with no scroll (its content scrolls inside
  // .popup). The settings window must scroll normally. Apply the lock conditionally.
  $effect(() => {
    if (typeof document === "undefined") return;
    const html = document.documentElement;
    const body = document.body;
    if (label === "popup") {
      html.style.height = "100%";
      body.style.height = "100%";
      html.style.overflow = "hidden";
      body.style.overflow = "hidden";
    } else {
      html.style.height = "";
      body.style.height = "";
      html.style.overflow = "";
      body.style.overflow = "";
    }
  });
</script>

{#if label === "popup"}
  <Popup />
{:else if label === "settings"}
  <Settings />
{:else}
  <main class="loading">Loading…</main>
{/if}

<style>
  :global(html, body) {
    margin: 0;
    padding: 0;
    background: transparent;
    overscroll-behavior: none;
    /* Twemoji Country Flags is loaded by the polyfill in onMount; listing it first
       in the stack makes the browser use it for flag emoji while still falling
       through to the system font for the rest of the text. */
    font-family: "Twemoji Country Flags", system-ui, "Segoe UI", sans-serif;
  }
  .loading {
    padding: 1rem;
    font-family: system-ui, "Segoe UI", sans-serif;
    color: #888;
  }
</style>
