---
name: frontend-gui
description: >-
  Use when working on the rustcrape-gui frontend — the Svelte 5 + Tauri v2
  desktop app in rustcrape-gui/. Covers views, components, class-based stores,
  Tauri invoke/event patterns, Svelte 5 rune conventions (class stores/$state
  not top-level $state exports), 3-route navigation, CSS variables, and build
  with `npm run build` in rustcrape-gui/. NOT for Rust backend commands or
  persistence (those are in src-tauri/src/).
---

# rustcrape-gui — Svelte 5 Frontend

## Project structure

```
rustcrape-gui/
├── package.json              # Scripts: build, dev, check, preview, tauri
├── vite.config.ts            # Vite + Svelte plugin, Tauri dev host
├── svelte.config.js          # vitePreprocess only
├── tsconfig.json
├── index.html                # Mount point: <div id="app"></div>
└── src/
    ├── main.ts               # mount(App, { target: document.getElementById("app")! })
    ├── App.svelte            # Root layout: WindowControls + route switching
    ├── styles.css            # CSS variables, base input/button styles, scrollbar
    ├── assets/
    ├── lib/
    │   ├── types.ts          # All TypeScript interfaces
    │   ├── tauri.ts          # Wrappers for all invoke/event calls
    │   └── stores/
    │       ├── app.store.svelte.ts       # Navigation + global settings
    │       ├── project.store.svelte.ts   # Project CRUD + form state
    │       └── execution.store.svelte.ts # Execution lifecycle
    ├── views/
    │   ├── SettingsView.svelte
    │   ├── ProjectView.svelte
    │   └── ExecutionView.svelte
    └── components/
        ├── WindowControls.svelte    # Custom title bar (drag + min/max/close)
        ├── Sidebar.svelte           # Project list + settings button
        ├── ExecutionConfig.svelte   # Rate limit, delays, VPN, headless, etc.
        ├── UnsavedProjectForm.svelte # Search term, zoom, DB fields
        ├── SavedProjectInfo.svelte  # Read-only project info + mock stats
        ├── LogTerminal.svelte       # Scrolling log viewer
        ├── ProgressPanel.svelte     # 4 stat cards grid
        ├── Modal.svelte             # Confirm dialog overlay
        └── NumberInput.svelte       # Number field with ▲▼ spinner buttons
```

## Architecture

### 3-route navigation (`appStore.currentRoute`)
- `"settings"` → SettingsView (vertical centered, no sidebar)
- `"project"` → Sidebar + ProjectView (flex row, sidebar left)
- `"execution"` → ExecutionView (full width, no sidebar)

Routes are switched in `App.svelte` with `{#key appStore.currentRoute}` blocks.
The `layout` wrapper (sidebar + main) is only rendered for `"project"`.

### Layout containers (App.svelte)
- `.view-container` — scrollable, used by execution view
- `.view-container--centered` — flexbox centering on both axes, used by settings
- `div.layout > Sidebar + main.main-content` — sidebar layout for project view (`.main-content` also vertically centers ProjectView via `justify-content: center`)

## Svelte 5 conventions

### Stores: class-based with `$state` fields (NEVER top-level `$state` exports)

Every store is a class instantiated as a singleton with `export const storeName = new StoreClass()`. This avoids Svelte 5's `state_invalid_export` error that fires when reassigning a top-level `$state` variable across module boundaries.

**Store file naming:** `*.store.svelte.ts` (the `.svelte.ts` extension is required for Svelte 5 rune support).

```typescript
// ✅ Correct: class-based
class MyStore {
  someState = $state("hello");
  updateState(): void { this.someState = "world"; }
}
export const myStore = new MyStore();

// ❌ WRONG: top-level $state exports (causes state_invalid_export)
// export let someState = $state("hello");
```

Key rule: use `$state()` on class FIELDS, not on `let` at module top level. Full mutation works (push, splice, reassign, =).

### Components: `$props()` and `$bindable()`

```typescript
let {
  value = $bindable(0),
  label = "",
  min = -Infinity,
}: {
  value?: number;
  label?: string;
  min?: number;
} = $props();
```

### Direct store access (no prop-drilling)

Components import and read stores directly rather than receiving store values via props. This is intentional to keep interfaces simple and avoid `$bindable()` complexity on intermediary components.

```typescript
import { projectStore } from "../lib/stores/project.store.svelte";
// Then use projectStore.executionConfig.rate_limit directly in template
```

### Other runes used
- `$derived(expression)` — computed values
- `$effect(() => { ... return () => cleanup; })` — side effects + teardown

