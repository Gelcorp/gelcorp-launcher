<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { invoke } from "@tauri-apps/api/tauri";
  import { gameLogsStore, launcherLogsStore } from "./stores/loggers";

  import LauncherTabs from "./lib/LauncherTabs.svelte";
  import ProgressBar from "./lib/ProgressBar.svelte";

  let progress: { status: string; current: number; total: number } | undefined;

  // Keep this active
  gameLogsStore.subscribe(() => {});

  let playButton: HTMLButtonElement;
  async function startGame() {
    if (playButton.disabled) return;
    playButton.disabled = true;
    gameLogsStore.clear();

    try {
      await invoke("start_game");
    } catch (e) {
      console.error(e);
      launcherLogsStore.log("Failed to launch the game: " + e);
    } finally {
      playButton.disabled = false;
    }
  }

  onMount(() => {
    const unlisten = listen("update_progress", ({ payload }) => {
      progress = payload as { status: string; current: number; total: number } | undefined;
    });
    return () => unlisten.then((fn) => fn());
  });
</script>

<main class="container">
  <div class="upper">
    <LauncherTabs />
  </div>
  <div class="lower">
    {#if progress}
      <div class="progressbar-container">
        <ProgressBar info={progress?.status} progress={progress?.current / progress?.total} --height="14px" --bar-color="#0078d7" />
      </div>
    {/if}
    <div class="lower-parts">
      <div class="left">
        <img src="gelcorp-title.png" alt="logo" draggable="false" />
      </div>
      <div class="right">
        <button class="main-btn" bind:this={playButton} on:click={startGame}>Jugar</button>
      </div>
    </div>
  </div>
</main>

<style>
  .container {
    background-repeat: no-repeat;
    background-size: cover;
    image-rendering: optimizeQuality;

    display: grid;
    grid-template-rows: 1fr min-content;
    justify-content: stretch;
    justify-items: stretch;
    max-height: 100%;
    overflow: hidden;
  }

  .upper {
    display: grid;
    /* place-content: center; */
    /* align-items: center; */
    justify-content: stretch;
    align-content: stretch;
    overflow: hidden;
  }

  .main-btn {
    background-color: #2a632a;
    border: 2px solid #1c421c;
    font-size: 2.5rem;
    padding: 0.2rem 1.8rem;
    font-family: "Minecraft Ten";
    text-transform: uppercase;
    font-weight: 800;
    color: white;
  }

  .main-btn:disabled {
    background-color: #5c5e5c;
    border: 2px solid #3a3b3a;
  }

  .main-btn:hover:not(:disabled) {
    background-color: #225022;
  }

  .main-btn:active:not(:disabled) {
    background-color: #163316;
  }

  .lower {
    grid-template-rows: min-content 1fr;
    align-items: center;
    /* border-top: 2px solid #000; */
  }

  .lower-parts {
    background-image: url("/bedrock.png");
    background-size: 84px;
    image-rendering: pixelated;

    box-shadow: inset 0 -10px 80px -5px black;

    text-align: center;

    display: grid;
    grid-template-columns: 1fr 1fr;
    align-items: center;
  }

  .progressbar-container {
    grid-column: -1 / -3;
  }

  .lower .left {
    display: flex;
    justify-content: start;
    padding-left: 2em;
  }

  .lower .left img {
    box-sizing: border-box;
    width: 80%;
    max-width: 260px;
    padding: 15px 5px;
  }

  .lower .right {
    display: flex;
    justify-content: end;
    gap: 10px;
    padding-right: 2em;
  }

  .lower .right .user-info * {
    margin: 0;
    text-wrap: nowrap;
  }
</style>
