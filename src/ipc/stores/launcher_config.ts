import { invoke } from "@tauri-apps/api/tauri";
import { appWindow } from "@tauri-apps/api/window";
import { get, writable } from "svelte/store";

export type MsaAuthentication = {
  username: string;
  uuid: string;
  moj_token: string;
  moj_expiration_date: number;
  msa_access_token: string;
  msa_refresh_token: string;
  msa_expiration_date: string;
};
export type OfflineAuthentication = { username: string; uuid: string };

export type LauncherConfig = {
  authentication?: OfflineAuthentication | MsaAuthentication;
  memory_max: number;
  selected_options?: string[];
};

/**
 *
 * @returns A svelte store with the launcher config
 */
function createLauncherConfigStore() {
  let store = writable<LauncherConfig>({ memory_max: 1024 }, (set) => {
    // Ask for the config
    invoke("get_launcher_config").then((config) => set(config as LauncherConfig));

    // Listen for config updates
    let unsubscriber = appWindow.listen("launcher_config_update", (config) => {
      set(config.payload as LauncherConfig);
    });

    // Register unsubscriber
    return () => unsubscriber.then((unlisten) => unlisten());
  });

  function set(config: LauncherConfig) {
    invoke("set_launcher_config", { config });
    store.set(config);
  }

  function update(updater: (actual: LauncherConfig) => LauncherConfig) {
    let config = get(store);
    set(updater(config));
  }

  function logout() {
    update((config) => {
      config.authentication = undefined;
      return config;
    });
  }

  return {
    subscribe: store.subscribe,
    set,
    update,
    logout,
  };
}

export const launcherConfigStore = createLauncherConfigStore();
