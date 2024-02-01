import { invoke } from "@tauri-apps/api/tauri";
import { appWindow } from "@tauri-apps/api/window";
import { get, writable } from "svelte/store";

export enum GameStatus {
  Idle = "Idle",
  Downloading = "Downloading",
  Playing = "Playing",
}

function createGameStatusStore() {
  const store = writable<GameStatus>(GameStatus.Idle, (set) => {
    // Ask for the status
    invoke("get_game_status").then((status) => set(status as GameStatus));

    // Listen for status updates
    let unsubscriber = appWindow.listen("game_status", (event) => {
      set(event.payload as GameStatus);
    });

    // Register unsubscriber
    return () => unsubscriber.then((unlisten) => unlisten());
  });

  async function startGame() {
    if (get(store) !== GameStatus.Idle) return;
    await invoke("start_game");
  }

  return { subscribe: store.subscribe, startGame };
}

export const gameStatusStore = createGameStatusStore();
