<script lang="ts">
  import { onMount } from "svelte";

  export let logs: string[];

  let consoleElement: HTMLElement;
  function scrollConsole() {
    if (consoleElement) {
      // let scroll = Math.abs(
      //   consoleElement.scrollHeight -
      //     consoleElement.clientHeight -
      //     consoleElement.scrollTop
      // );
      // if (scroll > 1) {
      setTimeout(() => {
        consoleElement.scrollTop = consoleElement.scrollHeight - consoleElement.clientHeight;
      }, 0);
      // }
    }
  }

  $: {
    logs && scrollConsole();
  }

  onMount(scrollConsole);
</script>

<div
  class="logs"
  bind:this={consoleElement}
  contenteditable="true"
  on:keydown={(e) => !e.metaKey && !e.ctrlKey && e.preventDefault()}
  on:paste|preventDefault={() => {}}
  on:cut|preventDefault={() => {}}
  spellcheck="false"
  role="textbox"
  tabindex="0"
>
  {#each logs as log}
    <p>{log}</p>
  {/each}
</div>

<style>
  .logs {
    background-color: #00000010;
    /* padding: 3px; */
    overflow: auto;

    /* color: #fff; */
    font-family: monospace;
    font-size: 13px;
    line-height: 17px;

    resize: none;
    outline: none;

    text-wrap: nowrap;
    height: 100%;

    box-sizing: border-box;
  }

  .logs * {
    margin: 0;
    padding: 0;
    /* text-wrap: wrap; */
    width: 100%;
  }
</style>
