import { createStore } from "solid-js/store";
import { listen } from "@tauri-apps/api/event";
import { api } from "../api/tauri";
import type { Source, LogEntry, AppSettings } from "./types";

const LICENSE_API = "https://license.berkansozer.com";

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
  setStore("isLoading", false);
}

export async function initLicense(): Promise<void> {
  setStore("licenseStatus", "checking");
  try {
    const storedKey = await api.license.getStored();
    if (!storedKey) {
      setStore("licenseStatus", "invalid");
      return;
    }
    const hardwareId = await api.license.getHardwareId();
    const res = await fetch(`${LICENSE_API}/licenses/validate`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ key: storedKey, hardware_id: hardwareId }),
    });
    const data = await res.json();
    setStore("licenseStatus", data.valid ? "valid" : "invalid");
  } catch {
    // Ağ hatası: saklı key varsa geçerli say (offline toleransı)
    const storedKey = await api.license.getStored().catch(() => null);
    setStore("licenseStatus", storedKey ? "valid" : "invalid");
  }
}

export async function activateLicense(key: string): Promise<{ success: boolean; error?: string }> {
  try {
    const hardwareId = await api.license.getHardwareId();
    const res = await fetch(`${LICENSE_API}/licenses/activate`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ key, hardware_id: hardwareId }),
    });
    const data = await res.json();
    if (data.valid || data.activated_at) {
      await api.license.store(key);
      setStore("licenseStatus", "valid");
      return { success: true };
    }
    return { success: false, error: data.message ?? "Geçersiz lisans anahtarı." };
  } catch {
    return { success: false, error: "Sunucuya bağlanılamadı. İnternet bağlantınızı kontrol edin." };
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

export { store, setStore, LICENSE_API };
