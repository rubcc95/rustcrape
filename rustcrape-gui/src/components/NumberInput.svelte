<script lang="ts">
  let {
    value = $bindable(0),
    min = -Infinity,
    max = Infinity,
    step = 1,
    id = "",
    label = "",
    disabled = false,
  }: {
    value?: number;
    min?: number;
    max?: number;
    step?: number;
    id?: string;
    label?: string;
    disabled?: boolean;
  } = $props();

  function stepUp(): void {
    if (disabled) return;
    let newVal = value + step;
    if (Number.isInteger(step)) newVal = Math.round(newVal);
    if (newVal <= max) value = newVal;
  }

  function stepDown(): void {
    if (disabled) return;
    let newVal = value - step;
    if (Number.isInteger(step)) newVal = Math.round(newVal);
    if (newVal >= min) value = newVal;
  }
</script>

<div class="number-wrap" class:disabled>
  {#if label}
    <label for={id}>{label}</label>
  {/if}
  <div class="number-input-row">
    <input
      type="number"
      {id}
      bind:value
      {min}
      {max}
      {step}
      {disabled}
    />
    <div class="number-btns">
      <button class="num-btn" onclick={stepUp} tabindex="-1" type="button"
        disabled={disabled}>▲</button
      >
      <button class="num-btn" onclick={stepDown} tabindex="-1" type="button"
        disabled={disabled}>▼</button
      >
    </div>
  </div>
</div>

<style>
  .number-wrap {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1;
  }

  .number-wrap label {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-muted, #8892b0);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .number-input-row {
    display: flex;
    align-items: stretch;
  }

  .number-input-row :global(input[type="number"]) {
    flex: 1;
    min-width: 0;
    border-top-right-radius: 0;
    border-bottom-right-radius: 0;
    border-right: none;
  }

  .number-btns {
    display: flex;
    flex-direction: column;
    width: 24px;
    border: 1px solid var(--border, #2a3a5c);
    border-left: none;
    border-radius: 0 6px 6px 0;
    overflow: hidden;
    flex-shrink: 0;
  }

  .num-btn {
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

  .num-btn:hover {
    background: var(--sidebar-bg, #16213e);
    color: var(--text, #e0e0e0);
  }

  .num-btn:active {
    background: var(--border, #2a3a5c);
    transform: none;
  }

  .num-btn + .num-btn {
    border-top: 1px solid var(--border, #2a3a5c);
  }

  .disabled :global(input[type="number"]) {
    opacity: 0.35;
    background: var(--sidebar-bg, #16213e);
  }

  .disabled label {
    opacity: 0.4;
  }

  .disabled .number-btns {
    opacity: 0.35;
  }
</style>
