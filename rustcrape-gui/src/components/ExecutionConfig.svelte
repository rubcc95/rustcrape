<script lang="ts">
  import { projectStore } from "../lib/stores/project.store.svelte";
  import NumberInput from "./NumberInput.svelte";
  import CheckboxNumber from "./CheckboxNumber.svelte";
  import type { ExecutionMode } from "../lib/types";

  function onExecutionModeChange(e: Event): void {
    const value = (e.currentTarget as HTMLSelectElement).value as ExecutionMode;
    projectStore.executionConfig.execution_mode = value;
  }
</script>

<div class="panel">
  <h4>Ejecución</h4>

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
</div>

{#if projectStore.projectDraft.enable_google_maps}
  <div class="panel">
    <h4>Google Maps</h4>

    <div class="field-row">
      <CheckboxNumber
        label="Rate limit (por hora)"
        id="gm-rate-limit"
        min={1}
        bind:value={projectStore.executionConfig.google_maps.rate_limit}
      />
      <CheckboxNumber
        label="Iteraciones"
        id="gm-iterations"
        min={1}
        bind:value={projectStore.executionConfig.google_maps.iterations}
      />
    </div>

    <div class="field-row">
      <NumberInput
        label="Delay min (ms)"
        id="gm-delay-min"
        min={100}
        bind:value={projectStore.executionConfig.google_maps.delay_min}
      />
      <NumberInput
        label="Delay max (ms)"
        id="gm-delay-max"
        min={100}
        bind:value={projectStore.executionConfig.google_maps.delay_max}
      />
    </div>

    <div class="field-row">
      <NumberInput
        label="Stop threshold"
        id="gm-stop-threshold"
        min={1}
        bind:value={projectStore.executionConfig.google_maps.stop_threshold}
      />
      <div class="field toggle-field">
        <!-- svelte-ignore a11y_label_has_associated_control -->
        <label>Headless</label>
        <div
          class="check-track"
          class:checked={projectStore.executionConfig.google_maps.headless}
          role="checkbox"
          aria-checked={projectStore.executionConfig.google_maps.headless}
          aria-label="Headless Google Maps"
          tabindex="0"
          onclick={() => projectStore.executionConfig.google_maps.headless = !projectStore.executionConfig.google_maps.headless}
          onkeydown={(e) => e.key === 'Enter' && (projectStore.executionConfig.google_maps.headless = !projectStore.executionConfig.google_maps.headless)}
        >
          <svg class="check-mark" viewBox="0 0 24 24" width="18" height="18">
            <path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z" fill="currentColor" />
          </svg>
        </div>
      </div>
    </div>
  </div>
{/if}

{#if projectStore.projectDraft.enable_empresite}
  <div class="panel">
    <h4>Empresite</h4>

    <div class="field-row">
      <CheckboxNumber
        label="Rate limit (por hora)"
        id="em-rate-limit"
        min={1}
        bind:value={projectStore.executionConfig.empresite.rate_limit}
      />
      <CheckboxNumber
        label="Iteraciones"
        id="em-iterations"
        min={1}
        bind:value={projectStore.executionConfig.empresite.iterations}
      />
    </div>

    <div class="field-row">
      <NumberInput
        label="Delay min (ms)"
        id="em-delay-min"
        min={100}
        bind:value={projectStore.executionConfig.empresite.delay_min}
      />
      <NumberInput
        label="Delay max (ms)"
        id="em-delay-max"
        min={100}
        bind:value={projectStore.executionConfig.empresite.delay_max}
      />
    </div>

    <div class="field-row">
      <div class="field toggle-field">
        <!-- svelte-ignore a11y_label_has_associated_control -->
        <label>Headless</label>
        <div
          class="check-track"
          class:checked={projectStore.executionConfig.empresite.headless}
          role="checkbox"
          aria-checked={projectStore.executionConfig.empresite.headless}
          aria-label="Headless Empresite"
          tabindex="0"
          onclick={() => projectStore.executionConfig.empresite.headless = !projectStore.executionConfig.empresite.headless}
          onkeydown={(e) => e.key === 'Enter' && (projectStore.executionConfig.empresite.headless = !projectStore.executionConfig.empresite.headless)}
        >
          <svg class="check-mark" viewBox="0 0 24 24" width="18" height="18">
            <path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z" fill="currentColor" />
          </svg>
        </div>
      </div>
    </div>
  </div>
{/if}

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

  .panel + .panel {
    margin-top: 20px;
  }

  .panel h4 {
    font-size: 12px;
    margin: 0 0 8px 0;
    text-transform: uppercase;
    letter-spacing: 1px;
    color: var(--primary);
  }

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

  .toggle-field label {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-muted, #8892b0);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .check-track {
    width: 34px;
    height: 34px;
    border: 1px solid var(--border, #2a3a5c);
    border-radius: 6px;
    background: var(--bg);
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .check-track:hover {
    border-color: var(--text-muted, #8892b0);
  }

  .check-track.checked {
    background: var(--primary, #4f8cff);
    border-color: var(--primary, #4f8cff);
  }

  .check-mark {
    color: #fff;
    opacity: 0;
    transform: scale(0.5);
    transition: opacity 0.15s, transform 0.15s;
  }

  .check-track.checked .check-mark {
    opacity: 1;
    transform: scale(1);
  }
</style>
