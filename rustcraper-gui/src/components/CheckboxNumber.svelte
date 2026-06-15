<script lang="ts">
  let {
    value = $bindable(0),
    min = 1,
    id = "",
    label = "",
    hint = "",
  }: {
    value?: number;
    min?: number;
    id?: string;
    label?: string;
    hint?: string;
  } = $props();

  let checked = $derived(value > 0);

  function toggleCheck(): void {
    if (value > 0) {
      value = 0;
    } else {
      value = min;
    }
  }

  function stepUp(): void {
    if (!checked) return;
    value = value + 1;
  }

  function stepDown(): void {
    if (!checked) return;
    let next = value - 1;
    if (next >= min) value = next;
  }
</script>

<div class="cn-wrap">
  {#if label}
    <!-- svelte-ignore a11y_label_has_associated_control -->
    <label for={id}>{label}</label>
  {/if}
  <div class="cn-row">
    <div
      class="cn-check"
      class:checked
      role="checkbox"
      aria-checked={checked}
      tabindex="0"
      onclick={toggleCheck}
      onkeydown={(e) => e.key === 'Enter' && toggleCheck()}
    >
      <svg class="check-mark" viewBox="0 0 24 24" width="18" height="18">
        <path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z" fill="currentColor" />
      </svg>
    </div>
    <div class="cn-input-row">
      <input
        type="number"
        {id}
        bind:value
        disabled={!checked}
      />
      <div class="cn-btns">
        <button class="cn-btn" onclick={stepUp} tabindex="-1" type="button"
          disabled={!checked}>▲</button>
        <button class="cn-btn" onclick={stepDown} tabindex="-1" type="button"
          disabled={!checked}>▼</button>
      </div>
    </div>
  </div>
  {#if hint}
    <span class="hint">{hint}</span>
  {/if}
</div>

<style>
  .cn-wrap {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1;
  }

  .cn-wrap label {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-muted, #8892b0);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .cn-row {
    display: flex;
    gap: 8px;
    align-items: stretch;
  }

  .cn-check {
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

  .cn-check:hover {
    border-color: var(--text-muted, #8892b0);
  }

  .cn-check.checked {
    background: var(--primary, #4f8cff);
    border-color: var(--primary, #4f8cff);
  }

  .cn-check .check-mark {
    color: #fff;
    opacity: 0;
    transform: scale(0.5);
    transition: opacity 0.15s, transform 0.15s;
  }

  .cn-check.checked .check-mark {
    opacity: 1;
    transform: scale(1);
  }

  .cn-input-row {
    display: flex;
    align-items: stretch;
    flex: 1;
    min-width: 0;
  }

  .cn-input-row :global(input[type="number"]) {
    flex: 1;
    min-width: 0;
    border-top-right-radius: 0;
    border-bottom-right-radius: 0;
    border-right: none;
  }

  .cn-input-row :global(input[type="number"]:disabled) {
    opacity: 0.35;
  }

  .cn-btns {
    display: flex;
    flex-direction: column;
    width: 24px;
    border: 1px solid var(--border, #2a3a5c);
    border-left: none;
    border-radius: 0 6px 6px 0;
    overflow: hidden;
    flex-shrink: 0;
    transition: opacity 0.15s;
  }

  .cn-btns:has(.cn-btn:disabled) {
    opacity: 0.35;
  }

  .cn-btn {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    border: none;
    border-radius: 0;
    background: var(--card-bg, #1f2b47);
    color: var(--text-muted, #8892b0);
    cursor: pointer;
    font-size: 7px;
    line-height: 1;
    min-height: 0;
    transition: background 0.1s, color 0.1s;
  }

  .cn-btn:hover {
    background: var(--sidebar-bg, #16213e);
    color: var(--text, #e0e0e0);
  }

  .cn-btn:active {
    background: var(--border, #2a3a5c);
    transform: none;
  }

  .cn-btn + .cn-btn {
    border-top: 1px solid var(--border, #2a3a5c);
  }

  .cn-btn:disabled {
    cursor: not-allowed;
  }

  .hint {
    font-size: 11px;
    color: var(--text-muted, #8892b0);
  }
</style>
