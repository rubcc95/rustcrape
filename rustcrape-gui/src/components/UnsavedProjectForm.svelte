<script lang="ts">
  import { projectStore } from "../lib/stores/project.store.svelte";
  import NumberInput from "./NumberInput.svelte";
</script>

<div class="panel">
  <h4>Proyecto</h4>

  <div class="field">
    <label for="project-name">Nombre del proyecto</label>
    <input
      type="text"
      id="project-name"
      required
      bind:value={projectStore.projectDraft.name}
      placeholder="Mi proyecto"
    />
  </div>
</div>

<div class="panel">
  <h4>Objetivos</h4>

  <div class="toggle-row">
    <div
      class="check-track"
      class:checked={projectStore.projectDraft.enable_google_maps}
      role="checkbox"
      aria-checked={projectStore.projectDraft.enable_google_maps}
      aria-label="Google Maps"
      tabindex="0"
      onclick={() => projectStore.projectDraft.enable_google_maps = !projectStore.projectDraft.enable_google_maps}
      onkeydown={(e) => e.key === 'Enter' && (projectStore.projectDraft.enable_google_maps = !projectStore.projectDraft.enable_google_maps)}
    >
      <svg class="check-mark" viewBox="0 0 24 24" width="18" height="18">
        <path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z" fill="currentColor" />
      </svg>
    </div>
    <span class="toggle-label">Google Maps</span>
  </div>

  <div class="toggle-row">
    <div
      class="check-track"
      class:checked={projectStore.projectDraft.enable_empresite}
      role="checkbox"
      aria-checked={projectStore.projectDraft.enable_empresite}
      aria-label="Empresite"
      tabindex="0"
      onclick={() => projectStore.projectDraft.enable_empresite = !projectStore.projectDraft.enable_empresite}
      onkeydown={(e) => e.key === 'Enter' && (projectStore.projectDraft.enable_empresite = !projectStore.projectDraft.enable_empresite)}
    >
      <svg class="check-mark" viewBox="0 0 24 24" width="18" height="18">
        <path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z" fill="currentColor" />
      </svg>
    </div>
    <span class="toggle-label">Empresite (eleconomista)</span>
  </div>
</div>

<div class="panel">
  <h4>Búsqueda</h4>

  <div class="field">
    <label for="search-query">Término de búsqueda</label>
    <input
      type="text"
      id="search-query"
      bind:value={projectStore.projectDraft.search_query}
    />
  </div>

  {#if projectStore.projectDraft.enable_google_maps}
    <div class="field-row">
      <NumberInput
        label="Zoom"
        id="zoom"
        min={1}
        max={20}
        bind:value={projectStore.projectDraft.zoom}
      />
    </div>
  {/if}

  <p class="hint">El término se usa para buscar en las webs seleccionadas.</p>
</div>

<div class="panel">
  <h4>Base de datos</h4>

  <div class="toggle-row">
    <div
      class="check-track"
      class:checked={projectStore.projectDraft.use_mysql}
      role="checkbox"
      aria-checked={projectStore.projectDraft.use_mysql}
      aria-label="Usar MySQL"
      tabindex="0"
      onclick={() => projectStore.projectDraft.use_mysql = !projectStore.projectDraft.use_mysql}
      onkeydown={(e) => e.key === 'Enter' && (projectStore.projectDraft.use_mysql = !projectStore.projectDraft.use_mysql)}
    >
      <svg class="check-mark" viewBox="0 0 24 24" width="18" height="18">
        <path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z" fill="currentColor" />
      </svg>
    </div>
    <span class="toggle-label">Usar MySQL (avanzado)</span>
  </div>

  {#if projectStore.projectDraft.use_mysql}
    <div class="field">
      <label for="db-host">Host</label>
      <input
        type="text"
        id="db-host"
        bind:value={projectStore.projectDraft.db_host}
      />
    </div>

    <div class="field-row">
      <NumberInput
        label="Puerto"
        id="db-port"
        min={1}
        max={65535}
        bind:value={projectStore.projectDraft.db_port}
      />
      <div class="field">
        <label for="db-database">Base de datos</label>
        <input
          type="text"
          id="db-database"
          bind:value={projectStore.projectDraft.db_database}
        />
      </div>
    </div>

    <div class="field-row">
      <div class="field">
        <label for="db-user">Usuario</label>
        <input
          type="text"
          id="db-user"
          bind:value={projectStore.projectDraft.db_user}
        />
      </div>
      <div class="field">
        <label for="db-password">Contraseña</label>
        <input
          type="password"
          id="db-password"
          bind:value={projectStore.projectDraft.db_password}
        />
      </div>
    </div>
  {:else}
    <p class="hint">Usando SQLite local — sin configuración necesaria.</p>
  {/if}
</div>

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

  label {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-muted, #8892b0);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .toggle-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .toggle-label {
    font-size: 13px;
    color: var(--text, #e0e0e0);
    text-transform: none;
    letter-spacing: normal;
    font-weight: 400;
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
    flex-shrink: 0;
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

  .hint {
    font-size: 13px;
    color: var(--text-muted, #8892b0);
    padding: 8px 0 4px 0;
  }

  input[type="text"],
  input[type="password"] {
    width: 100%;
    padding: 8px 12px;
    border: 1px solid var(--border, #2a3a5c);
    border-radius: 6px;
    background: var(--bg, #1a1a2e);
    color: var(--text, #e0e0e0);
    font-size: 13px;
    font-family: var(--font);
    outline: none;
    transition: border-color 0.15s;
  }

  input[type="text"]::placeholder,
  input[type="password"]::placeholder {
    color: var(--text-muted, #8892b0);
    font-style: italic;
    opacity: 0.6;
  }

  input[type="text"]:focus,
  input[type="password"]:focus {
    border-color: var(--primary, #4f8cff);
  }
</style>
