<script lang="ts">
  import type { CoincidenceColumn, CoincidenceRecord } from "../lib/types";
  import { resultsStore } from "../lib/stores/results.store.svelte";

  const MIN_COLUMN_WIDTH = 60;

  const columns: { key: CoincidenceColumn; label: string; width: number }[] = [
    { key: "id", label: "ID", width: 70 },
    { key: "name", label: "Nombre", width: 180 },
    { key: "web", label: "Web", width: 200 },
    { key: "email", label: "Email", width: 200 },
    { key: "tfno", label: "Teléfono", width: 130 },
    { key: "source_url", label: "URL origen", width: 220 },
    { key: "source", label: "Fuente", width: 110 },
    { key: "creado", label: "Creado", width: 130 },
    { key: "legal_name", label: "Razón social", width: 200 },
    { key: "tax_id", label: "CIF/NIF", width: 110 },
    { key: "legal_form", label: "Forma jurídica", width: 140 },
    { key: "sector", label: "Sector", width: 140 },
    { key: "incorporation_date", label: "Constitución", width: 120 },
    { key: "last_change_date", label: "Último cambio", width: 120 },
    { key: "corporate_purpose", label: "Objeto social", width: 260 },
    { key: "activity", label: "Actividad", width: 200 },
    { key: "cnae_activity", label: "CNAE", width: 90 },
    { key: "company_status", label: "Estado", width: 130 },
  ];

  let widths = $state<Record<CoincidenceColumn, number>>(
    Object.fromEntries(columns.map((col) => [col.key, col.width])) as Record<
      CoincidenceColumn,
      number
    >,
  );

  let resizing = $state<{
    key: CoincidenceColumn;
    startX: number;
    startWidth: number;
  } | null>(null);

  const totalWidth = $derived(
    columns.reduce((sum, col) => sum + widths[col.key], 0),
  );

  function cellValue(row: CoincidenceRecord, key: CoincidenceColumn): string {
    const value = row[key];
    return value == null ? "" : String(value);
  }

  function onSort(key: CoincidenceColumn): void {
    void resultsStore.setSort(key);
  }

  function onResizeMove(event: PointerEvent): void {
    if (!resizing) return;
    const delta = event.clientX - resizing.startX;
    widths[resizing.key] = Math.max(
      MIN_COLUMN_WIDTH,
      resizing.startWidth + delta,
    );
  }

  function onResizeEnd(): void {
    resizing = null;
    window.removeEventListener("pointermove", onResizeMove);
    window.removeEventListener("pointerup", onResizeEnd);
  }

  $effect(() => {
    return () => {
      window.removeEventListener("pointermove", onResizeMove);
      window.removeEventListener("pointerup", onResizeEnd);
    };
  });

  function onResizeStart(
    event: PointerEvent,
    key: CoincidenceColumn,
    th: HTMLTableCellElement,
  ): void {
    event.preventDefault();
    event.stopPropagation();
    resizing = { key, startX: event.clientX, startWidth: th.offsetWidth };
    window.addEventListener("pointermove", onResizeMove);
    window.addEventListener("pointerup", onResizeEnd);
  }

  function onPageSizeChange(event: Event): void {
    const value = Number((event.currentTarget as HTMLSelectElement).value);
    void resultsStore.setPageSize(value);
  }
</script>

