<script lang="ts">
  import LauncherTabs from "$/components/LauncherTabs.svelte";
  import ProgressBar from "$/components/ProgressBar.svelte";
  import { gameLogsStore, launcherLogsStore } from "$/ipc/stores/loggers";
  import { progressStore } from "$/ipc/stores/progress";
  import { GameStatus, gameStatusStore } from "$/ipc/stores/game_status";
  import { launcherConfigStore } from "$/ipc/stores/launcher_config";

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

  let buttonLabel: string;
  $: {
    if ($gameStatusStore === GameStatus.Idle) {
      buttonLabel = "Jugar";
    } else if ($gameStatusStore === GameStatus.Downloading) {
      buttonLabel = "Descargando";
    } else {
      buttonLabel = "Jugando";
    }
  }

  let { username, uuid } = $launcherConfigStore.authentication!;
  let { logout } = launcherConfigStore;
</script>

<main>
  <LauncherTabs bind:selectedTab />
  <footer>
    {#if $progressStore}
      <div class="progressbar-container">
        <ProgressBar info={$progressStore?.status} progress={$progressStore?.current / $progressStore?.total} --height="14px" --bar-color="#0078d7" />
      </div>
    {/if}

    <div class="lower">
      <img src="gelcorp-title.webp" alt="Logo de Gelcorp" />
      <button class="start-btn" on:click={handleClick} disabled={isRunning}>
        {buttonLabel}
      </button>
      <section>
        <p>Bienvenido, <b><img src="https://crafatar.com/avatars/{uuid}?overlay=true?size=64" alt="Avatar del jugador" /> {username}</b></p>
        <button on:click={logout}>Cambiar Usuario</button>
      </section>
    </div>
  </footer>
</main>

<style>
  main {
    display: grid;
    grid-template-rows: 1fr min-content;
    max-height: 100vh;
  }

  footer {
    display: grid;
    grid-template-rows: min-content 1fr;
    height: 100%;
  }

  footer .lower {
    box-sizing: border-box;

    background-image: url("/bedrock.png");
    background-size: 75px;
    image-rendering: pixelated;
    box-shadow: inset 0 -10px 80px -5px #000;

    height: 75px;
    padding: 5px 15px;
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: 1fr;
    align-items: center;
  }

  footer .lower img {
    width: 225px;
  }

  footer .lower section {
    justify-self: right;
    text-align: center;
    color: #fff;
  }

  footer .lower section p {
    margin: 0;
    font-size: 14.5px;
  }

  footer .lower section b {
    font-weight: 600;
  }

  footer .lower section img {
    width: 14.5px;
    height: auto;
  }

  .start-btn {
    justify-self: center;

    background-color: #2a632a;
    border: 2px solid #1c421c;

    font-family: "Minecraft Ten";
    font-size: 2.45rem;
    text-align: center;
    text-transform: uppercase;
    color: #fff;
    font-weight: bold;

    width: 220px;
    height: 60px;
  }

  .start-btn:disabled {
    background-color: #5c5e5c;
    border: 2px solid #3a3b3a;
    font-size: 28px;
  }

  .start-btn:hover:not(:disabled) {
    background-color: #225022;
  }

  .start-btn:active:not(:disabled) {
    background-color: #163316;
  }
</style>
