<script lang="ts">
  import type { LogEntry } from "../lib/types";
  import Panel from "./Panel.svelte";

  let { logs }: { logs: LogEntry[] } = $props();
  let terminalEl: HTMLDivElement | undefined = $state();

  $effect(() => {
    if (logs.length > 0 && terminalEl) {
      requestAnimationFrame(() => {
        terminalEl!.scrollTop = terminalEl!.scrollHeight;
      });
    }
  });
</script>

<Panel title="Log" flex >
  <div class="log-output" bind:this={terminalEl}>
    {#each logs as log (log.id)}
      <div class="log-entry log-{log.kind}">
        [{log.timestamp}] {log.message}
      </div>
    {/each}

    {#if logs.length === 0}
      <div class="log-empty">Esperando eventos...</div>
    {/if}
  </div>
</Panel>

<style>

  .log-output {
    overflow-y: auto;
    min-height: 200px;
    max-height: calc(100vh - 420px);
    padding: 8px 0;
    font-family: "Consolas", "JetBrains Mono", "Fira Code", monospace;
    font-size: 12px;
    line-height: 1.6;
  }

  .log-entry {
    padding: 1px 0;
    word-break: break-word;
  }

  .log-empty {
    color: var(--text-muted, #8892b0);
    font-style: italic;
  }

  :global(.log-info) {
    color: var(--info, #3498db);
  }

  :global(.log-warn) {
    color: var(--warn, #f39c12);
  }

  :global(.log-error) {
    color: var(--error, #e74c3c);
  }

  :global(.log-opening_browser),
  :global(.log-scraping_start),
  :global(.log-generating_bounds),
  :global(.log-connecting_db),
  :global(.log-obtaining_bound) {
    color: var(--text, #e0e0e0);
  }

  :global(.log-processed_coincidence),
  :global(.log-written_coincidences),
  :global(.log-writing_coincidences),
  :global(.log-finished) {
    color: var(--run, #2ecc71);
  }

  :global(.log-accepting_cookies),
  :global(.log-searching_coincidences),
  :global(.log-found_single_coincidence),
  :global(.log-found_multiple_coincidences),
  :global(.log-closing_browser),
  :global(.log-released_bound) {
    color: var(--text-muted, #8892b0);
  }
</style>
