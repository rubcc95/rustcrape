<script lang="ts">
  import type { SavedConfig } from "../lib/types";
  import { appStore } from "../lib/stores/app.store.svelte";
  import { projectStore } from "../lib/stores/project.store.svelte";

  function onNewProject(): void {
    projectStore.resetToNew();
    if (appStore.currentRoute !== "project") appStore.navigate("project");
  }

  function onSelectProject(p: SavedConfig): void {
    if (appStore.currentRoute !== "project") appStore.navigate("project");
    projectStore.selectProject(p);
  }
</script>

<aside id="sidebar">
  <div class="sidebar-header">
    <h2>Proyectos</h2>
    <button class="add-btn" onclick={onNewProject}>+</button>
  </div>

  <div class="project-list">
    {#each projectStore.projects as project (project.id)}
      <div
        class="project-item"
        class:selected={projectStore.currentProject?.id === project.id}
        onclick={() => onSelectProject(project)}
        role="button"
        tabindex="0"
        onkeydown={(e) => e.key === "Enter" && onSelectProject(project)}
      >
        <span class="project-name">{project.name}</span>
        <span class="project-date"
          >{project.id.split("-").slice(-2).join("-")}</span
        >
      </div>
    {/each}

    {#if projectStore.projects.length === 0}
      <p class="empty-state">Ningún proyecto guardado</p>
    {/if}
  </div>
</aside>

<style>
  #sidebar {
    width: var(--sidebar-width, 260px);
    min-width: var(--sidebar-width, 260px);
    background: var(--sidebar-bg, #16213e);
    border-right: 1px solid var(--border, #2a3a5c);
    display: flex;
    flex-direction: column;
    padding: 16px;
    gap: 8px;
  }

  .sidebar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .sidebar-header h2 {
    font-size: 13px;
    text-transform: uppercase;
    letter-spacing: 1px;
    color: var(--text-muted, #8892b0);
    margin: 0;
  }

  .add-btn {
    width: 22px;
    height: 22px;
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    border: none;
    background: transparent;
    color: var(--text-muted, #8892b0);
    cursor: pointer;
    font-size: 16px;
    font-weight: 600;
    line-height: 1;
    transition: background 0.15s, color 0.15s;
  }

  .add-btn:hover {
    background: rgba(79, 140, 255, 0.15);
    color: var(--primary, #4f8cff);
  }

  .project-list {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .project-item {
    padding: 10px 12px;
    border-radius: 6px;
    cursor: pointer;
    transition: background 0.15s;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .project-item:hover {
    background: rgba(79, 140, 255, 0.1);
  }

  .project-item.selected {
    background: rgba(79, 140, 255, 0.2);
    border-left: 3px solid var(--primary, #4f8cff);
  }

  .project-name {
    font-weight: 500;
    font-size: 13px;
  }

  .project-date {
    font-size: 11px;
    color: var(--text-muted, #8892b0);
  }

  .empty-state {
    font-size: 12px;
    color: var(--text-muted, #8892b0);
    text-align: center;
    padding: 20px 0;
  }

</style>
