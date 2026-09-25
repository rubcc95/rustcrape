<script lang="ts">
  import type { Town } from "../lib/data/empresiteLocations";

  let {
    province,
    towns,
    selected,
    onchange,
  }: {
    province: string;
    towns: Town[];
    selected: string[];
    onchange: (ids: string[]) => void;
  } = $props();

  let query = $state("");
  let open = $state(false);
  let active = $state(0);
  let root = $state<HTMLDivElement | null>(null);
  let list = $state<HTMLUListElement | null>(null);

  const matches = $derived.by(() => {
    const q = query.trim().toLowerCase();
    return towns.filter((t) => q === "" || t.name.toLowerCase().includes(q));
  });

  const byId = $derived(new Map(towns.map((t) => [t.id, t.name])));

  // Al cambiar la lista de coincidencias, la entrada destacada vuelve arriba.
  $effect(() => {
    matches;
    active = 0;
  });

  // Mantiene visible la entrada destacada dentro del dropdown.
  $effect(() => {
    active;
    open;
    list
      ?.querySelectorAll(".dropdown-row")
      [active]?.scrollIntoView({ block: "nearest" });
  });

  function toggle(id: string): void {
    if (selected.includes(id)) {
      onchange(selected.filter((s) => s !== id));
    } else {
      onchange([...selected, id]);
    }
    // El dropdown se mantiene abierto para encadenar selecciones.
  }

  function toggleActive(): void {
    const match = matches[active];
    if (match) toggle(match.id);
  }

  function remove(id: string): void {
    onchange(selected.filter((s) => s !== id));
  }

  function closeIfOutside(e: FocusEvent): void {
    const next = e.relatedTarget as Node | null;
    if (!root || (next && root.contains(next))) return;
    open = false;
  }

  function onKeydown(e: KeyboardEvent): void {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      open = true;
      if (matches.length > 0) active = (active + 1) % matches.length;
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      open = true;
      if (matches.length > 0) active = (active - 1 + matches.length) % matches.length;
    } else if (e.key === "Enter") {
      e.preventDefault();
      toggleActive();
    } else if (e.key === "Escape") {
      open = false;
    }
  }
</script>

<div class="locality-picker" bind:this={root} onfocusout={closeIfOutside}>
  <div class="location-search">
    <input
      type="text"
      id="emp-town-search-{province}"
      bind:value={query}
      oninput={() => (open = true)}
      onfocus={() => (open = true)}
      onkeydown={onKeydown}
      placeholder="Buscar localidad..."
      autocomplete="off"
    />
    {#if open && matches.length > 0}
      <ul class="dropdown" bind:this={list}>
        {#each matches as town, i (town.id)}
          <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <li
            class="dropdown-row"
            class:active={i === active}
            onmousedown={(e) => e.preventDefault()}
            onmouseenter={() => (active = i)}
            onclick={() => toggle(town.id)}
          >
            <input
              type="checkbox"
              checked={selected.includes(town.id)}
              tabindex="-1"
              readonly
            />
            {town.name}
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  {#if selected.length > 0}
    <div class="chips">
      {#each selected as id (id)}
        <span class="chip">
          {byId.get(id) ?? id}
          <button
            type="button"
            class="chip-remove"
            aria-label="Quitar {byId.get(id) ?? id}"
            onclick={() => remove(id)}
          >
            ×
          </button>
        </span>
      {/each}
    </div>
  {:else}
    <p class="hint">Sin localidades: se buscará en toda la provincia.</p>
  {/if}
</div>

<style>
  .locality-picker {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .location-search {
    position: relative;
  }

  input[type="text"] {
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

  .dropdown-row:hover,
  .dropdown-row.active {
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
</style>
