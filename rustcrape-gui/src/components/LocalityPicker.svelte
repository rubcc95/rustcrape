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

  const matches = $derived.by(() => {
    const q = query.trim().toLowerCase();
    return towns
      .filter((t) => !selected.includes(t.id) && (q === "" || t.name.toLowerCase().includes(q)))
      .slice(0, 8);
  });

  const byId = $derived(new Map(towns.map((t) => [t.id, t.name])));

  function add(id: string): void {
    if (!selected.includes(id)) onchange([...selected, id]);
    query = "";
    open = false;
  }

  function remove(id: string): void {
    onchange(selected.filter((s) => s !== id));
  }

  function onKeydown(e: KeyboardEvent): void {
    if (e.key === "Enter" && matches.length > 0) {
      e.preventDefault();
      add(matches[0].id);
    } else if (e.key === "Escape") {
      open = false;
    }
  }
</script>

<div class="locality-picker">
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
      <ul class="dropdown">
        {#each matches as town (town.id)}
          <li>
            <button
              type="button"
              onmousedown={(e) => {
                e.preventDefault();
                add(town.id);
              }}
            >
              {town.name}
            </button>
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

  input {
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

  .dropdown button {
    display: block;
    width: 100%;
    padding: 0.45rem 0.7rem;
    text-align: left;
    background: none;
    border: none;
    color: inherit;
    cursor: pointer;
    font: inherit;
  }

  .dropdown button:hover {
    background: var(--bg-hover, rgba(127, 127, 127, 0.15));
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
