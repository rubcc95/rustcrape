<script lang="ts">
  import { projectStore } from "../lib/stores/project.store.svelte";
  import NumberInput from "./NumberInput.svelte";
  import CheckboxNumber from "./CheckboxNumber.svelte";
  import Panel from "./Panel.svelte";
  import Toggle from "./Toggle.svelte";
</script>

<Panel title="Ejecución">
  <div class="field-row">
    <CheckboxNumber
      label="Rate limit (por hora)"
      id="rate-limit"
      min={1}
      bind:value={projectStore.executionConfig.rate_limit}
    />
    <CheckboxNumber
      label="Iteraciones"
      id="iterations"
      min={1}
      bind:value={projectStore.executionConfig.iterations}
    />
  </div>

  <div class="field-row">
    <NumberInput
      label="Delay min (ms)"
      id="delay-min"
      min={100}
      bind:value={projectStore.executionConfig.delay_min}
    />
    <NumberInput
      label="Delay max (ms)"
      id="delay-max"
      min={100}
      bind:value={projectStore.executionConfig.delay_max}
    />
  </div>

  <div class="field-row">
    <NumberInput
      label="Stop threshold"
      id="stop-threshold"
      min={1}
      bind:value={projectStore.executionConfig.stop_threshold}
    />
    <div class="field toggle-field">
      <Toggle bind:checked={projectStore.executionConfig.headless} ariaLabel="Headless" />
    </div>
  </div>

  <div class="field-row">
   <CheckboxNumber
      label="VPN (frecuencia de rotación)"
      id="ip_rotation_frequency"
      min={1}
      bind:value={projectStore.executionConfig.ip_rotation_frequency}
    />     
  </div>
</Panel>

<style>
  .field-row {
    display: flex;
    gap: 12px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1;
  }


</style>
