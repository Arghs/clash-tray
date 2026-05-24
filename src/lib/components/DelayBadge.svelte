<script lang="ts">
  let { delay }: { delay: number | null } = $props();

  let cls = $derived(
    delay === null || delay === undefined
      ? "unknown"
      : delay === 0
        ? "bad"
        : delay < 200
          ? "good"
          : delay < 500
            ? "ok"
            : "bad",
  );
  let text = $derived(
    delay === null || delay === undefined
      ? "—"
      : delay === 0
        ? "✕"
        : `${delay}`,
  );
</script>

<span class="badge {cls}">{text}</span>

<style>
  .badge {
    display: inline-block;
    min-width: 2.5em;
    text-align: center;
    padding: 0 0.4em;
    border-radius: 8px;
    font-size: 0.68rem;
    font-weight: 500;
    font-variant-numeric: tabular-nums;
  }
  .good { background: rgba(46, 204, 113, 0.2);  color: #5fdda4; }
  .ok   { background: rgba(241, 196, 15, 0.18); color: #ddb44d; }
  .bad  { background: rgba(231, 76, 60, 0.18);  color: #e87a6e; }
  .unknown { background: rgba(150, 150, 150, 0.15); color: #999; }
</style>
