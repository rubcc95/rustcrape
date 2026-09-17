<script lang="ts">
  import { projectStore } from "../lib/stores/project.store.svelte";
  import ExecutionConfig from "../components/ExecutionConfig.svelte";
  import UnsavedProjectForm from "../components/UnsavedProjectForm.svelte";
  import SavedProjectForm from "../components/SavedProjectForm.svelte";
</script>

<div class="project-view">
  <div class="project-section">
    {#if projectStore.isNewProject}
      <UnsavedProjectForm />
    {:else}
      <SavedProjectForm />
    {/if}
  </div>
   <ExecutionConfig />
</div>

<style>
  .project-view {
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 20px;
      width: 80%;
  }
  @media (min-width: 1300px) {
    .project-view {
      width: 840px;
    }
  }


</style>
