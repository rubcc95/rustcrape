<script lang="ts">
  import { appStore } from "../lib/stores/app.store.svelte";
  import { pickExecutable, saveGlobalSettings, onExecutablePicked } from "../lib/tauri";
 
  let nordvpnPath = $state("");
  let browserPath = $state("");

  $effect(() => {
    nordvpnPath = appStore.globalSettings.nordvpn_path ?? "";
  });

  $effect(() => {
    browserPath = appStore.globalSettings.browser_path ?? "";
  });

  async function onPickExecutable(key: string): Promise<void> {
    await pickExecutable(key);
  }

  function clearPath(key: string): void {
    if (key === "browser") {
      browserPath = "";
      appStore.setBrowserPath(null);
    } else {
      nordvpnPath = "";
      appStore.setNordvpnPath(null);
    }
  }

  $effect(() => {
    const unlistenPromise = onExecutablePicked(({ key, path }) => {
      if (path) {
        if (key === "browser") {
          browserPath = path;
          appStore.setBrowserPath(path);
        } else {
          nordvpnPath = path;
          appStore.setNordvpnPath(path);
        }
      }
    });
    return () => {
      unlistenPromise.then((fn) => fn());
    };
  });

  async function onSave(): Promise<void> {
    const settings = {
      nordvpn_path: nordvpnPath || null,
      browser_path: browserPath || null,
    };
    appStore.setNordvpnPath(settings.nordvpn_path);
    appStore.setBrowserPath(settings.browser_path);
    await saveGlobalSettings(settings);
  }
</script>

<div class="settings">
  <div class="settings-body">
      <div class="panel">
        <h4>VPN</h4>
        <div class="field">
          <label for="nordvpn-path">Ejecutable de NordVPN</label>
          <div class="file-row">
            <button class="file-btn" onclick={() => onPickExecutable("nordvpn")}>
              {nordvpnPath || "Seleccionar ejecutable..."}
            </button>
            {#if nordvpnPath}
              <button class="clear-btn" onclick={() => clearPath("nordvpn")} aria-label="Limpiar">
                <svg viewBox="0 0 24 24" width="14" height="14">
                  <path d="M18 6L6 18M6 6l12 12" stroke="currentColor" stroke-width="2.5" fill="none" stroke-linecap="round" />
                </svg>
              </button>
            {/if}
          </div>
        </div>
      </div>

      <div class="panel">
        <h4>Navegador</h4>
        <div class="field">
          <label for="browser-path">Ejecutable del navegador</label>
          <div class="file-row">
            <button class="file-btn" onclick={() => onPickExecutable("browser")}>
              {browserPath || "Seleccionar ejecutable..."}
            </button>
            {#if browserPath}
              <button class="clear-btn" onclick={() => clearPath("browser")} aria-label="Limpiar">
                <svg viewBox="0 0 24 24" width="14" height="14">
                  <path d="M18 6L6 18M6 6l12 12" stroke="currentColor" stroke-width="2.5" fill="none" stroke-linecap="round" />
                </svg>
              </button>
            {/if}
          </div>
        </div>
      </div>

    <div class="actions">
      <button class="btn-primary" onclick={onSave}>Guardar</button>
    </div>
  </div>
</div>

<style>
  .settings {
    width: 520px;
    margin: 0 auto;
  }

  .settings-body {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .panel {
    flex: 1;
    background: var(--card-bg, #1f2b47);
    border: 0px;
    border-radius: 8px;
    padding: 16px;
  }
 
  .panel h4 {
    font-size: 12px;
    margin: 0 0 8px 0;
    text-transform: uppercase;
    letter-spacing: 1px;
    color: var(--primary, #8892b0);
  }
  
  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .field label {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-muted, #8892b0);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .file-btn {
    width: 100%;
    padding: 8px 12px;
    border: 1px solid var(--border, #2a3a5c);
    border-radius: 6px;
    background: var(--bg, #1a1a2e);
    color: var(--text-muted, #8892b0);
    font-size: 13px;
    font-family: var(--font);
    cursor: pointer;
    text-align: left;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    outline: none;
  }

  .file-row {
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .file-row .file-btn {
    flex: 1;
  }

  .clear-btn {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    border: none;
    padding: 0;
    line-height: 1;
    background: #e53e3e;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    transition: background 0.15s;
  }

  .clear-btn:hover {
    background: #c53030;
  }

  .clear-btn svg {
    display: block;
    color: #fff;
  }

  .actions {
    display: flex;
    gap: 8px;
  }
</style>
