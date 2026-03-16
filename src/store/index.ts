import { createStore } from "solid-js/store";
import { listen } from "@tauri-apps/api/event";
import { api } from "../api/tauri";
import type { Source, LogEntry, AppSettings, CopyProgress } from "./types";

const LOG_PAGE_SIZE = 50;

interface AppStore {
  sources: Source[];
  logs: LogEntry[];
  logTotal: number;
  settings: AppSettings | null;
  isSchedulerPaused: boolean;
  runningJobs: Set<string>;
  isLoading: boolean;
  activeSourceId: string | null;
  activePage: "dashboard" | "sources" | "logs" | "settings" | "license" | "vault";
  watcherWarning: string | null;
  licenseStatus: "checking" | "valid" | "invalid";
  sidebarCollapsed: boolean;
  copyProgress: Record<string, CopyProgress>;
}

const [store, setStore] = createStore<AppStore>({
  sources: [],
  logs: [],
  logTotal: 0,
  settings: null,
  isSchedulerPaused: false,
  runningJobs: new Set(),
  isLoading: false,
  activeSourceId: null,
  activePage: "dashboard",
  watcherWarning: null,
  licenseStatus: "checking",
  sidebarCollapsed: false,
  copyProgress: {},
});

export async function refreshSources() {
  const sources = await api.sources.getAll();
  setStore("sources", sources);
}

export async function refreshLogs(sourceId?: string) {
  const [logs, total] = await Promise.all([
    api.logs.get({ sourceId, limit: LOG_PAGE_SIZE, offset: 0 }),
    api.logs.count(sourceId),
  ]);
  setStore("logs", logs);
  setStore("logTotal", total);
}

export async function loadMoreLogs(sourceId?: string) {
  const offset = store.logs.length;
  if (offset >= store.logTotal) return;
  const more = await api.logs.get({ sourceId, limit: LOG_PAGE_SIZE, offset });
  setStore("logs", [...store.logs, ...more]);
}

export async function loadSettings() {
  const settings = await api.settings.get();
  setStore("settings", settings);
}

export async function initStore() {
  setStore("isLoading", true);
  await Promise.all([refreshSources(), refreshLogs(), loadSettings()]);
  const collapsed = await api.settings.getValue("sidebar_collapsed").catch(() => null);
  if (collapsed !== null) {
    setStore("sidebarCollapsed", collapsed === "true");
  }
  setStore("isLoading", false);
}

// BUG-04: retry up to 3 times — AppState may not be ready immediately on startup
export async function initLicense(retries = 3): Promise<void> {
  setStore("licenseStatus", "checking");
  for (let attempt = 0; attempt < retries; attempt++) {
    try {
      const result = await api.license.validate();
      setStore("licenseStatus", result.status);
      return;
    } catch {
      if (attempt < retries - 1) {
        await new Promise((r) => setTimeout(r, 1500));
      }
    }
  }
  setStore("licenseStatus", "invalid");
}

export async function deactivateLicense(): Promise<{ success: boolean; error?: string }> {
  try {
    await api.license.deactivate();
    setStore("licenseStatus", "invalid");
    return { success: true };
  } catch (e: any) {
    return { success: false, error: e?.message ?? "Deaktivasyon başarısız." };
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

listen<CopyProgress>("copy-progress", (event) => {
  setStore("copyProgress", event.payload.destination_id, event.payload);
});

listen<{ destination_id: string; status: string }>("copy-completed", (event) => {
  const id = event.payload.destination_id;
  setStore("runningJobs", (set) => { const next = new Set(set); next.delete(id); return next; });
  setStore("copyProgress", (prev) => { const next = { ...prev }; delete next[id]; return next; });
  refreshSources();
  refreshLogs();
});

listen<{ destination_id: string }>("copy-error", (event) => {
  const id = event.payload.destination_id;
  setStore("runningJobs", (set) => { const next = new Set(set); next.delete(id); return next; });
  setStore("copyProgress", (prev) => { const next = { ...prev }; delete next[id]; return next; });
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
