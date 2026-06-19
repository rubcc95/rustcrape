<script lang="ts">
  let {
    open = false,
    title = "Confirmar",
    message = "¿Estás seguro?",
    onConfirm = () => {},
    onCancel = () => {},
  }: {
    open?: boolean;
    title?: string;
    message?: string;
    onConfirm?: () => void;
    onCancel?: () => void;
  } = $props();
</script>

{#if open}
  <div class="modal-overlay" role="presentation" onclick={onCancel} onkeydown={(e) => e.key === 'Escape' && onCancel()}>
    <div class="modal-card" role="dialog" aria-modal="true" tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
      <h3 class="modal-title">{title}</h3>
      <p class="modal-message">{message}</p>
      <div class="modal-actions">
        <button class="btn-small" onclick={onCancel}>Cancelar</button>
        <button class="btn-danger" onclick={onConfirm}>Confirmar</button>
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
    padding: 28px 32px 24px;
    min-width: 340px;
    max-width: 420px;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
    text-align: center;
  }

  .modal-title {
    font-size: 16px;
    color: var(--text, #e0e0e0);
    margin: 0 0 12px 0;
  }

  .modal-message {
    font-size: 15px;
    color: var(--text, #e0e0e0);
    margin-bottom: 24px;
    line-height: 1.5;
  }

  .modal-actions {
    display: flex;
    gap: 10px;
    justify-content: center;
  }

  .modal-actions :global(.btn-small) {
    min-width: 100px;
    padding: 8px 20px;
    font-size: 13px;
  }

  .modal-actions :global(.btn-danger) {
    min-width: 100px;
    padding: 8px 20px;
    font-size: 13px;
  }
</style>
