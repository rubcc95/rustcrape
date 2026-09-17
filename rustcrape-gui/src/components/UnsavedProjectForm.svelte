<script lang="ts">
  import { projectStore } from "../lib/stores/project.store.svelte";
  import NumberInput from "./NumberInput.svelte";
  import Checkbox from "./Checkbox.svelte";
  import type { CompanySize, IncorporationDate, LegalForm } from "../lib/types";

  function onCompanySizeChange(e: Event): void {
    const value = (e.currentTarget as HTMLSelectElement).value;
    projectStore.projectDraft.empresite_filters.company_size =
      value === "" ? null : (value as CompanySize);
  }

  function onIncorporationDateChange(e: Event): void {
    const value = (e.currentTarget as HTMLSelectElement).value;
    projectStore.projectDraft.empresite_filters.incorporation_date =
      value === "" ? null : (value as IncorporationDate);
  }

  function onLegalFormChange(e: Event): void {
    const value = (e.currentTarget as HTMLSelectElement).value;
    projectStore.projectDraft.empresite_filters.legal_form =
      value === "" ? null : (value as LegalForm);
  }
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

  <Checkbox
    bind:value={projectStore.projectDraft.enable_google_maps}
    label="Google Maps"
  />

  <Checkbox
    bind:value={projectStore.projectDraft.enable_empresite}
    label="Empresite (eleconomista)"
  />
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

  {#if projectStore.projectDraft.enable_empresite}
    <div class="filter-group">
      <span class="filter-title">Filtros del listado (Empresite)</span>

      <Checkbox
        bind:value={projectStore.projectDraft.empresite_filters.web}
        label="Web"
      />
      <Checkbox
        bind:value={projectStore.projectDraft.empresite_filters.phone}
        label="Teléfono"
      />
      <Checkbox
        bind:value={projectStore.projectDraft.empresite_filters.email}
        label="Email"
      />
      <Checkbox
        bind:value={projectStore.projectDraft.empresite_filters.location}
        label="Ubicación"
      />
      <Checkbox
        bind:value={projectStore.projectDraft.empresite_filters.branch}
        label="Sucursal"
      />

      <div class="field-row">
        <div class="field">
          <label for="emp-company-size">Tamaño de empresa</label>
          <select
            id="emp-company-size"
            value={projectStore.projectDraft.empresite_filters.company_size ?? ""}
            onchange={onCompanySizeChange}
          >
            <option value="">Cualquiera</option>
            <option value="small">Pequeña empresa</option>
            <option value="medium">Mediana empresa</option>
            <option value="large">Gran empresa</option>
            <option value="corporate">Corporativa</option>
          </select>
        </div>
        <div class="field">
          <label for="emp-incorporation-date">Fecha de creación</label>
          <select
            id="emp-incorporation-date"
            value={projectStore.projectDraft.empresite_filters.incorporation_date ??
              ""}
            onchange={onIncorporationDateChange}
          >
            <option value="">Cualquiera</option>
            <option value="last_month">Último mes</option>
            <option value="last_three_months">Últimos 3 meses</option>
            <option value="last_year">Último año</option>
            <option value="more_than_a_year">Más de un año</option>
          </select>
        </div>
      </div>

      <div class="field">
        <label for="emp-legal-form">Forma jurídica</label>
        <select
          id="emp-legal-form"
          value={projectStore.projectDraft.empresite_filters.legal_form ?? ""}
          onchange={onLegalFormChange}
        >
          <option value="">Cualquiera</option>
          <option value="limited_liability_company">Sociedad Limitada</option>
          <option value="community_of_property">Comunidad de Bienes</option>
          <option value="civil_partnership">Sociedad Civil</option>
          <option value="public_limited_company">Sociedad Anónima</option>
          <option value="temporary_joint_venture"
            >Unión Temporal de Empresas</option
          >
          <option value="cooperative">Cooperativa</option>
          <option value="public_body">Organismo Público</option>
          <option value="local_corporation">Corporación Local</option>
          <option value="public_administration"
            >Admón. del Estado y CCAA</option
          >
          <option value="foreign_entity">Entidad Extranjera no Residente</option
          >
        </select>
      </div>

      <Checkbox
        bind:value={projectStore.projectDraft.empresite_filters.employees_enabled}
        label="Nº de empleados"
      />
      {#if projectStore.projectDraft.empresite_filters.employees_enabled}
        <div class="field-row">
          <NumberInput
            label="Mínimo"
            id="emp-employees-min"
            min={0}
            max={100}
            bind:value={projectStore.projectDraft.empresite_filters.employees_min}
          />
          <NumberInput
            label="Máximo"
            id="emp-employees-max"
            min={0}
            max={100}
            bind:value={projectStore.projectDraft.empresite_filters.employees_max}
          />
        </div>
      {/if}
    </div>
  {/if}

  <p class="hint">El término se usa para buscar en las webs seleccionadas.</p>
</div>

<div class="panel">
  <h4>Base de datos</h4>

  <Checkbox
    bind:value={projectStore.projectDraft.use_mysql}
    label="Usar MySQL (avanzado)"
  />

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

  .field select {
    width: 100%;
    padding: 8px 12px;
    border: 1px solid var(--border, #2a3a5c);
    border-radius: 6px;
    background: var(--bg, #1a1a2e);
    color: var(--text, #e0e0e0);
    font-size: 13px;
    font-family: var(--font);
    outline: none;
    cursor: pointer;
  }

  .field select:focus {
    border-color: var(--primary, #4f8cff);
  }

  .filter-group {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding-top: 10px;
    border-top: 1px solid var(--border, #2a3a5c);
  }

  .filter-title {
    font-size: 11px;
    color: var(--text-muted, #8892b0);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  label {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-muted, #8892b0);
    text-transform: uppercase;
    letter-spacing: 0.5px;
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
