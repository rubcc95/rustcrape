<script lang="ts">
  import WindowControls from "./components/WindowControls.svelte";
  import Sidebar from "./components/Sidebar.svelte";
  import SubHeader from "./components/SubHeader.svelte";
  import AnnouncementModal from "./components/AnnouncementModal.svelte";
  import SettingsView from "./views/SettingsView.svelte";
  import ProjectView from "./views/ProjectView.svelte";
  import ExecutionView from "./views/ExecutionView.svelte";
  import ResultsView from "./views/ResultsView.svelte";
  import { appStore } from "./lib/stores/app.store.svelte";
  import { projectStore } from "./lib/stores/project.store.svelte";
  import { executionStore } from "./lib/stores/execution.store.svelte";
  import { loadGlobalSettings, checkAnnouncement } from "./lib/tauri";

  $effect(() => {
    projectStore.loadProjects();
    loadGlobalSettings().then((s) => {
      appStore.updateGlobalSettings(s);
    });
    checkAnnouncement().then((a) => {
      appStore.announcement = a;
    });
    const cleanup = executionStore.setupExecutionListeners();
    return () => cleanup();
  });
</script>

<WindowControls />
<AnnouncementModal />

<div class="resize-handle top" data-edge="t"></div>
<div class="resize-handle bottom" data-edge="b"></div>
<div class="resize-handle left" data-edge="l"></div>
<div class="resize-handle right" data-edge="r"></div>
<div class="resize-handle top-left" data-edge="tl"></div>
<div class="resize-handle top-right" data-edge="tr"></div>
<div class="resize-handle bottom-left" data-edge="bl"></div>
<div class="resize-handle bottom-right" data-edge="br"></div>

{#key appStore.currentRoute}
  {#if appStore.currentRoute === "settings"}
    <div class="layout">
      <Sidebar />
      <main class="main-content">
        <SubHeader />
        <div class="scroll-area">
          <SettingsView />
        </div>
      </main>
    </div>
  {:else if appStore.currentRoute === "project"}
    <div class="layout">
      <Sidebar />
      <main class="main-content">
        <SubHeader />
        <div class="scroll-area">
          <ProjectView />
        </div>
      </main>
    </div>
  {:else if appStore.currentRoute === "execution"}
    <div class="execution-layout">
      <SubHeader />
      <div class="scroll-area">
        <ExecutionView />
      </div>
    </div>
  {:else if appStore.currentRoute === "results"}
    <div class="execution-layout">
      <SubHeader />
      <div class="scroll-area">
        <ResultsView />
      </div>
    </div>
  {/if}
{/key}

<style>
  :global(html, body) {
    height: 100%;
    margin: 0;
    padding: 0;
    overflow: hidden;
  }

  :global(#app) {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--bg);
    color: var(--text);
    font-family: var(--font);
    font-size: 14px;
  }

  .layout {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .main-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    padding: 20px 0 0 24px;
    min-height: 0;
    overflow: hidden;
  }

  .scroll-area {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    min-height: 0;
    padding-bottom: 20px;
  }

  .main-content > .scroll-area {
    padding-right: 24px;
  }

  .scroll-area > :global(.project-view),
  .scroll-area > :global(.settings) {
    margin: auto;
  }

  .execution-layout {
    flex: 1;
    display: flex;
    flex-direction: column;
    padding: 20px 24px 0 24px;
    overflow: hidden;
  }

  .resize-handle {
    position: fixed;
    z-index: 99999;
  }

  .resize-handle.top {
    top: 0;
    left: 5px;
    right: 5px;
    height: 4px;
    cursor: n-resize;
  }

  .resize-handle.bottom {
    bottom: 0;
    left: 5px;
    right: 5px;
    height: 4px;
    cursor: s-resize;
  }

  .resize-handle.left {
    left: 0;
    top: 5px;
    bottom: 5px;
    width: 4px;
    cursor: w-resize;
  }

  .resize-handle.right {
    right: 0;
    top: 5px;
    bottom: 5px;
    width: 4px;
    cursor: e-resize;
  }

  .resize-handle.top-left {
    top: 0;
    left: 0;
    width: 9px;
    height: 9px;
    cursor: nw-resize;
  }

  .resize-handle.top-right {
    top: 0;
    right: 0;
    width: 9px;
    height: 9px;
    cursor: ne-resize;
  }

  .resize-handle.bottom-left {
    bottom: 0;
    left: 0;
    width: 9px;
    height: 9px;
    cursor: sw-resize;
  }

  .resize-handle.bottom-right {
    bottom: 0;
    right: 0;
    width: 9px;
    height: 9px;
    cursor: se-resize;
  }
</style>
