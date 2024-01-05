<script lang="ts">
  import { onMount } from "svelte";
  import { launcherLogsStore, gameLogsStore } from "./stores/loggers";

  export let show: boolean;
  let showLauncherLogs = false;
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
        consoleElement.scrollTop =
          consoleElement.scrollHeight - consoleElement.clientHeight;
      }, 0);
      // }
    }
  }

  gameLogsStore.subscribe(() => {
    if (!showLauncherLogs) scrollConsole();
  });

  launcherLogsStore.subscribe(() => {
    if (showLauncherLogs) scrollConsole();
  });

  let hidden = false;
  $: {
    if (show) hidden = false;
    else setTimeout(() => (hidden = true), 500);

    scrollConsole();
  }

  onMount(() => {
    scrollConsole();
  })
</script>

<main class={`${show ? "shown" : ""}${hidden ? "hidden" : ""}`}>
  <div class="buttons">
    <button
      class={`logBtn ${showLauncherLogs ? "selected" : ""}`}
      on:click={() => (showLauncherLogs = true) && scrollConsole()}
      >Launcher logs</button
    >
    <button
      class={`logBtn ${!showLauncherLogs ? "selected" : ""}`}
      on:click={() => (showLauncherLogs = false) && scrollConsole()}
      >Game logs</button
    >
    <button class="closeBtn" on:click={() => (show = false)}>X</button>
  </div>
  <textarea class="logs" bind:this={consoleElement} readonly>
    {(showLauncherLogs ? $launcherLogsStore : $gameLogsStore).join("\n")}
  </textarea>
</main>

<style>
  * {
    user-select: none;
  }

  main {
    position: absolute;
    top: 0;
    left: 0;
    box-sizing: border-box;
    width: 100vw;
    height: 100vh;

    transition: transform 0.5s ease-in;

    background-color: #2c2c2c;
    display: grid;
    grid-template-rows: min-content 1fr;
  }

  main .buttons {
    display: grid;
    grid-template-columns: 1fr 1fr min-content;
  }

  main .buttons .logBtn {
    box-sizing: border-box;
    border: 4px solid #00000000;
  }

  main .buttons .logBtn.selected {
    border-bottom: 4px solid green;
  }

  main .buttons .closeBtn {
    font-size: 1.5rem;
    background: #ffffff17;

    height: 2rem;
    aspect-ratio: 1;
    /* width: 2rem; */
    padding: 0px;

    line-height: 2rem;
    margin: 1px 4px;
    /* border: none; */
    font-family: "Minecraft";
    text-align: center;
  }

  main .logs {
    background-color: #00000010;
    padding: 0;
    margin: 4px;

    padding: 3px;
    box-shadow: inset 0px 0px 0px 1px #0000005d;
    overflow-y: scroll;
    overflow-x: scroll; /*hidden;*/

    color: #fff;
    font-family: monospace;

    resize: none;
    outline: none;

    text-wrap: nowrap;
  }

  main .logs::-webkit-scrollbar {
    width: 5px;
    height: 5px;
  }

  main .logs::-webkit-scrollbar-thumb {
    background: #1a1a1a;
    border-radius: 20px;
  }

  main .logs * {
    margin: 0;
    padding: 0;
    text-wrap: wrap;
    width: 100%;
  }

  main:not(.shown) {
    transform: translateX(100vw);
  }

  main.hidden {
    opacity: 0;
  }
</style>
