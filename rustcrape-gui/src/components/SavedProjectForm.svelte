<script lang="ts">
  import { projectStore } from "../lib/stores/project.store.svelte";
  import { resultsStore } from "../lib/stores/results.store.svelte";
  import { isSqlite } from "../lib/types";
  import CheckboxNumber from "./CheckboxNumber.svelte";
  import ExecutionPanel from "./ExecutionPanel.svelte";
  import NumberInput from "./NumberInput.svelte";
  import Panel from "./Panel.svelte";

  function dbLabel(): string {
    const project = projectStore.currentProject;
    if (!project) return "";
    const db = project.config.db;
    return isSqlite(db)
      ? (db.path ?? "SQLite local")
      : `${db.user}@${db.host}:${db.port}/${db.database}`;
  }

  function onOpenResults(): void {
    void resultsStore.openForCurrentProject();
  }
</script>

{#if projectStore.currentProject}
  <div class="panels">
    <Panel title="Información">
      <div class="info-grid">
        <div class="info-item">
          <span class="info-label">Término de búsqueda</span>
          <span class="info-value"
            >{projectStore.currentProject.config.gmaps.search_query}</span
          >
        </div>
        <div class="info-item">
          <span class="info-label">Zoom</span>
          <span class="info-value"
            >{projectStore.currentProject.config.gmaps.zoom}</span
          >
        </div>
        <div class="info-item">
          <span class="info-label">Google Maps</span>
          <span class="info-value"
            >{projectStore.currentProject.config.gmaps.enabled
              ? "Sí"
              : "No"}</span
          >
        </div>
        <div class="info-item">
          <span class="info-label">Empresite</span>
          <span class="info-value"
            >{projectStore.currentProject.config.empresite.enabled
              ? "Sí"
              : "No"}</span
          >
        </div>
        <div class="info-item">
          <span class="info-label">Modo de ejecución</span>
          <span class="info-value"
            >{projectStore.currentProject.config.execution_mode === "Parallel"
              ? "Paralelo"
              : "Secuencial"}</span
          >
        </div>
        <div class="info-item">
          <span class="info-label">Base de datos</span>
          <span class="info-value">{dbLabel()}</span>
        </div>
        <div class="info-item">
          <span class="info-label">Ejecutado</span>
          <span class="info-value"
            >{projectStore.currentProject.started ? "Sí" : "No"}</span
          >
        </div>
      </div>
    </Panel>

    <Panel title="Estado">
      <div class="stats-grid">
        <div class="stat-card">
          <span class="stat-value">{projectStore.stats.bounds_processed}</span>
          <span class="stat-label">Tareas analizadas</span>
        </div>
        <div class="stat-card">
          <span class="stat-value">{projectStore.stats.bounds_remaining}</span>
          <span class="stat-label">Tareas restantes</span>
        </div>
        <div
          class="stat-card clickable"
          role="button"
          tabindex="0"
          onclick={onOpenResults}
          onkeydown={(e) => e.key === "Enter" && onOpenResults()}
        >
          <span class="stat-value">{projectStore.stats.results_found}</span>
          <span class="stat-label">Resultados</span>
        </div>
        <div class="stat-card">
          <span class="stat-value">{projectStore.stats.phones_found}</span>
          <span class="stat-label">Teléfonos</span>
        </div>
      </div>
    </Panel>

  <ExecutionPanel />

    {#if projectStore.projectDraft.enable_google_maps}
      <Panel title="Google Maps">
        <div class="field-row">
          <CheckboxNumber
            label="Rate limit (por hora)"
            id="gm-rate-limit"
            min={1}
            bind:value={projectStore.executionConfig.gmaps.rate_limit}
          />
          <CheckboxNumber
            label="Iteraciones"
            id="gm-iterations"
            min={1}
            bind:value={projectStore.executionConfig.gmaps.iterations}
          />
        </div>

        <div class="field-row">
          <NumberInput
            label="Delay min (ms)"
            id="gm-delay-min"
            min={100}
            bind:value={projectStore.executionConfig.gmaps.delay_min}
          />
          <NumberInput
            label="Delay max (ms)"
            id="gm-delay-max"
            min={100}
            bind:value={projectStore.executionConfig.gmaps.delay_max}
          />
        </div>

        <div class="field-row">
          <NumberInput
            label="Stop threshold"
            id="gm-stop-threshold"
            min={1}
            bind:value={projectStore.executionConfig.gmaps.stop_threshold}
          />
          <div class="field toggle-field">
            <!-- svelte-ignore a11y_label_has_associated_control -->
            <label>Headless</label>
            <div
              class="check-track"
              class:checked={projectStore.executionConfig.gmaps.headless}
              role="checkbox"
              aria-checked={projectStore.executionConfig.gmaps.headless}
              aria-label="Headless Google Maps"
              tabindex="0"
              onclick={() =>
                (projectStore.executionConfig.gmaps.headless =
                  !projectStore.executionConfig.gmaps.headless)}
              onkeydown={(e) =>
                e.key === "Enter" &&
                (projectStore.executionConfig.gmaps.headless =
                  !projectStore.executionConfig.gmaps.headless)}
            >
              <svg
                class="check-mark"
                viewBox="0 0 24 24"
                width="18"
                height="18"
              >
                <path
                  d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z"
                  fill="currentColor"
                />
              </svg>
            </div>
          </div>
        </div>
      </Panel>
    {/if}

    {#if projectStore.projectDraft.enable_empresite}
      <Panel title="Empresite">
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
              onclick={() =>
                (projectStore.executionConfig.empresite.headless =
                  !projectStore.executionConfig.empresite.headless)}
              onkeydown={(e) =>
                e.key === "Enter" &&
                (projectStore.executionConfig.empresite.headless =
                  !projectStore.executionConfig.empresite.headless)}
            >
              <svg
                class="check-mark"
                viewBox="0 0 24 24"
                width="18"
                height="18"
              >
                <path
                  d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z"
                  fill="currentColor"
                />
              </svg>
            </div>
          </div>
        </div>
      </Panel>
    {/if}
  </div>
{/if}

<style>
  .panels {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .info-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }

  .info-item {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .info-label {
    font-size: 11px;
    color: var(--text-muted, #8892b0);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .info-value {
    font-size: 14px;
    color: var(--text, #e0e0e0);
    font-weight: 500;
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 12px;
  }

  .stat-card {
    background: var(--bg, #1a1a2e);
    border: 1px solid var(--border, #2a3a5c);
    border-radius: 8px;
    padding: 16px;
    text-align: center;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .stat-value {
    font-size: 24px;
    font-weight: 700;
    color: var(--primary, #4f8cff);
  }

  .stat-label {
    font-size: 11px;
    color: var(--text-muted, #8892b0);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .stat-card.clickable {
    cursor: pointer;
    transition:
      border-color 0.15s,
      background 0.15s;
  }

  .stat-card.clickable:hover {
    border-color: var(--primary, #4f8cff);
    background: rgba(79, 140, 255, 0.08);
  }

  .stat-card.clickable:focus-visible {
    outline: 2px solid var(--primary, #4f8cff);
    outline-offset: 2px;
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
    transition:
      background 0.15s,
      border-color 0.15s;
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
    transition:
      opacity 0.15s,
      transform 0.15s;
  }

  .check-track.checked .check-mark {
    opacity: 1;
    transform: scale(1);
  }
</style>
