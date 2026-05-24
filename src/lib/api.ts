import { invoke } from "@tauri-apps/api/core";
import type { Settings, StateSnapshot, VersionInfo } from "./types";

export const api = {
  getState: () => invoke<StateSnapshot>("get_state"),
  refreshNow: () => invoke<void>("refresh_now"),
  switchProxy: (group: string, node: string) =>
    invoke<void>("switch_proxy", { group, node }),
  testGroupDelay: (group: string) =>
    invoke<Record<string, number>>("test_group_delay", { group }),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (newSettings: Settings) =>
    invoke<void>("save_settings", { newSettings }),
  testConnection: (url: string, secret: string | null) =>
    invoke<VersionInfo>("test_connection", { url, secret }),
  hidePopup: () => invoke<void>("hide_popup"),
  openSettings: () => invoke<void>("open_settings"),
};
