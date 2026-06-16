<script lang="ts">
  import { appStore } from "../lib/stores/app.store.svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";

  let announcement = $derived(appStore.announcement);

  function close(): void {
    if (announcement?.closeable) {
      appStore.announcement = null;
    }
  }

  async function openLink(): Promise<void> {
    if (!announcement) return;
    await openUrl(announcement.link);
  }
</script>

{#if announcement}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="modal-overlay" onclick={announcement.closeable ? close : undefined} role="dialog" tabindex="-1">
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div class="modal-card" onclick={(e) => e.stopPropagation()} role="presentation">
      <h2 class="modal-title">{announcement.title}</h2>
      <p class="modal-content">{announcement.content}</p>
      <div class="modal-actions">
        <button class="btn-primary btn-link" onclick={openLink}>
          {announcement.link_label}
        </button>
        {#if announcement.closeable}
          <button class="btn-small" onclick={close}>Cerrar</button>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal-card {
    background: var(--card-bg, #1f2b47);
    border: 1px solid var(--border, #2a3a5c);
    border-radius: 10px;
    padding: 32px 36px 28px;
    min-width: 360px;
    max-width: 480px;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
    text-align: center;
  }

  .modal-title {
    font-size: 20px;
    color: var(--text, #e0e0e0);
    margin: 0 0 16px 0;
  }

  .modal-content {
    font-size: 15px;
    color: var(--text, #e0e0e0);
    margin-bottom: 28px;
    line-height: 1.6;
    white-space: pre-wrap;
  }

  .modal-actions {
    display: flex;
    gap: 12px;
    justify-content: center;
    flex-wrap: wrap;
  }

  .btn-link {
    min-width: 140px;
    padding: 10px 24px;
    font-size: 14px;
    text-decoration: none;
  }

  .btn-link:hover {
    filter: brightness(1.1);
  }
</style>