### Event handling
- `onclick={() => fn()}` — never `on:click`, never inline handler names without arrow wrapper
- `onkeydown={(e) => e.key === "Enter" && fn()}`

## Key types (src/lib/types.ts)

```typescript
type Route = "settings" | "project" | "execution";

interface Config {
  search: SearchConfig;    // persistent (zoom, query), stop_threshold, delays, headless
  rate_limit: number;
  iterations: number;
  db: DbConfig;            // host, port, user, password, database
  nordvpn_path: string | null;   // null = VPN disabled
  ip_rotation_frequency: number;
}

interface ExecutionConfig {
  rate_limit, iterations, delay_min, delay_max, stop_threshold;
  use_vpn: boolean;           // toggles nordvpn_path and ip_rotation_frequency in buildConfig
  ip_rotation_frequency;
  headless: boolean;
}

interface ProjectDraft {
  name, search_query, zoom;
  db_host, db_port, db_database, db_user, db_password;
}

interface GlobalSettings {
  nordvpn_path: string | null;
}

interface SavedConfig {
  id: string;
  name: string;
  config: Config;
  started: boolean;
}

interface IterationStats {
  bounds_total, bounds_processed, bounds_remaining, results_found, phones_found: number;
}

interface LogEntry { id, timestamp, kind, message: string; }
```

## State management flow

### ProjectStore
- `projectStore.projects` — full list of `SavedConfig[]`
- `projectStore.currentProject` — selected project (null = new project)
- `projectStore.executionConfig` — mutable `ExecutionConfig` (same form for new and saved)
- `projectStore.projectDraft` — mutable `ProjectDraft` (name, search query, DB fields)
- `projectStore.buildConfig()` → merges executionConfig + projectDraft + globalSettings into a `Config`
- `projectStore.selectProject(p)` — populates form fields from a saved project
- `projectStore.saveCurrentProject()` — persists and updates list
- `projectStore.removeProject(id)` — deletes and resets if was current

### ExecutionStore
- `executionStore.isRunning` — whether scraping task is active
- `executionStore.isCancelling` — whether cancel has been requested but not confirmed yet
- `executionStore.startExecution(config)` — resets state, sets configSnapshot, navigates to execution, fires `runScraping`
- `executionStore.cancelExecution()` — sets `isCancelling=true`, fires `cancelScraping`, button shows "Cancelando..." until `scraping-finished` resets it
- `executionStore.goBack()` — if running, fires cancel silently + navigates immediately (cancellation continues in background)
- `executionStore.setupExecutionListeners()` — registers Tauri event listeners, returns cleanup function

### Event flow (execution)
```
Frontend calls runScraping(config)
  → Rust spawns async task, returns Ok(()) immediately
  → Rust emits "scraping-started"
  → Rust emits "verboser-event" (kind, message) during scraping
  → Rust emits "scraping-finished" when done

Cancel flow:
  Frontend calls cancelScraping()
    → Rust sets AtomicBool(true), returns immediately
    → Rust async task checks flag, stops, emits "scraping-finished"
```

## Tauri integration (src/lib/tauri.ts)

All Tauri `invoke` and `listen` calls are wrapped in `tauri.ts` as async functions.

### Commands (invoke)
- `listConfigs()`, `saveConfig(name, config, started)`, `deleteConfig(id)`
- `getLastSelected()`, `setLastSelected(id)`
- `runScraping(config)`, `cancelScraping()`
- `loadGlobalSettings()`, `saveGlobalSettings(settings)`
- `pickExecutable()` — opens native file dialog, emits `executable-picked` event

### Events (listen)
- `onVerboserEvent(callback)` — `VerboserPayload { kind, message }`
- `onScrapingStarted(callback)` — no payload
- `onScrapingFinished(callback)` — no payload
- `onExecutablePicked(callback)` — `string | null` (file path or cancelled)

### SettingsView pattern (file picker + global settings)

```typescript
// Listen for file dialog result
$effect(() => {
  const unlistenPromise = onExecutablePicked((path) => {
    if (path) {
      nordvpnPath = path;
      appStore.setNordvpnPath(path);
    }
  });
  return () => { unlistenPromise.then((fn) => fn()); };
});

// Save global settings
async function onSave(): Promise<void> {
  const settings = { nordvpn_path: nordvpnPath || null };
  appStore.setNordvpnPath(settings.nordvpn_path);
  await saveGlobalSettings(settings);
}
```

