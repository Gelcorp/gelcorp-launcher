<script lang="ts">
  import { invoke } from "@tauri-apps/api/tauri";
  import { onMount } from "svelte";

  import ProgressBar from "./lib/ProgressBar.svelte";
  import { tauriListen } from "./tauriUtils";
  import { type ClientEventPayload } from "./tauriUtils";
  import ConsoleTab from "./ConsoleTab.svelte";
  import { gameLogsStore, launcherLogsStore } from "./stores/loggers";

  let showConsole = false;

  let progress: ClientEventPayload["download"] | undefined;

  let playButton: HTMLButtonElement;
  async function startGame() {
    if (playButton.disabled) return;
    playButton.disabled = true;
    gameLogsStore.clear();

    try {
      await invoke("start_game", {
        mcDir: "%appdata%/.minecraft_test_rust",
        gameVersion: "1.20.1",
        javaPath:
          "C:/Program Files/Eclipse Adoptium/jdk-17.0.6.10-hotspot/bin/java.exe",
      });
    } catch (e) {
      console.error(e);
      launcherLogsStore.log("Failed to launch the game: " + e);
    } finally {
      playButton.disabled = false;
    }
  }

  onMount(() => {
    return tauriListen("client_event", ({ payload }) => {
      if (payload.type === "download") {
        let data = payload.data as ClientEventPayload["download"];
        if (data.count === data.total) progress = undefined;
        else progress = data;
      }
    });
  });
</script>

<main class="container">
  <ConsoleTab bind:show={showConsole} />
  <div class="upper">
    <div class="buttons">
      <button on:click={() => (showConsole = true)}>Consola</button>
    </div>
    <!-- <header>
      <img class="title" src="gelcorp-title.png" alt="" />
    </header> -->
  </div>
  <div class="lower">
    <div class="left">
      <img src="gelcorp-title.png" alt="" />
    </div>
    <div class="right">
      <button class="console-btn" on:click={() => (showConsole = true)}>
      </button>
      <button class="main-btn" bind:this={playButton} on:click={startGame}
        >Jugar</button
      >
    </div>
    <!-- <div>
      <ProgressBar
        info={!progress
          ? ""
          : `${progress.info} (${progress.count}/${progress.total})`}
        progress={progress ? progress.count / progress.total : 0}
        --width="100%"
        --height="1rem"
      />
    </div> -->
  </div>
</main>

<style>
  main {
    background-image: url("/background.png");
    background-repeat: no-repeat;
    background-size: cover;
    image-rendering: optimizeQuality;

    display: grid;
    grid-template-rows: 1fr min-content; /* 70% minmax(min-content, auto);*/
    justify-content: stretch;
    justify-items: stretch;

    overflow: hidden;
  }

  .upper {
    display: grid;
    place-content: center;
    align-items: center;

    grid-template-rows: min-content 1fr;
    grid-template-columns: 1fr;
  }

  .upper .buttons {
    display: flex;
    justify-content: end;
  }

  .upper .buttons button {
    margin: 1px;
    padding: 2px 6px;
    font-size: 1.2rem;
    font-family: "Minecraft Ten";
  }

  /* .upper header {
    text-align: center;
    padding-bottom: 4rem;
  }

  .upper .title {
    width: 80%;
    max-width: 850px;
    user-select: none;
    pointer-events: none;
  } */

  .main-btn {
    background-color: #2a632a;
    border: 2px solid #1c421c;
    font-size: 2.5rem;
    padding: 0.2rem 1.8rem;
    font-family: "Minecraft Ten";
    text-transform: uppercase;
    font-weight: 800;

    /* margin-top: -50%; */
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
    background-image: url("/bedrock.png");
    /* background-color: #0000008a; */
    /* background-blend-mode: multiply; */
    background-size: contain;
    image-rendering: pixelated;

    box-shadow: inset 0 -10px 80px -5px black;

    border-top: 2px solid #000; /*rgba(63, 63, 63, 0.514);*/
    text-align: center;

    /* animation: ease-out 1.1s appear; */

    display: grid;
    grid-template-columns: 1fr 1fr;
    align-items: center;
  }

  .lower .right {
    display: flex;
    justify-content: end;
    padding: 0 20px;
  }

  .lower .right .console-btn {
    box-sizing: border-box;
    background-image: url("/icon/file-lines.svg");
    background-repeat: no-repeat;
    background-position: center;
    background-size: 1.3em;
    background-color: #535353;
    border: 2px solid #363636;
    
    width: 5em;
    aspect-ratio: 1 / 1;
  }

  .lower img {
    box-sizing: border-box;
    width: 80%;
    max-width: 260px;
    padding: 15px 5px;
  }

  /* @keyframes appear {
    0% {
      transform: translateY(150%);
    }

    100% {
      transform: translateY(0);
    }
  } */
</style>
