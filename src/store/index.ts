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
  activePage: "dashboard" | "sources" | "logs" | "settings";
  watcherWarning: string | null;
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
  setStore("isLoading", false);
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
