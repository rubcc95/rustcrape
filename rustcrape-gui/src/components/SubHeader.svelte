<script lang="ts">
  import { appStore } from "../lib/stores/app.store.svelte";
  import { projectStore } from "../lib/stores/project.store.svelte";
  import { executionStore } from "../lib/stores/execution.store.svelte";
  import { resultsStore } from "../lib/stores/results.store.svelte";

  let showDeleteConfirm = $state(false);
  let statusMessage = $state("");

  async function onSave(): Promise<void> {
    const { saved } = await projectStore.saveCurrentProject();
    if (saved) {
      statusMessage = `Proyecto "${saved.name}" guardado`;
    } else {
      statusMessage = "Error: El nombre del proyecto es obligatorio";
    }
    setTimeout(() => (statusMessage = ""), 3000);
  }

  async function onRun(): Promise<void> {
    if (!projectStore.projectDraft.name.trim()) {
      statusMessage = "El nombre del proyecto es obligatorio";
      setTimeout(() => (statusMessage = ""), 3000);
      return;
    }
    await projectStore.saveCurrentProject();
    const config = projectStore.buildConfig();
    await executionStore.startExecution(config);
  }

  async function onDelete(): Promise<void> {
    if (!projectStore.currentProject) return;
    const ok = await projectStore.removeProject(projectStore.currentProject.id);
    if (ok) {
      showDeleteConfirm = false;
      statusMessage = "Proyecto eliminado";
      setTimeout(() => (statusMessage = ""), 3000);
    }
  }

  async function onPause(): Promise<void> {
    await executionStore.cancelExecution();
  }

  function onCancel(): void {
    executionStore.goBack();
  }

  async function onResume(): Promise<void> {
    await executionStore.resumeExecution();
  }

  function onReturn(): void {
    executionStore.returnToProject();
  }

  function onResults(): void {
    void resultsStore.openForCurrentProject();
  }

  function onBackToProject(): void {
    appStore.navigate("project");
  }
</script>

<div class="sub-header">
  {#if appStore.currentRoute === "project"}
    <div class="sub-header-left">
      <button class="btn-run" onclick={onRun}>▶ Ejecutar</button>
      {#if !projectStore.isNewProject}
        <button class="btn-primary" onclick={onSave}>Guardar</button>
        {#if !showDeleteConfirm}
          <button class="btn-danger" onclick={() => (showDeleteConfirm = true)}>Eliminar</button>
        {:else}
          <div class="confirm-inline">
            <span class="confirm-text">¿Eliminar?</span>
            <button class="btn-danger" onclick={onDelete}>Sí, eliminar</button>
            <button class="btn-small" onclick={() => (showDeleteConfirm = false)}>No</button>
          </div>
        {/if}
        <button class="btn-back" onclick={onResults}>Resultados</button>
      {/if}
    </div>

  {:else if appStore.currentRoute === "execution"}
    <div class="sub-header-left">
      {#if executionStore.isRunning && executionStore.isCancelling}
        <button class="btn-cancelling" disabled>Pausando...</button>
        <button class="btn-back" onclick={onReturn}>Volver</button>
      {:else if executionStore.isRunning}
        <button class="btn-pause" onclick={onPause}>Pausar</button>
        <button class="btn-danger" onclick={onCancel}>Guardar y salir</button>
      {:else}
        <button class="btn-run" onclick={onResume}>Reanudar</button>
        <button class="btn-back" onclick={onReturn}>Volver</button>
      {/if}
    </div>
  {:else if appStore.currentRoute === "results"}
    <div class="sub-header-left">
      <button class="btn-back" onclick={onBackToProject}>← Volver</button>
    </div>
  {/if}

  {#if statusMessage}
    <span class="status-msg">{statusMessage}</span>
  {/if}
</div>

<style>
  .sub-header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding-bottom: 16px;
    flex-wrap: wrap;
  }

  .sub-header-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .sub-header button:not(.btn-small) {
    min-width: 130px;
  }

  .btn-back {
    border: 1px solid var(--border, #2a3a5c);
    border-radius: 6px;
    background: transparent;
    color: var(--text, #e0e0e0);
    cursor: pointer;
    transition: background 0.15s;
  }

  .btn-back:hover {
    background: var(--card-bg, #1f2b47);
  }

  .btn-pause {
    background: var(--warn, #f39c12);
    color: #fff;
    cursor: pointer;
    transition: background 0.15s;
  }

  .btn-pause:hover {
    background: #e67e22;
  }

  .btn-cancelling {
    border-radius: 6px;
    opacity: 0.6;
    cursor: not-allowed;
    background: var(--border, #2a3a5c);
    color: var(--text-muted, #8892b0);
    border: 1px solid var(--border, #2a3a5c);
    font-size: 13px;
    font-family: var(--font);
  }

  .confirm-inline {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 10px;
    background: rgba(231, 76, 60, 0.1);
    border: 1px solid var(--danger, #e74c3c);
    border-radius: 6px;
  }

  .confirm-text {
    font-size: 13px;
    color: var(--danger, #e74c3c);
  }

  .status-msg {
    padding: 4px 12px;
    background: var(--card-bg, #1f2b47);
    border: 1px solid var(--border, #2a3a5c);
    border-radius: 6px;
    font-size: 12px;
    color: var(--text, #e0e0e0);
  }
</style>