### Execution events pattern (in setupExecutionListeners)

```typescript
onScrapingStarted(() => { this.isRunning = true; }).then((fn) => unlisteners.push(fn));
onScrapingFinished(() => {
  this.isRunning = false;
  this.isCancelling = false;
}).then((fn) => unlisteners.push(fn));
```

## CSS conventions

### CSS variables (in src/styles.css)
```css
--bg: #1a1a2e;              --sidebar-bg: #16213e;
--card-bg: #1f2b47;         --border: #2a3a5c;
--text: #e0e0e0;            --text-muted: #8892b0;
--primary: #4f8cff;         --primary-hover: #3a7bff;
--run: #2ecc71;             --run-hover: #27ae60;
--danger: #e74c3c;          --danger-hover: #c0392b;
--warn: #f39c12;            --info: #3498db;
--error: #e74c3c;
--font: "Inter", "Segoe UI", system-ui, sans-serif;
```

### Component styles
- **Always scoped** (`<style>` inside `.svelte`)
- Use `:global()` only when necessary (e.g., to style global types like `html, body`, or child elements from parent)
- Use `var(--name, fallback)` for all CSS variable references
- Always include a fallback value: `var(--primary, #4f8cff)`
- `fieldset + fieldset { margin-top: 20px }` for consistent section spacing
- Panel pattern: `background`, `border: 1px solid var(--border)`, `border-radius: 8px`, `padding: 16px`
- Legend pattern: small uppercase, `color: var(--primary)`, letter-spacing

### Fixed conventions in ExecutionView (bottom button state machine)
```
isRunning && !isCancelling → "Cancelar" (btn-danger, clickable)
isRunning && isCancelling  → "Cancelando..." (disabled, opacity 0.6, cursor not-allowed)
!isRunning                 → "Reanudar" (btn-primary, navigates to project)
```

Top "←" button always calls `executionStore.goBack()` (cancels silently if running, navigates immediately).

## Build & verification

**Only run in `rustcrape-gui/`:**

```bash
cd rustcrape-gui && npm run build
```

This runs `tsc && vite build`. Output goes to `rustcrape-gui/dist/`.

**DO NOT compile Rust** (`cargo check`, `cargo build`, `cargo test`). The Rust Tauri backend requires system-specific dependencies. Only run `npm run build`.

**Type checking:**

```bash
cd rustcrape-gui && npm run check
```

This runs `svelte-check` which catches Svelte-specific errors.

## Critical constraints

1. **NEVER export `$state` at top level** — always wrap in a class. The `state_invalid_export` error fires on reassignment of top-level `$state` across module boundaries.
2. **Store files must end in `.svelte.ts`** — otherwise Svelte 5 won't process runes.
3. **DO NOT import from `svelte/store`** (writable, readable, derived) — everything uses `$state` in class instances.
4. **Components import stores directly** — avoid prop-drilling for store data; use `$props()` only for truly local component props like `stats` in ProgressPanel or `logs` in LogTerminal.
5. **No `on:click`**, no `on:submit` — use `onclick={() => fn()}`, `onsubmit={(e) => ...}`.
6. **`svelte-check` must pass** before considering a task done.
7. **Use `$effect()` for lifecycle** (init + cleanup), not `onMount`.
8. **`runScraping` returns immediately** after spawning the async task in Rust — actual progress comes via Tauri events.
9. **`use_vpn` toggle** maps to `nordvpn_path: null` when disabled; `ip_rotation_frequency` set to `0` when VPN off (handled in `buildConfig()`).

## Related backend commands (src-tauri/src/commands.rs)

These are the Rust commands the frontend calls — do NOT edit Rust files unless the task explicitly requires backend changes:

| Command | Purpose |
|---------|---------|
| `list_configs` | Returns all saved configs |
| `save_config(name, config, started)` | Creates/updates a config file |
| `delete_config(id)` | Deletes by id |
| `get_last_selected` | Returns last selected config |
| `set_last_selected(id)` | Persists last selected |
| `run_scraping(config)` | Spawns async scraper, emits events |
| `cancel_scraping` | Sets AtomicBool(true) |
| `pick_executable` | Native file dialog, emits `executable-picked` |
| `load_global_settings` | Returns `GlobalSettings` from settings.json |
| `save_global_settings(settings)` | Persists to settings.json |

## Build output sizes (reference)
- `dist/index.html`: ~0.40 kB
- JS bundle: ~84 kB (~27 kB gzip)
- CSS bundle: ~15.5 kB (~3 kB gzip)
