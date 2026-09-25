<script lang="ts">
  import { projectStore } from "../lib/stores/project.store.svelte";
  import NumberInput from "./NumberInput.svelte";
  import Checkbox from "./Checkbox.svelte";
  import Panel from "./Panel.svelte";
  import type {
    CompanySize,
    IncorporationDate,
    LegalForm,
    LocationMode,
    Province,
  } from "../lib/types";
  import { PROVINCES } from "../lib/types";
  import { TOWNS_BY_PROVINCE } from "../lib/data/empresiteLocations";
  import CheckboxNumber from "./CheckboxNumber.svelte";
  import ExecutionConfig from "./ExecutionPanel.svelte";

  function townsForProvince(province: Province | null) {
    const slug = PROVINCES.find((p) => p.value === province)?.slug;
    return slug ? (TOWNS_BY_PROVINCE[slug] ?? []) : [];
  }

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

  function onProvinceChange(e: Event): void {
    const value = (e.currentTarget as HTMLSelectElement).value;
    const filters = projectStore.projectDraft.empresite_filters;
    filters.province = value === "" ? null : (value as Province);
    filters.locality_id = "";
  }

  function onLocalityChange(e: Event): void {
    projectStore.projectDraft.empresite_filters.locality_id = (
      e.currentTarget as HTMLSelectElement
    ).value;
  }

  function onLocationModeChange(e: Event): void {
    const value = (e.currentTarget as HTMLSelectElement)
      .value as LocationMode;
    const filters = projectStore.projectDraft.empresite_filters;
    filters.location_mode = value;
    if (value === "none") {
      filters.province = null;
      filters.locality_id = "";
    } else if (value === "province") {
      filters.locality_id = "";
    }
  }
</script>

