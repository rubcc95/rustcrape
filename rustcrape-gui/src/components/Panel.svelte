<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    title = "",
    children,
  }: {
    title?: string;
    children: Snippet;
  } = $props();
</script>

<section class="panel">
  {#if title}
    <h4>{title}</h4>
  {/if}
  {@render children()}
</section>

<style>
  .panel {
    background-color: var(--card-bg);
    border: 0px;
    border-radius: 8px;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-width: 0;
  }

  .panel h4 {
    font-size: 12px;
    margin: 0 0 8px 0;
    text-transform: uppercase;
    letter-spacing: 1px;
    color: var(--primary);
  }
</style>
