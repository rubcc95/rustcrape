<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { appStore } from "../lib/stores/app.store.svelte";
  import { projectStore } from "../lib/stores/project.store.svelte";

  let appWindow = $state(getCurrentWindow());
  let isMaximized = $state(false);

  let headerTitle = $derived.by(() => {
    const route = appStore.currentRoute;
    if (route === "settings") return "Rustcraper — Configuración global";
    if (route === "execution") return "Rustcraper — Ejecución";
    if (projectStore.isNewProject) return "Rustcraper — Nuevo proyecto";
    return `Rustcraper — ${projectStore.currentProject?.name ?? "Proyecto"}`;
  });

  function onMaximizeToggle(): void {
    appWindow.toggleMaximize();
  }

  function onMinimize(): void {
    appWindow.minimize();
  }

  function onSettings(): void {
    appStore.navigate("settings");
  }

  function onClose(): void {
    appWindow.close();
  }

  $effect(() => {
    appWindow.onResized(() => {
      appWindow.isMaximized().then((m) => {
        isMaximized = m;
      });
    });
  });

  let dragStartX = 0;
  let dragStartY = 0;

  function onHeaderMouseDown(e: MouseEvent): void {
    if (e.button !== 0) return;
    dragStartX = e.screenX;
    dragStartY = e.screenY;

    const onMouseMove = (e2: MouseEvent) => {
      if (
        Math.abs(e2.screenX - dragStartX) > 3 ||
        Math.abs(e2.screenY - dragStartY) > 3
      ) {
        cleanup();
        appWindow.startDragging();
      }
    };
    const onMouseUp = () => cleanup();
    const cleanup = () => {
      document.removeEventListener("mousemove", onMouseMove);
      document.removeEventListener("mouseup", onMouseUp);
    };
    document.addEventListener("mousemove", onMouseMove);
    document.addEventListener("mouseup", onMouseUp);
  }
</script>

<header>
  <div class="header-col"></div>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="header-col header-title"
    data-tauri-drag-region
    onmousedown={onHeaderMouseDown}
  >
    <h1>{headerTitle}</h1>
  </div>
  <div class="header-col header-controls">
    <div class="window-controls">
      <button class="win-btn win-settings" title="Configuración global" onclick={onSettings}>
        <svg viewBox="0 0 512 512" width="12" height="12">
          <path
            fill="currentColor"
            d="M487.4 315.7l-42.6-24.6c4.3-23.2 4.3-47 0-70.2l42.6-24.6c4.9-2.8 7.1-8.6 5.5-14-11.1-35.6-30-67.8-54.7-94.6-3.8-4.1-10-5.1-14.8-2.3L380.8 110c-17.9-15.4-38.5-27.3-60.8-35.1V25.8c0-5.6-3.9-10.5-9.4-11.7-36.7-8.2-74.3-7.8-109.2 0-5.5 1.2-9.4 6.1-9.4 11.7V75c-22.2 7.9-42.8 19.8-60.8 35.1L88.7 85.5c-4.9-2.8-11-1.9-14.8 2.3-24.7 26.7-43.6 58.9-54.7 94.6-1.7 5.4.6 11.2 5.5 14L67.3 221c-4.3 23.2-4.3 47 0 70.2l-42.6 24.6c-4.9 2.8-7.1 8.6-5.5 14 11.1 35.6 30 67.8 54.7 94.6 3.8 4.1 10 5.1 14.8 2.3l42.6-24.6c17.9 15.4 38.5 27.3 60.8 35.1v49.2c0 5.6 3.9 10.5 9.4 11.7 36.7 8.2 74.3 7.8 109.2 0 5.5-1.2 9.4-6.1 9.4-11.7v-49.2c22.2-7.9 42.8-19.8 60.8-35.1l42.6 24.6c4.9 2.8 11 1.9 14.8-2.3 24.7-26.7 43.6-58.9 54.7-94.6 1.5-5.5-.7-11.3-5.6-14.1zM256 336c-44.1 0-80-35.9-80-80s35.9-80 80-80 80 35.9 80 80-35.9 80-80 80z"
          />
        </svg>
      </button>
      <button class="win-btn win-minimize" title="Minimizar" onclick={onMinimize}>
        <svg viewBox="0 0 12 12"
          ><rect x="1" y="5.5" width="10" height="1" fill="currentColor"
        /></svg>
      </button>
      <button
        class="win-btn win-maximize"
        title={isMaximized ? "Restaurar" : "Maximizar"}
        onclick={onMaximizeToggle}
      >
        {#if isMaximized}
          <svg viewBox="0 0 12 12"
            ><rect
              x="2.5"
              y="0.5"
              width="9"
              height="9"
              fill="none"
              stroke="currentColor"
              stroke-width="1"
            /><rect
              x="0.5"
              y="2.5"
              width="9"
              height="9"
              fill="var(--card-bg)"
              stroke="currentColor"
              stroke-width="1"
            /></svg
          >
        {:else}
          <svg viewBox="0 0 12 12"
            ><rect
              x="1.5"
              y="1.5"
              width="9"
              height="9"
              fill="none"
              stroke="currentColor"
              stroke-width="1"
            /></svg
          >
        {/if}
      </button>
      <button class="win-btn win-close" title="Cerrar" onclick={onClose}>
        <svg viewBox="0 0 12 12"
          ><path
            d="M1 1l10 10M11 1L1 11"
            stroke="currentColor"
            stroke-width="1.2"
          /></svg
        >
      </button>
    </div>
  </div>
</header>

<style>
  header {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    align-items: center;
    height: 40px;
    min-height: 40px;
    border-bottom: 1px solid var(--border);
    background: var(--card-bg);
    user-select: none;
    flex-shrink: 0;
  }

  .header-col {
    height: 100%;
    display: flex;
    align-items: center;
  }

  .header-title {
    justify-content: center;
  }

  .header-controls {
    justify-content: flex-end;
  }

  header h1 {
    font-size: 14px;
    font-weight: 600;
    pointer-events: none;
  }

  .window-controls {
    display: flex;
    align-items: center;
    height: 100%;
  }

  .win-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 46px;
    height: 100%;
    border: none;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    padding: 0;
    border-radius: 0;
    transition: background 0.15s, color 0.15s;
  }

  .win-btn svg {
    width: 12px;
    height: 12px;
  }

  .win-btn:hover {
    background: rgba(255, 255, 255, 0.08);
    color: var(--text);
  }

  .win-btn:active {
    transform: none;
    background: rgba(255, 255, 255, 0.04);
  }

  .win-close:hover {
    background: #e81123;
    color: #fff;
  }

  .win-close:active {
    background: #bf0f1d;
    color: #fff;
  }
</style>