<div class="panels">
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
  </Panel>

  <Panel title="Objetivos">
    <Checkbox
      bind:value={projectStore.projectDraft.enable_google_maps}
      label="Google Maps"
    />

    <Checkbox
      bind:value={projectStore.projectDraft.enable_empresite}
      label="Empresite (eleconomista)"
    />
  </Panel>

  <Panel title="Búsqueda">
    <div class="field">
      <label for="search-query">Término de búsqueda</label>
      <input
        type="text"
        id="search-query"
        bind:value={projectStore.projectDraft.search_query}
      />
    </div>

    <p class="hint">El término se usa para buscar en las webs seleccionadas.</p>
  </Panel>

  <Panel title="Base de datos">
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
  </Panel>

  <ExecutionConfig />
  
  {#if projectStore.projectDraft.enable_google_maps}
    <Panel title="Google Maps">
      <div class="field-row">
        <NumberInput
          label="Zoom"
          id="zoom"
          min={1}
          max={20}
          bind:value={projectStore.projectDraft.zoom}
        />
      </div>

      <div class="field-row">
        <CheckboxNumber
          label="Rate limit (por hora)"
          id="gm-rate-limit"
          min={1}
          bind:value={projectStore.executionConfig.gmaps.rate_limit}
        />
        <CheckboxNumber
          label="Iteraciones"
          id="gm-iterations"
          min={1}
          bind:value={projectStore.executionConfig.gmaps.iterations}
        />
      </div>

      <div class="field-row">
        <NumberInput
          label="Delay min (ms)"
          id="gm-delay-min"
          min={100}
          bind:value={projectStore.executionConfig.gmaps.delay_min}
        />
        <NumberInput
          label="Delay max (ms)"
          id="gm-delay-max"
          min={100}
          bind:value={projectStore.executionConfig.gmaps.delay_max}
        />
      </div>

      <div class="field-row">
        <NumberInput
          label="Stop threshold"
          id="gm-stop-threshold"
          min={1}
          bind:value={projectStore.executionConfig.gmaps.stop_threshold}
        />
        <div class="field toggle-field">
          <!-- svelte-ignore a11y_label_has_associated_control -->
          <label>Headless</label>
          <div
            class="check-track"
            class:checked={projectStore.executionConfig.gmaps.headless}
            role="checkbox"
            aria-checked={projectStore.executionConfig.gmaps.headless}
            aria-label="Headless Google Maps"
            tabindex="0"
            onclick={() =>
              (projectStore.executionConfig.gmaps.headless =
                !projectStore.executionConfig.gmaps.headless)}
            onkeydown={(e) =>
              e.key === "Enter" &&
              (projectStore.executionConfig.gmaps.headless =
                !projectStore.executionConfig.gmaps.headless)}
          >
            <svg class="check-mark" viewBox="0 0 24 24" width="18" height="18">
              <path
                d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z"
                fill="currentColor"
              />
            </svg>
          </div>
        </div>
      </div>
    </Panel>
  {/if}

  {#if projectStore.projectDraft.enable_empresite}
    <Panel title="Empresite">
      <div class="field-row">
        <CheckboxNumber
          label="Rate limit (por hora)"
          id="em-rate-limit"
          min={1}
          bind:value={projectStore.executionConfig.empresite.rate_limit}
        />
        <CheckboxNumber
          label="Iteraciones"
          id="em-iterations"
          min={1}
          bind:value={projectStore.executionConfig.empresite.iterations}
        />
      </div>

      <div class="field-row">
        <NumberInput
          label="Delay min (ms)"
          id="em-delay-min"
          min={100}
          bind:value={projectStore.executionConfig.empresite.delay_min}
        />
        <NumberInput
          label="Delay max (ms)"
          id="em-delay-max"
          min={100}
          bind:value={projectStore.executionConfig.empresite.delay_max}
        />
      </div>

      <div class="field-row">
        <div class="field toggle-field">
          <!-- svelte-ignore a11y_label_has_associated_control -->
          <label>Headless</label>
          <div
            class="check-track"
            class:checked={projectStore.executionConfig.empresite.headless}
            role="checkbox"
            aria-checked={projectStore.executionConfig.empresite.headless}
            aria-label="Headless Empresite"
            tabindex="0"
            onclick={() =>
              (projectStore.executionConfig.empresite.headless =
                !projectStore.executionConfig.empresite.headless)}
            onkeydown={(e) =>
              e.key === "Enter" &&
              (projectStore.executionConfig.empresite.headless =
                !projectStore.executionConfig.empresite.headless)}
          >
            <svg class="check-mark" viewBox="0 0 24 24" width="18" height="18">
              <path
                d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z"
                fill="currentColor"
              />
            </svg>
          </div>
        </div>
      </div>

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
              value={projectStore.projectDraft.empresite_filters.company_size ??
                ""}
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
              value={projectStore.projectDraft.empresite_filters
                .incorporation_date ?? ""}
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
            <option value="foreign_entity"
              >Entidad Extranjera no Residente</option
            >
          </select>
        </div>

        <div class="field">
          <label for="emp-location-mode">Ubicación</label>
          <select
            id="emp-location-mode"
            value={projectStore.projectDraft.empresite_filters.location_mode}
            onchange={onLocationModeChange}
          >
            <option value="none">Sin filtro</option>
            <option value="province">Provincia</option>
            <option value="locality">Localidad</option>
          </select>
        </div>

        {#if projectStore.projectDraft.empresite_filters.location_mode === "province"}
          <div class="field">
            <label for="emp-province">Provincia</label>
            <select
              id="emp-province"
              value={projectStore.projectDraft.empresite_filters.province ?? ""}
              onchange={onProvinceChange}
            >
              <option value="">Todas</option>
              {#each PROVINCES as province (province.value)}
                <option value={province.value}>{province.label}</option>
              {/each}
            </select>
          </div>
        {:else if projectStore.projectDraft.empresite_filters.location_mode === "locality"}
          <div class="field">
            <label for="emp-locality-province">Provincia</label>
            <select
              id="emp-locality-province"
              value={projectStore.projectDraft.empresite_filters.province ?? ""}
              onchange={onProvinceChange}
            >
              <option value="">Selecciona provincia</option>
              {#each PROVINCES as province (province.value)}
                <option value={province.value}>{province.label}</option>
              {/each}
            </select>
          </div>
          <div class="field">
            <label for="emp-locality-id">Localidad</label>
            {#if projectStore.projectDraft.empresite_filters.province}
              <select
                id="emp-locality-id"
                value={projectStore.projectDraft.empresite_filters.locality_id}
                onchange={onLocalityChange}
              >
                <option value="">Selecciona localidad</option>
                {#each townsForProvince(projectStore.projectDraft.empresite_filters.province) as town (town.id)}
                  <option value={town.id}>{town.name}</option>
                {/each}
              </select>
            {:else}
              <p class="hint">Selecciona una provincia primero.</p>
            {/if}
          </div>
        {/if}

        <Checkbox
          bind:value={
            projectStore.projectDraft.empresite_filters.employees_enabled
          }
          label="Nº de empleados"
        />
        {#if projectStore.projectDraft.empresite_filters.employees_enabled}
          <div class="field-row">
            <NumberInput
              label="Mínimo"
              id="emp-employees-min"
              min={0}
              max={100}
              bind:value={
                projectStore.projectDraft.empresite_filters.employees_min
              }
            />
            <NumberInput
              label="Máximo"
              id="emp-employees-max"
              min={0}
              max={100}
              bind:value={
                projectStore.projectDraft.empresite_filters.employees_max
              }
            />
          </div>
        {/if}
      </div>
    </Panel>
  {/if}
</div>

<style>
  .panels {
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

  .toggle-field label {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-muted, #8892b0);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .check-track {
    width: 34px;
    height: 34px;
    border: 1px solid var(--border, #2a3a5c);
    border-radius: 6px;
    background: var(--bg);
    cursor: pointer;
    transition:
      background 0.15s,
      border-color 0.15s;
    display: flex;
    align-items: center;
    justify-content: center;
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
    transition:
      opacity 0.15s,
      transform 0.15s;
  }

  .check-track.checked .check-mark {
    opacity: 1;
    transform: scale(1);
  }
</style>
