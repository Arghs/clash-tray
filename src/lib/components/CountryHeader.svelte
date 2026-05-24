<script lang="ts">
  import { iso2ToDisplay, iso2ToFlag } from "../country";
  let { country, count }: { country: string; count: number } = $props();
</script>

<div class="head">
  <span class="flag">{iso2ToFlag(country)}</span>
  <span class="name">{iso2ToDisplay(country)}</span>
  <span class="count">{count}</span>
</div>

<style>
  .head {
    display: flex;
    align-items: center;
    gap: 0.4em;
    padding: 0.3em 0.2em 0.1em;
    font-size: 0.7rem;
    color: #aaa;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.02em;
  }
  .flag { font-size: 0.95rem; }
  .name { font-size: 0.7rem; }
  .count { margin-left: auto; color: #777; font-weight: 400; }
</style>
