<script lang="ts">
  let {
    value = $bindable(false),
    label = "",
    ariaLabel = label,
  }: {
    value?: boolean;
    label?: string;
    ariaLabel?: string;
  } = $props();

  function toggle(): void {
    value = !value;
  }
</script>

<div class="toggle-row">
  <div
    class="check-track"
    class:checked={value}
    role="checkbox"
    aria-checked={value}
    aria-label={ariaLabel}
    tabindex="0"
    onclick={toggle}
    onkeydown={(e) => e.key === 'Enter' && toggle()}
  >
    <svg class="check-mark" viewBox="0 0 24 24" width="18" height="18">
      <path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z" fill="currentColor" />
    </svg>
  </div>
  {#if label}
    <span class="toggle-label">{label}</span>
  {/if}
</div>

<style>
  .toggle-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .toggle-label {
    font-size: 13px;
    color: var(--text, #e0e0e0);
    font-weight: 400;
  }

  .check-track {
    width: 34px;
    height: 34px;
    border: 1px solid var(--border, #2a3a5c);
    border-radius: 6px;
    background: var(--bg);
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
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
    transition: opacity 0.15s, transform 0.15s;
  }

  .check-track.checked .check-mark {
    opacity: 1;
    transform: scale(1);
  }
</style>
