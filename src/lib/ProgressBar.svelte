<script lang="ts">
  export let info = "";
  export let progress: number;
</script>

<div class="container">
  <div class="bar" class:infinite-load={progress < 0} style={`width: ${progress * 100}%;`}></div>
  {#if progress >= 0}
    <span class="text">{info}</span>
    <div class="color-layer"></div>
  {/if}
</div>

<style>
  .container {
    width: var(--width, inherit);
    height: var(--height, inherit);
    display: grid;

    background-color: white;
  }

  .bar,
  .text {
    height: var(--height, inherit);
    color: white;
    font-size: var(--height, inherit);
    line-height: var(--height, inherit);
    grid-column: 1;
    grid-row: 1;

    text-align: center;
  }

  .color-layer {
    position: absolute;
    background-color: var(--bar-color, #1cae30);
    height: var(--height, inherit);
    mix-blend-mode: screen;
    width: 100%;
  }

  .text {
    mix-blend-mode: difference;
  }

  .bar {
    background-color: black;
  }

  /* Infinite load */

  .infinite-load {
    border: 1px solid #bbb;
    position: relative;
    overflow: hidden;
    background: #e6e6e6;
  }

  .infinite-load:after {
    content: " ";
    display: block;
    width: 15%;
    top: -50%;
    height: 250%;
    position: absolute;
    animation: greenglow 1s linear infinite;
    background: var(--bar-color, #1cae30);
  }

  @keyframes greenglow {
    from {
      left: -120px;
    }
    to {
      left: 100%;
    }
  }
</style>
