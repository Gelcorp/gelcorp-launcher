import { invoke } from "@tauri-apps/api/tauri";
import { get, writable } from "svelte/store";

interface GameStatus {
  isRunning: boolean;
}

function createGameStatusStore() {
  const store = writable<GameStatus>({ isRunning: false });

  function setRunning(running: boolean) {
    store.update((config) => {
      config.isRunning = running;
      return config;
    });
  }

  async function startGame() {
    if (get(store).isRunning) return;
    setRunning(true);
    await invoke("start_game").finally(() => setRunning(false));
  }

  return { subscribe: store.subscribe, startGame };
}

export const gameStatusStore = createGameStatusStore();
