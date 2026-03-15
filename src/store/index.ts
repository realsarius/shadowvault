import { createStore } from "solid-js/store";
import { listen } from "@tauri-apps/api/event";
import { api } from "../api/tauri";
import type { Source, LogEntry, AppSettings } from "./types";

interface AppStore {
  sources: Source[];
  logs: LogEntry[];
  settings: AppSettings | null;
  isSchedulerPaused: boolean;
  runningJobs: Set<string>;
  isLoading: boolean;
  activeSourceId: string | null;
  activePage: "dashboard" | "sources" | "logs" | "settings" | "license";
  watcherWarning: string | null;
  licenseStatus: "checking" | "valid" | "invalid";
  sidebarCollapsed: boolean;
}

const [store, setStore] = createStore<AppStore>({
  sources: [],
  logs: [],
  settings: null,
  isSchedulerPaused: false,
  runningJobs: new Set(),
  isLoading: false,
  activeSourceId: null,
  activePage: "dashboard",
  watcherWarning: null,
  licenseStatus: "checking",
  sidebarCollapsed: false,
});

export async function refreshSources() {
  const sources = await api.sources.getAll();
  setStore("sources", sources);
}

export async function refreshLogs(sourceId?: string) {
  const logs = await api.logs.get({ sourceId, limit: 200 });
  setStore("logs", logs);
}

export async function loadSettings() {
  const settings = await api.settings.get();
  setStore("settings", settings);
}

export async function initStore() {
  setStore("isLoading", true);
  await Promise.all([refreshSources(), refreshLogs(), loadSettings()]);
  // Restore sidebar collapsed state from DB
  const collapsed = await api.settings.getValue("sidebar_collapsed").catch(() => null);
  if (collapsed !== null) {
    setStore("sidebarCollapsed", collapsed === "true");
  }
  setStore("isLoading", false);
}

export async function initLicense(): Promise<void> {
  setStore("licenseStatus", "checking");
  try {
    const result = await api.license.validate();
    setStore("licenseStatus", result.status);
  } catch {
    setStore("licenseStatus", "invalid");
  }
}

export async function activateLicense(key: string): Promise<{ success: boolean; error?: string }> {
  try {
    const result = await api.license.activate(key);
    if (result.success) {
      setStore("licenseStatus", "valid");
    }
    return result;
  } catch (e: any) {
    return { success: false, error: e?.message ?? "Aktivasyon başarısız." };
  }
}

// Listen to Tauri events
listen<{ destination_id: string }>("copy-started", (event) => {
  setStore("runningJobs", (set) => new Set([...set, event.payload.destination_id]));
});

listen<{ destination_id: string; status: string }>("copy-completed", (event) => {
  setStore("runningJobs", (set) => {
    const next = new Set(set);
    next.delete(event.payload.destination_id);
    return next;
  });
  refreshSources();
  refreshLogs();
});

listen<{ destination_id: string }>("copy-error", (event) => {
  setStore("runningJobs", (set) => {
    const next = new Set(set);
    next.delete(event.payload.destination_id);
    return next;
  });
  refreshSources();
  refreshLogs();
});

listen<{ paused: boolean }>("scheduler-status", (event) => {
  setStore("isSchedulerPaused", event.payload.paused);
});

listen<{ message: string }>("watcher-warning", (event) => {
  setStore("watcherWarning", event.payload.message);
});

export { store, setStore };
