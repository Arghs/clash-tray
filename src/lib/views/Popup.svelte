<script lang="ts">
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { onDestroy, onMount } from "svelte";
  import { api } from "../api";
  import ConnectionDot from "../components/ConnectionDot.svelte";
  import CurrentProxy from "../components/CurrentProxy.svelte";
  import ProxyGroup from "../components/ProxyGroup.svelte";
  import { formatBytes, formatSpeed } from "../format";
  import type { StateSnapshot } from "../types";

  let snapshot: StateSnapshot | null = $state(null);
  let connectionError: string | null = $state(null);
  let switchError: string | null = $state(null);
  let busyGroups: Set<string> = $state(new Set());
  let visible: boolean = $state(false);
  // Only one group expanded at a time so it gets the full body for scrolling.
  let expandedGroup: string | null = $state(null);
  let didInitExpand = false;
  // group name -> { node name -> delay ms } from the most recent Test latency
  let groupDelays: Record<string, Record<string, number>> = $state({});
  let unlistens: UnlistenFn[] = [];

  function autoExpand(s: StateSnapshot) {
    if (didInitExpand) return;
    didInitExpand = true;
    const primary = s.groups.find((g) => g.is_primary);
    let target: string | undefined = primary?.name;
    if (!target) {
      target = s.groups.find(
        (g) =>
          g.kind === "Selector" && g.members.length > 1 && g.name !== "GLOBAL",
      )?.name;
    }
    if (!target) target = s.groups[0]?.name;
    if (target) expandedGroup = target;
  }

  // Apply measured group-delay results on top of the snapshot's latest_delay.
  let augmented = $derived.by((): StateSnapshot | null => {
    if (!snapshot) return null;
    if (Object.keys(groupDelays).length === 0) return snapshot;
    return {
      ...snapshot,
      groups: snapshot.groups.map((g) => {
        const measured = groupDelays[g.name];
        if (!measured) return g;
        return {
          ...g,
          members: g.members.map((m) =>
            measured[m.name] !== undefined
              ? { ...m, latest_delay: measured[m.name] }
              : m,
          ),
        };
      }),
    };
  });

  function toggleExpand(name: string) {
    expandedGroup = expandedGroup === name ? null : name;
  }

  function markBusy(group: string, busy: boolean) {
    const next = new Set(busyGroups);
    if (busy) next.add(group);
    else next.delete(group);
    busyGroups = next;
  }

  async function selectNode(group: string, node: string) {
    if (busyGroups.has(group)) return;
    markBusy(group, true);
    try {
      await api.switchProxy(group, node);
    } catch (e) {
      switchError = `Switch failed: ${e}`;
      setTimeout(() => {
        switchError = null;
      }, 3000);
    } finally {
      markBusy(group, false);
    }
  }

  async function testLatency(group: string) {
    markBusy(group, true);
    try {
      const result = await api.testGroupDelay(group);
      groupDelays = { ...groupDelays, [group]: result };
    } catch (e) {
      switchError = `Latency test failed: ${e}`;
      setTimeout(() => {
        switchError = null;
      }, 3000);
    } finally {
      markBusy(group, false);
    }
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") api.hidePopup();
  }

  function ago(ts: number): string {
    const s = Math.max(0, Math.floor((Date.now() - ts) / 1000));
    if (s < 60) return `${s}s ago`;
    return `${Math.floor(s / 60)}m ago`;
  }

  function fmtTime(ts: number): string {
    if (!ts) return "—";
    return new Date(ts).toLocaleTimeString();
  }

  onMount(async () => {
    // First show: fade-in by setting visible after the initial paint at opacity:0.
    requestAnimationFrame(() => {
      visible = true;
    });

    try {
      const s = await api.getState();
      snapshot = s;
      autoExpand(s);
    } catch {
      // ignore — events will populate soon
    }

    unlistens.push(
      await listen<StateSnapshot>("state-updated", (e) => {
        snapshot = e.payload;
        autoExpand(e.payload);
      }),
      await listen<{ url: string; error?: string }>(
        "connection-lost",
        (e) => {
          connectionError = e.payload.error ?? "Connection lost";
        },
      ),
      await listen("connection-restored", () => {
        connectionError = null;
      }),
      // Rust signals show/hide so we can fade the .popup container before/after the
      // window itself is shown/hidden.
      await listen("popup-showing", () => {
        visible = false;
        requestAnimationFrame(() => {
          visible = true;
        });
      }),
      await listen("popup-hiding", () => {
        visible = false;
      }),
    );
  });

  onDestroy(() => {
    for (const u of unlistens) u();
  });
</script>

<svelte:window onkeydown={onKey} />

