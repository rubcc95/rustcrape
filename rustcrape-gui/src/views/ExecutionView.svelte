<script lang="ts">
  import { executionStore } from "../lib/stores/execution.store.svelte";
  import LogTerminal from "../components/LogTerminal.svelte";
  import ProgressPanel from "../components/ProgressPanel.svelte";
  import Panel from "../components/Panel.svelte";
</script>

<div class="execution-view">
  <div class="execution-panels">
    <Panel title="Configuración utilizada" flex gap="0" class="config-panel">
      {#if executionStore.configSnapshot}
        <div class="config-info">
          <div class="config-row">
            <span class="config-label">Rate limit</span>
            <span class="config-value"
              >{executionStore.configSnapshot.rate_limit}/h</span
            >
          </div>
          <div class="config-row">
            <span class="config-label">Delay</span>
            <span class="config-value"
              >{executionStore.configSnapshot.search.delay_min}ms -
              {executionStore.configSnapshot.search.delay_max}ms</span
            >
          </div>
          <div class="config-row">
            <span class="config-label">Stop threshold</span>
            <span class="config-value"
              >{executionStore.configSnapshot.search.stop_threshold}</span
            >
          </div>
          <div class="config-row">
            <span class="config-label">VPN</span>
            <span class="config-value"
              >{executionStore.configSnapshot.nordvpn_path
                ? "Sí"
                : "No"}</span
            >
          </div>
          <div class="config-row">
            <span class="config-label">Rotación IP</span>
            <span class="config-value"
              >{executionStore.configSnapshot.ip_rotation_frequency}</span
            >
          </div>
          <div class="config-row">
            <span class="config-label">Headless</span>
            <span class="config-value"
              >{executionStore.configSnapshot.search.headless
                ? "Sí"
                : "No"}</span
            >
          </div>
        </div>
      {:else}
        <p class="text-muted">Sin datos de configuración</p>
      {/if}
    </Panel>

    <ProgressPanel stats={executionStore.stats} />
  </div>

  <LogTerminal logs={executionStore.logs} />
</div>

<style>

  .execution-view {
    width: 90%;
    margin: 0 auto;
    overflow: hidden;
  }

  @media(min-width: 980px){
    .execution-view{
      width: 840px;
    }
  }

  .execution-panels {
    display: flex;
    gap: 24px;
    margin-bottom: 16px;
  }

  .config-info {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .config-row {
    display: flex;
    justify-content: space-between;
    font-size: 13px;
  }

  .config-label {
    color: var(--text-muted, #8892b0);
  }

  .config-value {
    color: var(--text, #e0e0e0);
    font-weight: 500;
  }

  .text-muted {
    color: var(--text-muted, #8892b0);
    font-size: 13px;
  }

</style>
