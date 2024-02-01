import { invoke } from "@tauri-apps/api/tauri";
import { readable } from "svelte/store";

export const totalMemoryStore = readable(0, (set) => {
  invoke("get_system_memory").then((memory) => set(memory as number));
});
