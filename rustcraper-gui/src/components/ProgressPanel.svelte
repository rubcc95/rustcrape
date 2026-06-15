<script lang="ts">
  import type { IterationStats } from "../lib/types";

  let { stats }: { stats: IterationStats } = $props();
</script>

<div class="panel">
  <h4>Progreso</h4>
  <div class="stats-grid">
    <div class="stat-card">
      <span class="stat-value">{stats.bounds_processed}</span>
      <span class="stat-label">Bounds analizados</span>
    </div>
    <div class="stat-card">
      <span class="stat-value">{stats.bounds_remaining}</span>
      <span class="stat-label">Bounds restantes</span>
    </div>
    <div class="stat-card">
      <span class="stat-value">{stats.results_found}</span>
      <span class="stat-label">Resultados</span>
    </div>
    <div class="stat-card">
      <span class="stat-value">{stats.phones_found}</span>
      <span class="stat-label">Teléfonos</span>
    </div>
  </div>
</div>

<style>
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
    color: var(--progreso, #8892b0);
  }

  .stats-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }

  .stat-card {
    background: var(--bg, #1a1a2e);
    border: 1px solid var(--border, #2a3a5c);
    border-radius: 8px;
    padding: 14px;
    text-align: center;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .stat-value {
    font-size: 22px;
    font-weight: 700;
    color: var(--primary, #4f8cff);
  }

  .stat-label {
    font-size: 10px;
    color: var(--text-muted, #8892b0);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
</style>
