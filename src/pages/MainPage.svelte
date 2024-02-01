<script lang="ts">
  import LauncherTabs from "$/components/LauncherTabs.svelte";
  import ProgressBar from "$/components/ProgressBar.svelte";
  import { gameLogsStore, launcherLogsStore } from "$/ipc/stores/loggers";
  import { progressStore } from "$/ipc/stores/progress";
  import { GameStatus, gameStatusStore } from "$/ipc/stores/game_status";

  let selectedTab = 0;

  $: isRunning = $gameStatusStore !== GameStatus.Idle;
  function handleClick() {
    if (isRunning) return;
    gameLogsStore.clear();

    selectedTab = 1;
    gameStatusStore.startGame().catch((e) => {
      console.error(e);
      launcherLogsStore.log("Failed to launch the game: " + e);
    });
  }
</script>

<main class="container">
  <div class="upper">
    <LauncherTabs bind:selectedTab />
  </div>
  <section class="lower">
    {#if $progressStore}
      <div class="progressbar-container">
        <ProgressBar info={$progressStore?.status} progress={$progressStore?.current / $progressStore?.total} --height="14px" --bar-color="#0078d7" />
      </div>
    {/if}

    <div class="lower-parts">
      <div class="left">
        <img src="gelcorp-title.png" alt="Logo de Gelcorp" />
      </div>
      <div class="right">
        <button class="main-btn" on:click={handleClick} disabled={isRunning}>
          {$gameStatusStore === GameStatus.Idle ? "Jugar" : $gameStatusStore === GameStatus.Downloading ? "Descargando" : "Jugando"}
        </button>
      </div>
    </div>
  </section>
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
    justify-content: stretch;
    align-content: stretch;
    overflow: hidden;
  }

  .main-btn {
    background-color: #2a632a;
    border: 2px solid #1c421c;
    font-size: 2.5rem;
    text-align: center;
    width: 177.6px;
    height: 66.4px;
    font-family: "Minecraft Ten";
    text-transform: uppercase;
    font-weight: 800;
    color: white;
  }

  .main-btn:disabled {
    background-color: #5c5e5c;
    border: 2px solid #3a3b3a;

    /* For states */
    font-size: 1.45rem;
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
