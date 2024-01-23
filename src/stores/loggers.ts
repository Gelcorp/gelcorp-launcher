import { writable } from "svelte/store";
import { tauriListen } from "../tauriUtils";
import { invoke } from "@tauri-apps/api";

const launcherLogsWritable = writable([] as string[], () => {
  invoke("get_launcher_logs_cache").then((e) => {
    launcherLogsWritable.set(e as string[]);
  });
  
  return tauriListen("launcher_log", (event) => {
    let arr = [];
    if (typeof event.payload === 'string') {
      arr.push(event.payload);
    } else {
      arr = [...event.payload];
    }
    addLauncherLog(...arr);
  });
});

const addLauncherLog = (...logs: string[]) => {
  launcherLogsWritable.update((arr) => {
    arr = [...arr, ...logs];
    while (arr.length >= 1001) {
      arr.shift();
    }
    return arr;
  });
}

const gameLogsWritable = writable([] as string[], () => {
  return tauriListen("log", (event) => {
    let arr = [];
    if (typeof event.payload === 'string') {
      arr.push(event.payload);
    } else {
      arr = [...event.payload];
    }
    addGameLog(...arr);
  });
});

const addGameLog = (...logs: string[]) => {
  gameLogsWritable.update((arr) => {
    arr = [...arr, ...logs];
    while (arr.length >= 1001) {
      arr.shift();
    }
    return arr;
  });
}

export const launcherLogsStore = {
  subscribe: launcherLogsWritable.subscribe,
  log: addLauncherLog,
  clear: () => launcherLogsWritable.set([]),
};

export const gameLogsStore = {
  subscribe: gameLogsWritable.subscribe,
  log: addGameLog,
  clear: () => gameLogsWritable.set([]),
}