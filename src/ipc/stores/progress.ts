import { listen } from "@tauri-apps/api/event";
import { writable } from "svelte/store";

export type Progress = { status: string; current: number; total: number };

export const progressStore = writable<Progress | undefined>(undefined, (set) => {
  const unsubscriber = listen("update_progress", ({ payload }) => {
    set(payload as Progress | undefined);
  });
  return () => unsubscriber.then((unlisten) => unlisten());
});
