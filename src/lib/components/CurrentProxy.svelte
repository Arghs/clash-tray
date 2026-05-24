<script lang="ts">
  import { iso2ToDisplay, iso2ToFlag } from "../country";
  import type { GroupKind, GroupView, StateSnapshot } from "../types";
  import DelayBadge from "./DelayBadge.svelte";

  let {
    snapshot,
    busyGroups,
    onTest,
  }: {
    snapshot: StateSnapshot;
    busyGroups: Set<string>;
    onTest: (group: string) => void;
  } = $props();

  type Status = "ok" | "direct" | "reject" | "unset" | "no-primary";

  type Resolution = {
    status: Status;
    primaryName: string | null;
    leafName: string | null;
    leafCountry: string;
    leafDelay: number | null;
    /** Group names traversed between primary and leaf, both endpoints excluded. */
    intermediates: string[];
    /** The deepest non-leaf group — the one whose `now` is the leaf. Null when
     *  there's nothing meaningful to test (no primary, unset, direct, reject). */
    testGroupName: string | null;
    /** Kind of the deepest non-leaf group. Drives the mode badge. */
    leafGroupKind: GroupKind | null;
  };

  function findPrimary(snap: StateSnapshot): GroupView | undefined {
    return (
      snap.groups.find((g) => g.is_primary) ??
      snap.groups.find((g) => g.kind === "Selector" && g.name !== "GLOBAL")
    );
  }

  // Walk primary -> primary.now -> (if that's a group) child.now -> ... until we
  // hit a leaf. Cap at 4 hops in case a config has cycles.
  function resolve(snap: StateSnapshot): Resolution {
    const primary = findPrimary(snap);
    if (!primary) {
      return {
        status: "no-primary",
        primaryName: null,
        leafName: null,
        leafCountry: "XX",
        leafDelay: null,
        intermediates: [],
        testGroupName: null,
        leafGroupKind: null,
      };
    }
    if (!primary.now) {
      return {
        status: "unset",
        primaryName: primary.name,
        leafName: null,
        leafCountry: "XX",
        leafDelay: null,
        intermediates: [],
        testGroupName: null,
        leafGroupKind: null,
      };
    }

    const chain: GroupView[] = [primary];
    let cur: GroupView = primary;
    let nextName: string = primary.now;

    for (let i = 0; i < 4; i++) {
      const child = snap.groups.find((g) => g.name === nextName);
      if (!child) break; // nextName is a leaf
      chain.push(child);
      cur = child;
      if (!child.now) {
        nextName = "";
        break;
      }
      nextName = child.now;
    }

    if (!nextName) {
      return {
        status: "unset",
        primaryName: primary.name,
        leafName: null,
        leafCountry: cur.now_country ?? "XX",
        leafDelay: null,
        intermediates: chain.slice(1).map((g) => g.name),
        testGroupName: null,
        leafGroupKind: null,
      };
    }

    if (nextName === "DIRECT") {
      return {
        status: "direct",
        primaryName: primary.name,
        leafName: "DIRECT",
        leafCountry: "XX",
        leafDelay: null,
        intermediates: chain.slice(1).map((g) => g.name),
        testGroupName: null,
        leafGroupKind: null,
      };
    }
    if (nextName === "REJECT") {
      return {
        status: "reject",
        primaryName: primary.name,
        leafName: "REJECT",
        leafCountry: "XX",
        leafDelay: null,
        intermediates: chain.slice(1).map((g) => g.name),
        testGroupName: null,
        leafGroupKind: null,
      };
    }

    const leaf = cur.members.find((m) => m.name === nextName);

    return {
      status: "ok",
      primaryName: primary.name,
      leafName: nextName,
      leafCountry: leaf?.country ?? cur.now_country ?? "XX",
      leafDelay: leaf?.latest_delay ?? cur.now_delay ?? null,
      intermediates: chain.slice(1).map((g) => g.name),
      testGroupName: cur.name,
      leafGroupKind: cur.kind,
    };
  }

  let resolution: Resolution = $derived(resolve(snapshot));

  let displayFlag = $derived.by(() => {
    switch (resolution.status) {
      case "direct":
        return "🏠";
      case "reject":
        return "⛔";
      case "no-primary":
      case "unset":
        return "—";
      default:
        return resolution.leafCountry === "XX"
          ? "🌐"
          : iso2ToFlag(resolution.leafCountry);
    }
  });

  let displayTitle = $derived.by(() => {
    switch (resolution.status) {
      case "direct":
        return "Direct";
      case "reject":
        return "Reject";
      case "no-primary":
        return "No primary group";
      case "unset":
        return "Not selected";
      default:
        return resolution.leafCountry === "XX"
          ? "Unknown region"
          : iso2ToDisplay(resolution.leafCountry);
    }
  });

  let displaySubtitle = $derived.by(() => {
    switch (resolution.status) {
      case "no-primary":
        return "Pick one in Settings → Primary group";
      case "unset":
        return `${resolution.primaryName}: no node selected`;
      case "direct":
      case "reject":
        return resolution.leafName!;
      default:
        return resolution.leafName ?? "—";
    }
  });

  let dimmed = $derived(snapshot.connection !== "Connected");
  let testing = $derived(
    !!resolution.testGroupName && busyGroups.has(resolution.testGroupName),
  );

  const MODE_LABELS: Record<GroupKind, string> = {
    Selector: "Manual",
    UrlTest: "Auto",
    Fallback: "Failover",
    LoadBalance: "Load balance",
    Relay: "Relay",
  };

  let modeLabel = $derived(
    resolution.leafGroupKind ? MODE_LABELS[resolution.leafGroupKind] : null,
  );
  let modeClass = $derived(
    resolution.leafGroupKind
      ? resolution.leafGroupKind.toLowerCase()
      : "",
  );

  function handleTest() {
    if (resolution.testGroupName && !testing) onTest(resolution.testGroupName);
  }
