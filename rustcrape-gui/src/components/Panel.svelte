<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    title = "",
    gap = "12px",
    flex = false,
    class: className = "",
    children,
  }: {
    title?: string;
    gap?: string;
    flex?: boolean;
    class?: string;
    children?: Snippet;
  } = $props();
</script>

<div class="panel {className}" style="gap: {gap}" class:panel--flex={flex} role="region" aria-label={title || undefined}>
  {#if title}
    <h4>{title}</h4>
  {/if}
  {@render children?.()}
</div>

<style>
  .panel {
    background: var(--card-bg);
    border: 0px;
    border-radius: 8px;
    padding: 16px;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .panel--flex {
    flex: 1;
  }

  .panel h4 {
    font-size: 12px;
    margin: 0;
    text-transform: uppercase;
    letter-spacing: 1px;
    color: var(--primary);
  }
</style>
