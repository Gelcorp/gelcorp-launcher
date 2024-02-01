<script lang="ts">
  export let value: number;
  export let min: number;
  export let max: number;
  export let step = 512;
  export let disabled = false;
</script>

<div class="memory_control">
  <input type="range" bind:value {min} {max} {step} {disabled} />
  <span>{(value / 1024).toFixed(1)}G</span>
</div>

<style>
  .memory_control {
    display: flex;
    justify-content: space-between;
  }

  input {
    width: 100%;
  }
</style>
