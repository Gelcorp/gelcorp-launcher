import type { EventCallback } from "@tauri-apps/api/event";
import { appWindow } from "@tauri-apps/api/window";

export type ClientEventPayload = {
  download: {
    info: string;
    count: number;
    total: number;
  },
  debug: string
}

type PayloadType = {
  "log": string | string[],
  "client_event": {
    type: keyof ClientEventPayload, data: ClientEventPayload[keyof ClientEventPayload]
  },
  "launcher_log": string | string[]
}

export const tauriListen = <T extends keyof PayloadType>(id: T, callback: EventCallback<PayloadType[T]>) => {
  let unlistenFn = appWindow.listen(id, callback);
  return () => unlistenFn.then(unlisten => unlisten()); // Synchronous unlisten fn
}