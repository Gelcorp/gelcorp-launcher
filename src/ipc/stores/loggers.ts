import { writable } from "svelte/store";
import { invoke } from "@tauri-apps/api";
import { listen } from "@tauri-apps/api/event";

export type Log = { level: string; message: string };

/**
 *
 * @param listenerName Name of the tauri event to listen to
 * @param commandName Name of the tauri command to invoke the first time
 * @returns A svelte store with the logs
 */
function createLogsStore(id: string) {
  let { subscribe, set, update } = writable<Log[]>([], (set) => {
    // Get logs from cache
    invoke("plugin:log-flusher|get_logs", { id }).then((logs) => {
      log(...(logs as string[]));
    });

    // Listen for logs
    let unsubscriber = listen(id, (event) => {
      let logs = event.payload as string | string[];
      log(...logs);
    });

    // Register unsubscriber
    return () => unsubscriber.then((unlisten) => unlisten());
  });

  function toLog(lastLevel: string | undefined, message: string) {
    const matches = /[\/\w]([A-Z]+)\]/g.exec(message) ?? [];
    const level = matches.length > 1 ? matches[1].toLowerCase() : lastLevel ?? "info";
    return { level, message } as Log;
  }

  function log(...logs: string[]) {
    update((arr) => {
      for (const log of logs) {
        arr.push(toLog(arr[arr.length - 1]?.level, log));
      }
      while (arr.length >= 1001) {
        arr.shift();
      }
      return arr;
    });
  }

  const clear = () => set([]);

  return { subscribe, log, clear };
}

export const gameLogsStore = createLogsStore("game_logs");
export const launcherLogsStore = createLogsStore("launcher_logs");
