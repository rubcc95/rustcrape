<script lang="ts">
  import { projectStore } from "../lib/stores/project.store.svelte";
  import { executionStore } from "../lib/stores/execution.store.svelte";
  import { fetchProjectStats } from "../lib/tauri";
  import Panel from "./Panel.svelte";
  import { isMysql } from "../lib/types";

  $effect(() => {
    const project = projectStore.currentProject;
    if (project) {
      fetchProjectStats(project.config).then((s) => {
        executionStore.stats.bounds_total = s.bounds_total;
        executionStore.stats.bounds_processed = s.bounds_processed;
        executionStore.stats.bounds_remaining = s.bounds_remaining;
        executionStore.stats.results_found = s.results_found;
        executionStore.stats.phones_found = s.phones_found;
      }).catch(() => {
        // DB not accessible yet — keep zeros
      });
    }
  });
</script>

{#if projectStore.currentProject}
  <div class="info-panels">
    <Panel title="Información">
      <div class="info-grid">
        <div class="info-item">
          <span class="info-label">Término de búsqueda</span>
          <span class="info-value"
            >{projectStore.currentProject.config.search.persistent.search_query}</span
          >
        </div>
        <div class="info-item">
          <span class="info-label">Zoom</span>
          <span class="info-value"
            >{projectStore.currentProject.config.search.persistent.zoom}</span
          >
        </div>
        <div class="info-item">
          <span class="info-label">Base de datos</span>
          <span class="info-value">
            {#if isMysql(projectStore.currentProject.config.db)}
              {projectStore.currentProject.config.db.database}@{projectStore.currentProject.config.db.host}
            {:else if projectStore.currentProject.config.db.path}
              {projectStore.currentProject.config.db.path}
            {:else}
              SQLite local
            {/if}
          </span>
        </div>
        <div class="info-item">
          <span class="info-label">Mapa</span>
          <span class="info-value">España</span>
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
          <span class="stat-value">{executionStore.stats.bounds_processed}</span>
          <span class="stat-label">Bounds analizados</span>
        </div>
        <div class="stat-card">
          <span class="stat-value">{executionStore.stats.bounds_remaining}</span>
          <span class="stat-label">Bounds restantes</span>
        </div>
        <div class="stat-card">
          <span class="stat-value">{executionStore.stats.results_found}</span>
          <span class="stat-label">Resultados</span>
        </div>
        <div class="stat-card">
          <span class="stat-value">{executionStore.stats.phones_found}</span>
          <span class="stat-label">Teléfonos</span>
        </div>
      </div>
    </Panel>
  </div>
{/if}

<style>
  .info-panels {
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
</style>
