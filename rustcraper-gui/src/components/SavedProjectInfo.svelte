<script lang="ts">
  import { projectStore } from "../lib/stores/project.store.svelte";
</script>

{#if projectStore.currentProject}
  <div class="panel">
    <h4>Información</h4>

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
        <span class="info-value"
          >{projectStore.currentProject.config.db.database}@{projectStore.currentProject.config.db.host}</span
        >
      </div>
      <div class="info-item">
        <span class="info-label">Ejecutado</span>
        <span class="info-value"
          >{projectStore.currentProject.started ? "Sí" : "No"}</span
        >
      </div>
    </div>
  </div>

  <div class="panel">
    <h4>Estado</h4>

    <div class="stats-grid">
      <div class="stat-card">
        <span class="stat-value">--</span>
        <span class="stat-label">Bounds analizados</span>
      </div>
      <div class="stat-card">
        <span class="stat-value">--</span>
        <span class="stat-label">Bounds restantes</span>
      </div>
      <div class="stat-card">
        <span class="stat-value">--</span>
        <span class="stat-label">Resultados</span>
      </div>
      <div class="stat-card">
        <span class="stat-value">--</span>
        <span class="stat-label">Teléfonos</span>
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
    margin-top: 20px;
  }

  .panel h4 {
    font-size: 12px;
    margin: 0 0 8px 0;
    text-transform: uppercase;
    letter-spacing: 1px;
    color: var(--primary);
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

  .hint {
    font-size: 12px;
    color: var(--text-muted, #8892b0);
    margin: 0;
  }
</style>
