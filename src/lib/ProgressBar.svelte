<script lang="ts">
  export let info = "";
  export let progress: number;
</script>

<div class="container">
  <div class="bar" style={`width: ${progress * 100}%;`}></div>
  <span class="text">{info}</span>
</div>

<style>
  .container {
    margin: 5px 0;

    width: var(--width, inherit);
    height: var(--height, inherit);
    display: grid;
  }

  .bar,
  .text {
    grid-column: 1;
    grid-row: 1;
  }

  .bar {
    background-color: green;
  }
</style>
