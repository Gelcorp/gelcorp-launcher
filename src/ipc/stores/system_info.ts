import { invoke } from "@tauri-apps/api/core";
import { readable } from "svelte/store";

export const totalMemoryStore = readable(0, (set) => {
  invoke("get_system_memory").then((memory) => set(memory as number));
});

export const defaultJREFlags = readable({} as { [key: string]: string }, (set) => {
  invoke("get_default_jre_flags").then((flags) => set(flags as { [key: string]: string }));
});
