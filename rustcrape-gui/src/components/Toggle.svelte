<script lang="ts">
  let {
    checked = $bindable(false),
    ariaLabel = "",
  }: {
    checked?: boolean;
    ariaLabel?: string;
  } = $props();
</script>

<div
  class="check-track"
  class:checked
  role="checkbox"
  aria-label={ariaLabel || undefined}
  aria-checked={checked}
  tabindex="0"
  onclick={() => checked = !checked}
  onkeydown={(e) => e.key === 'Enter' && (checked = !checked)}
>
  <svg class="check-mark" viewBox="0 0 24 24" width="18" height="18">
    <path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z" fill="currentColor" />
  </svg>
</div>

<style>
  .check-track {
    width: 34px;
    height: 34px;
    border: 1px solid var(--border);
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
    border-color: var(--text-muted);
  }

  .check-track.checked {
    background: var(--primary);
    border-color: var(--primary);
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
