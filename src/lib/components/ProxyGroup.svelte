<script lang="ts">
  import CountryHeader from "./CountryHeader.svelte";
  import DelayBadge from "./DelayBadge.svelte";
  import NodeRow from "./NodeRow.svelte";
  import { iso2ToFlag } from "../country";
  import type { GroupView, NodeView } from "../types";

  let {
    group,
    expanded,
    busy,
    onToggleExpand,
    onSelectNode,
    onTestLatency,
  }: {
    group: GroupView;
    expanded: boolean;
    busy: boolean;
    onToggleExpand: () => void;
    onSelectNode: (node: string) => void;
    onTestLatency: () => void;
  } = $props();

  // Drop subscription pseudo-nodes (fullwidth colon ：) — they are metadata, not switchable nodes.
  let visibleMembers = $derived(
    group.members.filter((m) => !m.name.includes("：")),
  );

  // Treat 0 / null / undefined as "dead" — sort to the bottom.
  function pingRank(d: number | null | undefined): number {
    if (d == null || d === 0) return Number.POSITIVE_INFINITY;
    return d;
  }

  let grouped = $derived.by((): [string, NodeView[]][] => {
    const map = new Map<string, NodeView[]>();
    for (const m of visibleMembers) {
      const c = m.country || "XX";
      if (!map.has(c)) map.set(c, []);
      map.get(c)!.push(m);
    }
    for (const [, nodes] of map) {
      nodes.sort(
        (a, b) =>
          pingRank(a.latest_delay) - pingRank(b.latest_delay) ||
          a.name.localeCompare(b.name),
      );
    }
    return Array.from(map.entries()).sort(([ka, va], [kb, vb]) => {
      if (ka === "XX" && kb !== "XX") return 1;
      if (kb === "XX" && ka !== "XX") return -1;
      const ra = pingRank(va[0]?.latest_delay);
      const rb = pingRank(vb[0]?.latest_delay);
      if (ra !== rb) return ra - rb;
      return ka.localeCompare(kb);
    });
  });
</script>

<div class="group" class:primary={group.is_primary} class:expanded>
  <button class="header" onclick={onToggleExpand}>
    <span class="chev">{expanded ? "▾" : "▸"}</span>
    <span class="flag">{iso2ToFlag(group.now_country || "XX")}</span>
    <span class="name">{group.name}</span>
    <span class="kind">{group.kind}</span>
    {#if group.now_delay !== null && group.now_delay !== undefined}
      <DelayBadge delay={group.now_delay} />
    {/if}
  </button>

  {#if expanded}
    <div class="body">
      <div class="actions">
        <button type="button" onclick={onTestLatency} disabled={busy}>
          Test latency
        </button>
      </div>
      {#each grouped as [country, members] (country)}
        <CountryHeader {country} count={members.length} />
        {#each members as m (m.name)}
          <NodeRow
            node={m}
            selected={m.name === group.now}
            disabled={busy}
            onclick={() => onSelectNode(m.name)}
          />
        {/each}
      {/each}
    </div>
  {/if}
</div>

<style>
  .group {
    background: rgba(255, 255, 255, 0.025);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 6px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  /* Expanded group fills the remaining popup height; its .body owns the scroll. */
  .group.expanded {
    flex: 1 1 0;
  }
  .group.primary {
    border-color: rgba(46, 204, 113, 0.3);
  }
  .header {
    display: grid;
    grid-template-columns: 0.9em 1.1em 1fr auto auto;
    align-items: center;
    gap: 0.4em;
    background: transparent;
    border: none;
    cursor: pointer;
    color: inherit;
    font: inherit;
    text-align: left;
    width: 100%;
    padding: 0.4em 0.55em;
  }
  .header:hover {
    background: rgba(255, 255, 255, 0.04);
  }
  .chev {
    color: #888;
    font-size: 0.7rem;
  }
  .flag {
    font-size: 0.95rem;
  }
  .name {
    font-weight: 600;
    font-size: 0.82rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .kind {
    font-size: 0.62rem;
    color: #888;
    font-weight: 600;
    text-transform: uppercase;
  }
  .body {
    padding: 0.15em 0.4em 0.4em;
    flex: 1 1 0;
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
    scrollbar-gutter: stable;
  }
  .body::-webkit-scrollbar {
    width: 10px;
  }
  .body::-webkit-scrollbar-track {
    background: rgba(255, 255, 255, 0.04);
    border-radius: 5px;
  }
  .body::-webkit-scrollbar-thumb {
    background: rgba(255, 255, 255, 0.25);
    border-radius: 5px;
    border: 2px solid transparent;
    background-clip: padding-box;
    min-height: 30px;
  }
  .body::-webkit-scrollbar-thumb:hover {
    background: rgba(255, 255, 255, 0.42);
    background-clip: padding-box;
  }
  .body::-webkit-scrollbar-thumb:active {
    background: rgba(255, 255, 255, 0.55);
    background-clip: padding-box;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    margin: 0.15em 0;
  }
  .actions button {
    font-size: 0.68rem;
    padding: 0.15em 0.55em;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: #ccc;
    border-radius: 4px;
    cursor: pointer;
  }
  .actions button:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.1);
  }
  .actions button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