</script>

<div
  class="hero"
  class:dimmed
  class:warn={resolution.status === "no-primary" ||
    resolution.status === "unset"}
>
  <div class="flag">{displayFlag}</div>
  <div class="text">
    <div class="title-row">
      <span class="title" title={displayTitle}>{displayTitle}</span>
      {#if modeLabel}
        <span class="mode {modeClass}" title={`Selection mode: ${modeLabel}`}>
          {modeLabel}
        </span>
      {/if}
      {#if resolution.status === "ok"}
        <DelayBadge delay={resolution.leafDelay} />
        <button
          class="test"
          class:spinning={testing}
          onclick={handleTest}
          disabled={testing}
          title={`Test latency for ${resolution.testGroupName}`}
          aria-label="Test latency"
        >↻</button>
      {/if}
    </div>
    <div class="subtitle" title={displaySubtitle}>{displaySubtitle}</div>
    {#if resolution.intermediates.length > 0}
      <div class="chain">via {resolution.intermediates.join(" → ")}</div>
    {/if}
  </div>
</div>

<style>
  .hero {
    display: grid;
    grid-template-columns: auto 1fr;
    align-items: center;
    gap: 0.75rem;
    padding: 0.6rem 0.7rem;
    background: rgba(46, 204, 113, 0.06);
    border: 1px solid rgba(46, 204, 113, 0.25);
    border-radius: 8px;
    transition: opacity 200ms ease-out;
  }
  .hero.warn {
    background: rgba(241, 196, 15, 0.06);
    border-color: rgba(241, 196, 15, 0.3);
  }
  .hero.dimmed {
    opacity: 0.55;
    filter: saturate(0.6);
  }
  .flag {
    font-size: 2rem;
    line-height: 1;
    text-align: center;
    min-width: 2.4rem;
  }
  .text {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
  }
  .title-row {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    min-width: 0;
  }
  .title {
    font-size: 1.05rem;
    font-weight: 600;
    color: #f0f0f0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1 1 auto;
    min-width: 0;
  }
  .subtitle {
    font-size: 0.78rem;
    color: #cfcfcf;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }
  .chain {
    font-size: 0.7rem;
    color: #888;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    margin-top: 0.05rem;
  }
  .mode {
    font-size: 0.62rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.03em;
    padding: 0.1em 0.45em;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: #bbb;
    flex-shrink: 0;
    white-space: nowrap;
  }
  .mode.urltest {
    color: #7fc1f0;
    background: rgba(127, 193, 240, 0.1);
    border-color: rgba(127, 193, 240, 0.3);
  }
  .mode.fallback {
    color: #ddb44d;
    background: rgba(241, 196, 15, 0.1);
    border-color: rgba(241, 196, 15, 0.3);
  }
  .mode.loadbalance {
    color: #a78bf3;
    background: rgba(167, 139, 243, 0.1);
    border-color: rgba(167, 139, 243, 0.3);
  }
  .test {
    background: transparent;
    border: 1px solid rgba(255, 255, 255, 0.12);
    color: #bbb;
    cursor: pointer;
    border-radius: 4px;
    width: 1.6em;
    height: 1.6em;
    line-height: 1;
    padding: 0;
    font-size: 0.9rem;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .test:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.08);
    color: #fff;
  }
  .test:disabled {
    opacity: 0.6;
    cursor: progress;
  }
  .test.spinning {
    animation: spin 1s linear infinite;
  }
  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }
</style>