<div class="results-view">
  <div class="toolbar">
    <span class="total">{resultsStore.total} resultados</span>
    <label class="page-size">
      Filas por página
      <select
        value={resultsStore.pageSize}
        onchange={onPageSizeChange}
      >
        {#each resultsStore.pageSizes as size (size)}
          <option value={size}>{size}</option>
        {/each}
      </select>
    </label>
  </div>

  {#if resultsStore.loading}
    <p class="state">Cargando…</p>
  {:else if resultsStore.error}
    <p class="state error">{resultsStore.error}</p>
  {:else if resultsStore.rows.length === 0}
    <p class="state">Sin resultados</p>
  {:else}
    <div class="table-wrap" class:resizing={resizing !== null}>
      <table style="width: {totalWidth}px">
        <colgroup>
          {#each columns as col (col.key)}
            <col style="width: {widths[col.key]}px" />
          {/each}
        </colgroup>
        <thead>
          <tr>
            {#each columns as col (col.key)}
              <th
                class:active={resultsStore.sortColumn === col.key}
                onclick={() => onSort(col.key)}
                onkeydown={(e) => e.key === "Enter" && onSort(col.key)}
              >
                <span class="th-content">
                  <span class="th-label">{col.label}</span>
                  <span class="sort-arrow">
                    {resultsStore.sortColumn === col.key
                      ? resultsStore.sortOrder === "asc"
                        ? "▲"
                        : "▼"
                      : ""}
                  </span>
                </span>
                <button
                  type="button"
                  class="resize-handle"
                  class:resizing={resizing?.key === col.key}
                  aria-label="Redimensionar columna"
                  onpointerdown={(e) =>
                    onResizeStart(
                      e,
                      col.key,
                      (e.currentTarget as HTMLElement)
                        .parentElement as HTMLTableCellElement,
                    )}
                  onclick={(e) => e.stopPropagation()}
                ></button>
              </th>
            {/each}
          </tr>
        </thead>
        <tbody>
          {#each resultsStore.rows as row (row.id)}
            <tr>
              {#each columns as col (col.key)}
                <td>{cellValue(row, col.key)}</td>
              {/each}
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

    <div class="pagination">
      <button
        class="page-btn"
        disabled={!resultsStore.canGoPrev}
        onclick={() => resultsStore.setPage(resultsStore.page - 1)}
      >
        ‹ Anterior
      </button>
      <span class="page-info"
        >Página {resultsStore.page} de {resultsStore.totalPages}</span
      >
      <button
        class="page-btn"
        disabled={!resultsStore.canGoNext}
        onclick={() => resultsStore.setPage(resultsStore.page + 1)}
      >
        Siguiente ›
      </button>
    </div>
  {/if}
</div>

<style>
  .results-view {
    display: flex;
    flex-direction: column;
    gap: 12px;
    flex: 1;
    min-height: 0;
  }

  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
  }

  .total {
    font-size: 13px;
    color: var(--text-muted, #8892b0);
  }

  .page-size {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--text-muted, #8892b0);
  }

  .page-size select {
    background: var(--bg, #1a1a2e);
    border: 1px solid var(--border, #2a3a5c);
    border-radius: 6px;
    color: var(--text, #e0e0e0);
    padding: 4px 8px;
    font-family: var(--font);
    font-size: 12px;
  }

  .state {
    padding: 32px;
    text-align: center;
    color: var(--text-muted, #8892b0);
    background: var(--card-bg, #1f2b47);
    border: 1px solid var(--border, #2a3a5c);
    border-radius: 8px;
  }

  .state.error {
    color: var(--error, #e74c3c);
  }

  .table-wrap {
    flex: 1;
    overflow: auto;
    border: 1px solid var(--border, #2a3a5c);
    border-radius: 8px;
    background: var(--card-bg, #1f2b47);
  }

  table {
    table-layout: fixed;
    border-collapse: collapse;
    font-size: 13px;
  }

  thead th {
    position: sticky;
    top: 0;
    z-index: 1;
    background: var(--sidebar-bg, #16213e);
    color: var(--text-muted, #8892b0);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    text-align: left;
    padding: 10px 12px;
    border-bottom: 1px solid var(--border, #2a3a5c);
    cursor: pointer;
    user-select: none;
    white-space: nowrap;
    overflow: hidden;
  }

  thead th:hover {
    color: var(--text, #e0e0e0);
  }

  thead th.active {
    color: var(--primary, #4f8cff);
  }

  .th-content {
    display: flex;
    align-items: center;
    min-width: 0;
  }

  .th-label {
    margin-right: 4px;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .sort-arrow {
    flex: none;
    font-size: 10px;
  }

  .resize-handle {
    position: absolute;
    top: 0;
    right: 0;
    width: 6px;
    height: 100%;
    padding: 0;
    border: none;
    cursor: col-resize;
    touch-action: none;
    background: transparent;
  }

  .resize-handle:hover,
  .resize-handle.resizing {
    background: var(--primary, #4f8cff);
  }

  tbody td {
    padding: 8px 12px;
    border-bottom: 1px solid var(--border, #2a3a5c);
    color: var(--text, #e0e0e0);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  tbody tr:nth-child(even) td {
    background: rgba(255, 255, 255, 0.025);
  }

  tbody tr:hover td {
    background: rgba(79, 140, 255, 0.1);
  }

  .table-wrap.resizing {
    cursor: col-resize;
    user-select: none;
  }

  .pagination {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 16px;
  }

  .page-btn {
    background: var(--card-bg, #1f2b47);
    border: 1px solid var(--border, #2a3a5c);
    border-radius: 6px;
    color: var(--text, #e0e0e0);
    padding: 6px 14px;
    cursor: pointer;
    transition: background 0.15s;
    font-family: var(--font);
    font-size: 13px;
  }

  .page-btn:hover:not(:disabled) {
    background: rgba(79, 140, 255, 0.15);
  }

  .page-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .page-info {
    font-size: 13px;
    color: var(--text-muted, #8892b0);
  }
</style>
