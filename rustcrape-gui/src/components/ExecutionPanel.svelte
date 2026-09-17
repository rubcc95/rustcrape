<script lang="ts">
  import { projectStore } from "../lib/stores/project.store.svelte";  
  import CheckboxNumber from "./CheckboxNumber.svelte";
  import Panel from "./Panel.svelte";
  import type { ExecutionMode } from "../lib/types";

  function onExecutionModeChange(e: Event): void {
    const value = (e.currentTarget as HTMLSelectElement).value as ExecutionMode;
    projectStore.executionConfig.execution_mode = value;
  }
</script>

  <Panel title="Ejecución">
    <div class="field-row">
      <div class="field">
        <label for="execution-mode">Modo de ejecución</label>
        <select
          id="execution-mode"
          value={projectStore.executionConfig.execution_mode}
          onchange={onExecutionModeChange}
        >
          <option value="Sequential">Secuencial</option>
          <option value="Parallel">Paralelo</option>
        </select>
      </div>
      <CheckboxNumber
        label="VPN (frecuencia de rotación)"
        id="ip_rotation_frequency"
        min={1}
        bind:value={projectStore.executionConfig.ip_rotation_frequency}
      />
    </div>
  </Panel>

<style>

  .field-row {
    display: flex;
    gap: 12px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1;
  }

  .field select {
    width: 100%;
    padding: 8px 12px;
    border: 1px solid var(--border, #2a3a5c);
    border-radius: 6px;
    background: var(--bg, #1a1a2e);
    color: var(--text, #e0e0e0);
    font-size: 13px;
    font-family: var(--font);
    outline: none;
    cursor: pointer;
  }

  .field select:focus {
    border-color: var(--primary, #4f8cff);
  }

</style>
