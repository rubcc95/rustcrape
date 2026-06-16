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
      <fieldset>
        <legend>VPN</legend>
        <div class="field">
          <label for="nordvpn-path">Ejecutable de NordVPN</label>
          <div class="file-picker">
            <input
              type="text"
              id="nordvpn-path"
              readonly
              placeholder="Seleccionar ejecutable..."
              bind:value={nordvpnPath}
            />
            <button class="btn-small" onclick={() => onPickExecutable("nordvpn")}>Examinar</button>
          </div>
        </div>
      </fieldset>

      <fieldset>
        <legend>Navegador</legend>
        <div class="field">
          <label for="browser-path">Ejecutable del navegador</label>
          <div class="file-picker">
            <input
              type="text"
              id="browser-path"
              readonly
              placeholder="Seleccionar ejecutable..."
              bind:value={browserPath}
            />
            <button class="btn-small" onclick={() => onPickExecutable("browser")}>Examinar</button>
          </div>
        </div>
      </fieldset>

    <div class="actions">
      <button class="btn-primary" onclick={onSave}>Guardar</button>
    </div>
  </div>
</div>

<style>
  .settings {
    max-width: 720px;
    margin: 0 auto;
  }

  .settings-body {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  fieldset {
    border: 1px solid var(--border, #2a3a5c);
    border-radius: 8px;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  legend {
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 1px;
    color: var(--primary, #4f8cff);
    padding: 0 8px;
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

  .file-picker {
    display: flex;
    gap: 8px;
  }

  .file-picker input {
    flex: 1;
    padding: 8px 12px;
    border: 1px solid var(--border, #2a3a5c);
    border-radius: 6px;
    background: var(--bg, #1a1a2e);
    color: var(--text-muted, #8892b0);
    font-size: 13px;
    font-family: var(--font);
    cursor: pointer;
    outline: none;
  }

  .actions {
    display: flex;
    gap: 8px;
  }
</style>
