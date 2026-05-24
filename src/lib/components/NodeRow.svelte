<script lang="ts">
  import DelayBadge from "./DelayBadge.svelte";
  import type { NodeView } from "../types";

  let {
    node,
    selected,
    disabled,
    onclick,
  }: {
    node: NodeView;
    selected: boolean;
    disabled?: boolean;
    onclick: () => void;
  } = $props();
</script>

<button class="row" class:selected disabled={disabled} {onclick}>
  <span class="check">{selected ? "✓" : ""}</span>
  <span class="name" title={node.name}>{node.name}</span>
  <DelayBadge delay={node.latest_delay} />
</button>

<style>
  .row {
    display: grid;
    grid-template-columns: 1.1em 1fr auto;
    gap: 0.4em;
    align-items: center;
    padding: 0.22em 0.5em;
    background: transparent;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    color: inherit;
    font: inherit;
    text-align: left;
    width: 100%;
  }
  .row:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.04);
  }
  .row.selected {
    background: rgba(46, 204, 113, 0.07);
  }
  .row:disabled {
    cursor: not-allowed;
    opacity: 0.55;
  }
  .check {
    color: #2ecc71;
    font-weight: 700;
    text-align: center;
    font-size: 0.85rem;
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.78rem;
  }
</style>
