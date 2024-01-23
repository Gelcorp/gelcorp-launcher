import { tauri } from "@tauri-apps/api";
import { appWindow } from "@tauri-apps/api/window";
import { get, writable } from "svelte/store";

export type OfflineAuthentication = { username: string, uuid: string };
export type MsaAuthentication = { username: string, uuid: string, moj_token: string, moj_expiration_date: number, msa_access_token: string, msa_refresh_token: string, msa_expiration_date: string };

type LauncherConfig = {
  authentication?: OfflineAuthentication | MsaAuthentication,
  memory_max: number
}

const default_config: LauncherConfig = {
  memory_max: 1024
};

let store = writable(default_config, () => {
  tauri.invoke("get_launcher_config").then((config) => {
    store.set(config as LauncherConfig);
  })
  let unsubscribe = appWindow.listen("launcher_config_update", (config) => {
    store.set(config.payload as LauncherConfig);
  });
  return () => unsubscribe.then(() => { });
});

export const launcherConfigStore = {
  subscribe: store.subscribe,
  set(config: LauncherConfig) {
    store.set(config);
    tauri.invoke("set_launcher_config", { config });
  },
  update(callback: (actual: LauncherConfig) => LauncherConfig) {
    let config = get(store);
    config = callback(config);
    store.set(config);
    tauri.invoke("set_launcher_config", { config });
  },
  logout() {
    launcherConfigStore.update((config) => ({ ...config, authentication: undefined }));
  }
}