<script lang="ts">
  import { onMount } from "svelte";
  import {
    isEnabled as isAutostartEnabled,
    enable as enableAutostart,
    disable as disableAutostart,
  } from "@tauri-apps/plugin-autostart";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api } from "../api";
  import { iso2ToDisplay, iso2ToFlag } from "../country";
  import type { Settings, StateSnapshot } from "../types";

  const FALLBACK_FAVORITES = ["HK", "JP", "SG", "TW", "US", "GB", "DE", "KR"];
  const ALL_ISO2 = [
    "HK", "JP", "SG", "TW", "US", "GB", "DE", "KR", "FR", "MY",
    "RU", "CA", "AU", "NL", "TR", "IN", "BR", "AR", "TH", "PH",
    "VN", "ID", "AE", "IL", "ZA", "IT", "ES", "CH", "SE", "NO",
    "FI", "DK", "PL", "UA", "MX",
  ];

  let settings = $state<Settings | null>(null);
  let secretInput: string = $state("");
  let snapshot = $state<StateSnapshot | null>(null);

  // Test connection ui state
  let testResult: string = $state("");
  let testOk: boolean | null = $state(null);
  let testing: boolean = $state(false);

  let saveResult: string = $state("");
  let saving: boolean = $state(false);

  // Override draft row
  let newOverrideNode: string = $state("");
  let newOverrideCode: string = $state("HK");

  let initialStartOnLogin: boolean = false;

  let selectorGroups = $derived(
    snapshot?.groups.filter((g) => g.kind === "Selector") ?? [],
  );

  let allNodeNames = $derived.by((): string[] => {
    if (!snapshot) return [];
    const set = new Set<string>();
    for (const g of snapshot.groups) {
      for (const m of g.members) {
        if (!m.is_group && !m.name.includes("：")) set.add(m.name);
      }
    }
    return Array.from(set).sort();
  });

  let observedCountries = $derived.by((): string[] => {
    if (!snapshot) return [];
    const set = new Set<string>();
    for (const g of snapshot.groups) {
      for (const m of g.members) {
        if (m.country && m.country !== "XX") set.add(m.country);
      }
    }
    return Array.from(set);
  });

  let favoriteOptions = $derived.by((): string[] => {
    const set = new Set<string>([...FALLBACK_FAVORITES, ...observedCountries]);
    if (settings) for (const c of settings.favorites) set.add(c);
    return Array.from(set).sort();
  });

  onMount(async () => {
    const [s, snap] = await Promise.all([
      api.getSettings(),
      api.getState().catch(() => null),
    ]);
    settings = s;
    secretInput = s.secret ?? "";
    snapshot = snap;
    // Sync the stored setting with the actual autostart state — they can drift if
    // the user toggled it from outside the app.
    try {
      const actual = await isAutostartEnabled();
      if (actual !== s.start_on_login && settings) {
        settings.start_on_login = actual;
      }
      initialStartOnLogin = settings?.start_on_login ?? false;
    } catch {
      initialStartOnLogin = s.start_on_login;
    }
  });

  async function test() {
    if (!settings) return;
    testing = true;
    testResult = "Testing…";
    testOk = null;
    const t0 = performance.now();
    try {
      const v = await api.testConnection(
        settings.clash_url,
        secretInput.length > 0 ? secretInput : null,
      );
      const ms = Math.round(performance.now() - t0);
      testResult = `OK — v${v.version}${v.meta ? " (meta)" : ""} · ${ms} ms`;
      testOk = true;
    } catch (e) {
      testResult = `Error: ${e}`;
      testOk = false;
    } finally {
      testing = false;
    }
  }

  function toggleFavorite(c: string) {
    if (!settings) return;
    const has = settings.favorites.includes(c);
    settings.favorites = has
      ? settings.favorites.filter((x) => x !== c)
      : [...settings.favorites, c];
  }

  function addOverride() {
    if (!settings) return;
    const node = newOverrideNode.trim();
    const code = newOverrideCode.trim().toUpperCase();
    if (!node || code.length !== 2) return;
    settings.country_overrides = { ...settings.country_overrides, [node]: code };
    newOverrideNode = "";
  }

  function removeOverride(node: string) {
    if (!settings) return;
    const next = { ...settings.country_overrides };
    delete next[node];
    settings.country_overrides = next;
  }

  async function save() {
    if (!settings) return;
    saving = true;
    saveResult = "Saving…";
    try {
      const payload: Settings = {
        ...settings,
        secret: secretInput.length > 0 ? secretInput : null,
      };
      await api.saveSettings(payload);
      settings = payload;

      // Apply autostart if changed.
      if (payload.start_on_login !== initialStartOnLogin) {
        try {
          if (payload.start_on_login) await enableAutostart();
          else await disableAutostart();
          initialStartOnLogin = payload.start_on_login;
        } catch (e) {
          saveResult = `Saved settings, but autostart toggle failed: ${e}`;
          saving = false;
          return;
        }
      }

      saveResult = "Saved.";
      setTimeout(() => (saveResult = ""), 2000);
    } catch (e) {
      saveResult = `Error: ${e}`;
    } finally {
      saving = false;
    }
  }

  async function openDashboard() {
    if (!settings) return;
    const url = `${settings.clash_url.replace(/\/$/, "")}/ui`;
    try {
      await openUrl(url);
    } catch {
      // Most OpenClash installs expose the dashboard at the same host on a UI path;
      // fall back to the base URL if /ui isn't routable.
      await openUrl(settings.clash_url);
    }
  }
