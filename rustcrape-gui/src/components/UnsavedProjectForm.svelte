<script lang="ts">
  import { projectStore } from "../lib/stores/project.store.svelte";
  import NumberInput from "./NumberInput.svelte";
  import Panel from "./Panel.svelte";
  import Toggle from "./Toggle.svelte";
</script>

<div class="form-panels">
  <Panel title="Proyecto">
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

    <div class="field">
      <label for="search-query">Término de búsqueda</label>
      <input
        type="text"
        id="search-query"
        bind:value={projectStore.projectDraft.search_query}
      />
    </div>

    <div class="field-row">
      <NumberInput
        label="Zoom"
        id="zoom"
        min={1}
        max={20}
        bind:value={projectStore.projectDraft.zoom}
      />
    </div>
  </Panel>

  <Panel title="Base de datos">
    <div class="toggle-row">
      <Toggle bind:checked={projectStore.projectDraft.use_mysql} />
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
  </Panel>
</div>

<style>
  .form-panels {
    display: flex;
    flex-direction: column;
    gap: 20px;
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
