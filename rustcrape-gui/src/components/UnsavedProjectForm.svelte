<script lang="ts">
  import { projectStore } from "../lib/stores/project.store.svelte";
  import NumberInput from "./NumberInput.svelte";
  import Checkbox from "./Checkbox.svelte";
  import Panel from "./Panel.svelte";
  import type {
    CompanySize,
    IncorporationDate,
    LegalForm,
    Province,
  } from "../lib/types";
  import { PROVINCES } from "../lib/types";
  import { TOWNS_BY_PROVINCE } from "../lib/data/empresiteLocations";
  import CheckboxNumber from "./CheckboxNumber.svelte";
  import LocalityPicker from "./LocalityPicker.svelte";
  import ExecutionConfig from "./ExecutionPanel.svelte";

  const filters = $derived(projectStore.projectDraft.empresite_filters);

  let provinceQuery = $state("");
  let provinceOpen = $state(false);
  let provinceRoot = $state<HTMLDivElement | null>(null);

  const selectedProvinces = $derived(filters.location_provinces);

  const provinceMatches = $derived.by(() => {
    const q = provinceQuery.trim().toLowerCase();
    return PROVINCES.filter(
      (p) => q === "" || p.label.toLowerCase().includes(q)
    ).slice(0, 8);
  });

  function provinceLabel(province: Province): string {
    return PROVINCES.find((p) => p.value === province)?.label ?? province;
  }

  function townsForProvince(province: Province) {
    const slug = PROVINCES.find((p) => p.value === province)?.slug;
    return slug ? (TOWNS_BY_PROVINCE[slug] ?? []) : [];
  }

  function addProvince(province: Province): void {
    if (!filters.location_provinces.includes(province)) {
      filters.location_provinces = [...filters.location_provinces, province];
    }
    provinceQuery = "";
    provinceOpen = false;
  }

  function toggleProvince(province: Province): void {
    if (filters.location_provinces.includes(province)) {
      removeProvince(province);
    } else {
      filters.location_provinces = [...filters.location_provinces, province];
    }
    // Con ratón el dropdown se mantiene abierto para encadenar selecciones.
    provinceQuery = "";
  }

  function removeProvince(province: Province): void {
    filters.location_provinces = filters.location_provinces.filter(
      (p) => p !== province
    );
    const next = { ...filters.location_localities };
    delete next[province];
    filters.location_localities = next;
  }

  function closeProvincesIfOutside(e: FocusEvent): void {
    const next = e.relatedTarget as Node | null;
    if (!provinceRoot || (next && provinceRoot.contains(next))) return;
    provinceOpen = false;
  }

  function onProvinceKeydown(e: KeyboardEvent): void {
    if (e.key === "Enter" && provinceMatches.length > 0) {
      e.preventDefault();
      addProvince(provinceMatches[0].value);
    } else if (e.key === "Escape") {
      provinceOpen = false;
    }
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

        <div class="field location-search" bind:this={provinceRoot} onfocusout={closeProvincesIfOutside}>
          <label for="emp-province-search">Provincia</label>
          <input
            type="text"
            id="emp-province-search"
            bind:value={provinceQuery}
            oninput={() => (provinceOpen = true)}
            onfocus={() => (provinceOpen = true)}
            onkeydown={onProvinceKeydown}
            placeholder="Buscar provincia..."
            autocomplete="off"
          />
          {#if provinceOpen && provinceMatches.length > 0}
            <ul class="dropdown">
              {#each provinceMatches as province (province.value)}
                <li>
                  <label class="dropdown-row">
                    <input
                      type="checkbox"
                      checked={selectedProvinces.includes(province.value)}
                      onmousedown={(e) => e.preventDefault()}
                      onchange={() => toggleProvince(province.value)}
                    />
                    {province.label}
                  </label>
                </li>
              {/each}
            </ul>
          {/if}
        </div>

        {#if selectedProvinces.length > 0}
          <div class="chips">
            {#each selectedProvinces as province (province)}
              <span class="chip">
                {provinceLabel(province)}
                <button
                  type="button"
                  class="chip-remove"
                  aria-label="Quitar {provinceLabel(province)}"
                  onclick={() => removeProvince(province)}
                >
                  ×
                </button>
              </span>
            {/each}
          </div>
        {/if}

        {#each selectedProvinces as province (province)}
          {@const towns = townsForProvince(province)}
          <div class="locality-block">
            <label for="emp-town-search-{province}">
              Localidades de {provinceLabel(province)}
            </label>
            {#if towns.length > 0}
              <LocalityPicker
                province={province}
                towns={towns}
                selected={filters.location_localities[province] ?? []}
                onchange={(ids) => {
                  filters.location_localities = {
                    ...filters.location_localities,
                    [province]: ids,
                  };
                }}
              />
            {:else}
              <p class="hint">Sin localidades: se buscará en toda la provincia.</p>
            {/if}
          </div>
        {/each}

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

  .location-search {
    position: relative;
  }

  .location-search input {
    width: 100%;
  }

  .dropdown {
    position: absolute;
    z-index: 30;
    top: calc(100% + 2px);
    left: 0;
    right: 0;
    margin: 0;
    padding: 0.25rem 0;
    list-style: none;
    background: var(--bg);
    border: 1px solid var(--border, #2a3a5c);
    border-radius: 6px;
    box-shadow: 0 8px 20px rgba(0, 0, 0, 0.35);
    max-height: 240px;
    overflow-y: auto;
  }

  .dropdown-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.45rem 0.7rem;
    cursor: pointer;
  }

  .dropdown-row:hover {
    background: var(--bg-hover, rgba(127, 127, 127, 0.15));
  }

  .dropdown-row input {
    cursor: pointer;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.2rem 0.35rem 0.2rem 0.6rem;
    border-radius: 999px;
    background: var(--bg-hover, rgba(127, 127, 127, 0.15));
    border: 1px solid var(--border, #2a3a5c);
    font-size: 0.85rem;
  }

  .chip-remove {
    background: none;
    border: none;
    color: inherit;
    cursor: pointer;
    font-size: 1rem;
    line-height: 1;
    padding: 0 0.15rem;
    opacity: 0.7;
  }

  .chip-remove:hover {
    opacity: 1;
  }

  .locality-block {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    padding: 0.75rem;
    border: 1px solid var(--border, #2a3a5c);
    border-radius: 6px;
  }
</style>
