import { invoke } from "@tauri-apps/api/tauri";
import { readable } from "svelte/store";

export interface Optional {
  id: string;
  name: string;
  description: string;
  icon: string;
  incompatible_with?: string[];
}

export interface ModpackInfo {
  forgeVersion: string;
  minecraftVersion: string;
  optionals: Optional[];
}

export const modpackInfoStore = readable<ModpackInfo | undefined>(undefined, (set) => {
  invoke("fetch_modpack_info").then((info) => set(info as ModpackInfo));
});