<div class="popup" class:visible>
  <header>
    <ConnectionDot state={augmented?.connection ?? null} />
    <span class="title">Clash Tray</span>
    <button class="gear" onclick={api.openSettings} title="Settings">⚙</button>
  </header>

  {#if augmented}
    <CurrentProxy
      snapshot={augmented}
      {busyGroups}
      onTest={(g) => testLatency(g)}
    />
  {/if}

  {#if connectionError}
    <div class="banner err">⚠ {connectionError}</div>
  {/if}
  {#if switchError}
    <div class="banner err">⚠ {switchError}</div>
  {/if}

  {#if augmented?.traffic}
    {@const t = augmented.traffic}
    <div class="traffic">
      <div class="row">
        <span class="rate dl" title="Download speed">↓ {formatSpeed(t.download_speed)}</span>
        <span class="rate ul" title="Upload speed">↑ {formatSpeed(t.upload_speed)}</span>
        <span class="conn" title="Active connections">{t.connection_count} conn</span>
      </div>
      <div class="row totals">
        <span class="total" title="Session download total">Σ ↓ {formatBytes(t.download_total)}</span>
        <span class="total" title="Session upload total">Σ ↑ {formatBytes(t.upload_total)}</span>
      </div>
    </div>
  {/if}

  {#if augmented}
    <div class="groups">
      {#each augmented.groups as g (g.name)}
        <ProxyGroup
          group={g}
          expanded={expandedGroup === g.name}
          busy={busyGroups.has(g.name)}
          onToggleExpand={() => toggleExpand(g.name)}
          onSelectNode={(node) => selectNode(g.name, node)}
          onTestLatency={() => testLatency(g.name)}
        />
      {/each}
    </div>

    {#if augmented.subscription && (augmented.subscription.plan || augmented.subscription.remaining || augmented.subscription.expiry || augmented.subscription.reset)}
      {@const sub = augmented.subscription}
      <div class="subscription">
        {#if sub.plan}<span><b>Plan</b> {sub.plan}</span>{/if}
        {#if sub.remaining}<span><b>Left</b> {sub.remaining}</span>{/if}
        {#if sub.expiry}<span><b>Expires</b> {sub.expiry}</span>{/if}
        {#if sub.reset}<span><b>Resets</b> {sub.reset}</span>{/if}
      </div>
    {/if}

    {#if augmented.recent_auto_switches.length > 0}
      {@const last =
        augmented.recent_auto_switches[
          augmented.recent_auto_switches.length - 1
        ]}
      <div class="footer">
        Last auto-switch: <b>{last.group}</b>
        {last.from} → {last.to} · {ago(last.at)}
      </div>
    {/if}

    <div class="meta">
      <span>{augmented.groups.length} groups · {augmented.leaf_count} leaves</span>
      <span class="muted">{fmtTime(augmented.fetched_at)}</span>
    </div>
  {:else}
    <p class="placeholder">Loading…</p>
  {/if}
</div>

<style>
  .popup {
    background: rgba(28, 28, 32, 0.97);
    color: #e6e6e6;
    position: fixed;
    inset: 0;
    box-sizing: border-box;
    padding: 0.7rem;
    border-radius: 10px;
    font-family: "Twemoji Country Flags", system-ui, "Segoe UI", sans-serif;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    border: 1px solid rgba(255, 255, 255, 0.06);
    font-size: 0.85rem;
    overflow: hidden;
    opacity: 0;
    transform: translateY(4px);
    transition: opacity 120ms ease-out, transform 120ms ease-out;
  }
  .popup.visible {
    opacity: 1;
    transform: translateY(0);
  }
  header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-weight: 600;
    padding: 0 0.2em;
  }
  .title {
    flex: 1;
  }
  .gear {
    background: transparent;
    border: none;
    color: #aaa;
    cursor: pointer;
    font-size: 1rem;
    padding: 0 0.3em;
  }
  .gear:hover {
    color: #fff;
  }
  .banner {
    padding: 0.3rem 0.5rem;
    border-radius: 6px;
    font-size: 0.78rem;
  }
  .banner.err {
    background: rgba(231, 76, 60, 0.15);
    border: 1px solid rgba(231, 76, 60, 0.4);
  }
  .traffic {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    padding: 0.35rem 0.55rem;
    background: rgba(255, 255, 255, 0.025);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 6px;
    font-variant-numeric: tabular-nums;
  }
  .traffic .row {
    display: flex;
    gap: 0.6rem;
    font-size: 0.78rem;
    align-items: center;
  }
  .traffic .rate {
    font-weight: 600;
  }
  .traffic .rate.dl { color: #5dade2; }
  .traffic .rate.ul { color: #f5b041; }
  .traffic .conn {
    margin-left: auto;
    color: #888;
    font-size: 0.72rem;
  }
  .traffic .totals {
    color: #888;
    font-size: 0.72rem;
  }
  .subscription {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem 0.7rem;
    padding: 0.3rem 0.5rem;
    font-size: 0.7rem;
    color: #999;
    border-top: 1px solid rgba(255, 255, 255, 0.06);
  }
  .subscription b {
    color: #ccc;
    font-weight: 600;
    margin-right: 0.25em;
  }
  .groups {
    flex: 1 1 0;
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }
  .footer {
    font-size: 0.72rem;
    color: #888;
    padding: 0.25em 0.4em;
    border-top: 1px solid rgba(255, 255, 255, 0.06);
  }
  .meta {
    display: flex;
    justify-content: space-between;
    font-size: 0.68rem;
    color: #777;
    padding: 0 0.2em;
  }
  .muted {
    color: #555;
  }
  .placeholder {
    color: #aaa;
    flex: 1;
    margin: 0;
  }
</style>
