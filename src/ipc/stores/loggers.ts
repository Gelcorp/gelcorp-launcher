import { writable } from "svelte/store";
import { invoke } from "@tauri-apps/api";
import { listen } from "@tauri-apps/api/event";

/**
 *
 * @param listenerName Name of the tauri event to listen to
 * @param commandName Name of the tauri command to invoke the first time
 * @returns A svelte store with the logs
 */
function createLogsStore(listenerName: string, commandName?: string) {
  let { subscribe, set, update } = writable<string[]>([], (set) => {
    // Get logs from cache (if needed)
    if (commandName) {
      invoke(commandName).then((logs) => set(logs as string[]));
    }

    // Listen for logs
    let unsubscriber = listen(listenerName, (event) => {
      let logs = event.payload as string | string[];
      log(...logs);
    });

    // Register unsubscriber
    return () => unsubscriber.then((unlisten) => unlisten());
  });

  function log(...logs: string[]) {
    update((arr) => {
      arr = [...arr, ...logs];
      while (arr.length >= 1001) {
        arr.shift();
      }
      return arr;
    });
  }

  const clear = () => set([]);

  return { subscribe, log, clear };
}

export const gameLogsStore = createLogsStore("log"); // TODO: load cache
export const launcherLogsStore = createLogsStore("launcher_log", "get_launcher_logs_cache");