</script>

<main>
  <h1>Clash Tray — Settings</h1>

  {#if settings}
    <form onsubmit={(e) => { e.preventDefault(); save(); }}>
      <!-- Connection -->
      <section>
        <h2>Connection</h2>
        <label>
          <span>Clash URL</span>
          <input
            type="text"
            bind:value={settings.clash_url}
            placeholder="http://192.168.8.1:9090"
          />
        </label>
        <label>
          <span>Secret</span>
          <input
            type="password"
            bind:value={secretInput}
            placeholder="(leave empty if none)"
          />
        </label>
        <div class="row">
          <button type="button" onclick={test} disabled={testing}>
            {testing ? "Testing…" : "Test connection"}
          </button>
          {#if testResult}
            <span
              class="pill"
              class:ok={testOk === true}
              class:err={testOk === false}>{testResult}</span>
          {/if}
        </div>
      </section>

      <!-- Behavior -->
      <section>
        <h2>Behavior</h2>
        <label class="slider-row">
          <span>Poll interval</span>
          <input
            type="range"
            min="1000"
            max="10000"
            step="500"
            bind:value={settings.poll_interval_ms}
          />
          <span class="val">{(settings.poll_interval_ms / 1000).toFixed(1)} s</span>
        </label>
        <label>
          <span>Primary group</span>
          <select bind:value={settings.primary_group}>
            <option value={null}>(auto — first selector)</option>
            {#each selectorGroups as g (g.name)}
              <option value={g.name}>{g.name}</option>
            {/each}
          </select>
        </label>
      </section>

      <!-- Notifications -->
      <section>
        <h2>Notifications</h2>
        <label class="check">
          <input type="checkbox" bind:checked={settings.notify_auto_switch} />
          <span>Toast when a group auto-switches</span>
        </label>
        <label class="check">
          <input type="checkbox" bind:checked={settings.notify_connection} />
          <span>Toast on connection lost / restored</span>
        </label>
      </section>

      <!-- Autostart -->
      <section>
        <h2>Startup</h2>
        <label class="check">
          <input type="checkbox" bind:checked={settings.start_on_login} />
          <span>Start with Windows (launches minimized to tray)</span>
        </label>
      </section>

      <!-- Favorites -->
      <section>
        <h2>Favorite countries</h2>
        <p class="hint">Shown in the tray's Quick switch submenu.</p>
        <div class="chips">
          {#each favoriteOptions as c (c)}
            {@const active = settings.favorites.includes(c)}
            <button
              type="button"
              class="chip"
              class:active
              onclick={() => toggleFavorite(c)}>
              <span class="flag">{iso2ToFlag(c)}</span>
              <span class="code">{c}</span>
              <span class="name">{iso2ToDisplay(c)}</span>
            </button>
          {/each}
        </div>
      </section>

      <!-- Country overrides -->
      <section>
        <h2>Country overrides</h2>
        <p class="hint">
          Force a specific node to be classified as a country. Useful when the
          parser miscategorizes.
        </p>
        <datalist id="node-names">
          {#each allNodeNames as n}
            <option value={n}></option>
          {/each}
        </datalist>
        <table class="overrides">
          <thead>
            <tr>
              <th>Node name</th>
              <th>Country</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {#each Object.entries(settings.country_overrides) as [node, code] (node)}
              <tr>
                <td class="node">{node}</td>
                <td>
                  <span class="flag">{iso2ToFlag(code)}</span>
                  {code} — {iso2ToDisplay(code)}
                </td>
                <td>
                  <button
                    type="button"
                    class="del"
                    onclick={() => removeOverride(node)}>×</button>
                </td>
              </tr>
            {/each}
            <tr>
              <td>
                <input
                  type="text"
                  list="node-names"
                  bind:value={newOverrideNode}
                  placeholder="Pick or type a node name…" />
              </td>
              <td>
                <select bind:value={newOverrideCode}>
                  {#each ALL_ISO2 as c}
                    <option value={c}>{iso2ToFlag(c)} {c} — {iso2ToDisplay(c)}</option>
                  {/each}
                </select>
              </td>
              <td>
                <button type="button" onclick={addOverride}>Add</button>
              </td>
            </tr>
          </tbody>
        </table>
      </section>

      <!-- Shortcuts -->
      <section>
        <h2>Shortcuts</h2>
        <p class="hint">Global keyboard shortcuts — work even when the popup isn't focused.</p>
        <div class="shortcut">
          <span>Toggle popup</span>
          <span class="keys">
            <kbd>Ctrl</kbd> + <kbd>Alt</kbd> + <kbd>P</kbd>
          </span>
        </div>
        <div class="shortcut">
          <span>Hide popup</span>
          <span class="keys"><kbd>Esc</kbd></span>
        </div>
      </section>

      <!-- About -->
      <section>
        <h2>About</h2>
        <p class="hint">
          Clash Tray — Windows tray monitor for OpenClash / Mihomo.
        </p>
        <div class="row">
          <button type="button" onclick={openDashboard}>
            Open dashboard
          </button>
        </div>
      </section>

      <div class="footer">
        <button type="submit" class="primary" disabled={saving}>
          {saving ? "Saving…" : "Save"}
        </button>
        {#if saveResult}
          <span class="pill">{saveResult}</span>
        {/if}
      </div>
    </form>
  {:else}
    <p>Loading…</p>
  {/if}
</main>

<style>
  main {
    padding: 1.25rem 1.5rem 2rem;
    font-family: "Twemoji Country Flags", system-ui, "Segoe UI", sans-serif;
    background: #fff;
    color: #111;
    min-height: 100vh;
    box-sizing: border-box;
  }
  @media (prefers-color-scheme: dark) {
    main { background: #1e1e22; color: #eee; }
    input, select { background: #2a2a30; color: #eee; border-color: #444; }
    button { background: #2a2a30; color: #eee; border-color: #555; }
    .chip { background: #2a2a30; border-color: #555; color: #eee; }
    .chip.active { background: #2d6cdf; border-color: #4d8be8; color: #fff; }
    .chip .name { color: #aaa; }
    .chip.active .name { color: #fff; opacity: 0.85; }
    section { border-color: #333; }
    table { border-color: #333; }
    th, td { border-color: #333; }
    .pill { background: #2a2a30; color: #ddd; }
    .hint { color: #aaa; }
  }
  h1 {
    margin: 0 0 0.6rem;
    font-size: 1.25rem;
  }
  h2 {
    margin: 0 0 0.55rem;
    font-size: 0.95rem;
    font-weight: 600;
    color: #6c6c70;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  @media (prefers-color-scheme: dark) {
    h2 { color: #aaa; }
  }
  form {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  section {
    border: 1px solid #e0e0e0;
    border-radius: 8px;
    padding: 0.9rem 1rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
  }
  .hint {
    color: #888;
    font-size: 0.78rem;
    margin: 0 0 0.2rem;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    font-size: 0.85rem;
  }
  label > span:first-child {
    font-weight: 600;
  }
  label.check {
    flex-direction: row;
    align-items: center;
    gap: 0.5rem;
    font-weight: 400;
  }
  label.check > span { font-weight: 400; }
  label.slider-row {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 0.6rem;
  }
  .val {
    font-variant-numeric: tabular-nums;
    font-size: 0.8rem;
    color: #666;
    min-width: 3em;
    text-align: right;
  }
  input, select {
    padding: 0.4rem 0.6rem;
    border: 1px solid #ccc;
    border-radius: 6px;
    font: inherit;
    color: #111;
    background: #fff;
  }
  input[type="checkbox"] { padding: 0; }
  input[type="range"] {
    padding: 0;
    width: 100%;
  }
  /* Hardcoded foreground/background — `inherit` can resolve to a WebView2 system
     color (e.g. white text under Windows dark theme) and disappear on our light bg. */
  button {
    background: #f5f5f5;
    border: 1px solid #bbb;
    border-radius: 6px;
    padding: 0.4rem 0.8rem;
    cursor: pointer;
    font: inherit;
    color: #111;
  }
  button:hover:not(:disabled) { filter: brightness(1.05); }
  button:disabled { opacity: 0.6; cursor: not-allowed; }
  button.primary {
    background: #2d6cdf;
    color: white;
    border-color: #2d6cdf;
  }
  .row {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    flex-wrap: wrap;
  }
  .pill {
    padding: 0.25rem 0.55rem;
    border-radius: 999px;
    font-size: 0.78rem;
    background: #f1f1f1;
    color: #444;
  }
  .pill.ok {
    background: rgba(46, 204, 113, 0.18);
    color: #1e8449;
  }
  .pill.err {
    background: rgba(231, 76, 60, 0.18);
    color: #c0392b;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    padding: 0.25rem 0.6rem;
    border-radius: 999px;
    background: #fff;
    border: 1px solid #bbb;
    color: #222;
    font-size: 0.78rem;
    cursor: pointer;
  }
  .chip.active {
    background: #2d6cdf;
    border-color: #2466c8;
    color: #fff;
  }
  .chip .flag { font-size: 0.95rem; }
  .chip .code { font-weight: 700; font-variant-numeric: tabular-nums; }
  .chip .name { color: #666; }
  .chip.active .name { color: #fff; opacity: 0.85; }

  table.overrides {
    border-collapse: collapse;
    font-size: 0.83rem;
    width: 100%;
  }
  table.overrides th {
    text-align: left;
    font-weight: 600;
    font-size: 0.72rem;
    text-transform: uppercase;
    color: #888;
    padding: 0.3rem 0.4rem;
    border-bottom: 1px solid #e0e0e0;
  }
  table.overrides td {
    padding: 0.3rem 0.4rem;
    border-bottom: 1px solid #efefef;
    vertical-align: middle;
  }
  table.overrides td.node {
    font-family: ui-monospace, SFMono-Regular, "Cascadia Code", monospace;
    font-size: 0.78rem;
    word-break: break-all;
  }
  table.overrides input,
  table.overrides select {
    padding: 0.25rem 0.4rem;
    font-size: 0.82rem;
    width: 100%;
    box-sizing: border-box;
  }
  .del {
    background: transparent;
    border: none;
    color: #c0392b;
    cursor: pointer;
    font-size: 1.05rem;
    padding: 0 0.3rem;
  }
  .footer {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding-top: 0.3rem;
  }
  .shortcut {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.85rem;
    padding: 0.25rem 0;
  }
  .keys {
    display: inline-flex;
    align-items: center;
    gap: 0.2rem;
  }
  kbd {
    display: inline-block;
    min-width: 1.4em;
    padding: 0.1em 0.45em;
    border: 1px solid #bbb;
    border-bottom-width: 2px;
    border-radius: 4px;
    background: #f7f7f7;
    color: #222;
    font-family: ui-monospace, SFMono-Regular, "Cascadia Code", monospace;
    font-size: 0.78em;
    text-align: center;
    line-height: 1.4;
  }
  @media (prefers-color-scheme: dark) {
    kbd {
      background: #2a2a30;
      border-color: #555;
      color: #eee;
    }
  }
</style>
