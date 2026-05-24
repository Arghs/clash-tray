<script lang="ts">
  import type { ConnectionState } from "../types";
  let { state }: { state: ConnectionState | null } = $props();
</script>

<span
  class="dot"
  class:c={state === "Connected"}
  class:d={state === "Degraded"}
  class:l={state === "Lost"}
></span>

<style>
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #666;
    display: inline-block;
  }
  .c { background: #2ecc71; }
  .d { background: #f39c12; }
  .l { background: #e74c3c; }
</style>
