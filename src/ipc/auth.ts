import { invoke } from "@tauri-apps/api/tauri";

export const loginCracked = (username: string) => {
  return invoke("login_offline", { username });
};

export const loginMicrosoft = () => {
  return invoke("login_msa");
};
