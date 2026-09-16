<script lang="ts">
  import { executionStore } from "../lib/stores/execution.store.svelte";
  import LogTerminal from "../components/LogTerminal.svelte";
  import ProgressPanel from "../components/ProgressPanel.svelte";
</script>

<div class="execution-view">
  <div class="execution-panels">
    <div class="panel config-panel">
      <h4>Configuración utilizada</h4>
      {#if executionStore.configSnapshot}
        <div class="config-info">
          <div class="config-row">
            <span class="config-label">Modo</span>
            <span class="config-value"
              >{executionStore.configSnapshot.execution_mode === "Parallel"
                ? "Paralelo"
                : "Secuencial"}</span
            >
          </div>
          <div class="config-row">
            <span class="config-label">Google Maps</span>
            <span class="config-value"
              >{executionStore.configSnapshot.gmaps.enabled
                ? "Sí"
                : "No"}</span
            >
          </div>
          {#if executionStore.configSnapshot.gmaps.enabled}
            <div class="config-row">
              <span class="config-label">Delay Maps</span>
              <span class="config-value"
                >{executionStore.configSnapshot.gmaps.delay_min}ms -
                {executionStore.configSnapshot.gmaps.delay_max}ms</span
              >
            </div>
            <div class="config-row">
              <span class="config-label">Stop threshold</span>
              <span class="config-value"
                >{executionStore.configSnapshot.gmaps.stop_threshold}</span
              >
            </div>
          {/if}
          <div class="config-row">
            <span class="config-label">Empresite</span>
            <span class="config-value"
              >{executionStore.configSnapshot.empresite.enabled
                ? "Sí"
                : "No"}</span
            >
          </div>
          {#if executionStore.configSnapshot.empresite.enabled}
            <div class="config-row">
              <span class="config-label">Delay Empresite</span>
              <span class="config-value"
                >{executionStore.configSnapshot.empresite.delay_min}ms -
                {executionStore.configSnapshot.empresite.delay_max}ms</span
              >
            </div>
          {/if}
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
        </div>
      {:else}
        <p class="text-muted">Sin datos de configuración</p>
      {/if}
    </div>

    <ProgressPanel stats={executionStore.stats} />
  </div>

  <LogTerminal logs={executionStore.logs} />
</div>

<style>

  .execution-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    width: 90%;
    margin: 0 auto;
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

  .panel {
    flex: 1;
    background: var(--card-bg, #1f2b47);
    border: 0px;
    border-radius: 8px;
    padding: 16px;
  }
 
  .panel h4 {
    font-size: 12px;
    margin: 0 0 8px 0;
    text-transform: uppercase;
    letter-spacing: 1px;
    color: var(--primary, #8892b0);
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
