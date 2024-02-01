import { invoke } from "@tauri-apps/api";

export const launchGame = () => invoke("start_game");
