import type { EventCallback } from "@tauri-apps/api/event";
import { appWindow } from "@tauri-apps/api/window";

type PayloadType = {
  "log": string | string[],
  "launcher_log": string | string[]
}

export const tauriListen = <T extends keyof PayloadType>(id: T, callback: EventCallback<PayloadType[T]>) => {
  let unlistenFn = appWindow.listen(id, callback);
  return () => unlistenFn.then(unlisten => unlisten()); // Synchronous unlisten fn
}