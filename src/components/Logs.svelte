<script lang="ts">
  import { onMount } from "svelte";
  import type { Log } from "$/ipc/stores/loggers";

  export let logs: Log[];

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

<section
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
  {#each logs as { message, level }}
    <span class={level}>{message}</span>
  {/each}
</section>

<style>
  .logs {
    background-color: #fff;
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

  .logs span {
    display: block;
    margin: 0;
    padding: 0;
    /* text-wrap: wrap; */
    width: 100%;
    white-space: pre;
  }

  .logs span.warn,
  .logs span.debug {
    color: #ffa500;
  }

  .logs span.error {
    color: #ff0000;
  }

  .logs span.fatal {
    color: #8b0000;
  }
</style>
