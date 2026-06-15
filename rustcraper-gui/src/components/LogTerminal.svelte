<script lang="ts">
  import type { LogEntry } from "../lib/types";

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

<div class="panel">
    <h4>Log</h4>
  
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
</div>

<style>
  .panel {
    flex: 1;
    background: var(--card-bg, #1f2b47);
    border: 0px;
    border-radius: 8px;
    padding: 16px;
    display: flex;
    flex-direction: column;
    min-height: 200px;
  }

  .panel h4 {
    font-size: 12px;
    margin: 0 0 8px 0;
    text-transform: uppercase;
    letter-spacing: 1px;
    color: var(--primary);
  }

  .log-output {
    flex: 1;
    overflow-y: auto;
    padding: 8px 0ox;
    font-family: "JetBrains Mono", "Fira Code", "Consolas", monospace;
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
