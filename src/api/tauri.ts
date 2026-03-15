import { invoke } from "@tauri-apps/api/core";
import type { Source, Destination, LogEntry, AppSettings, DiskInfo, ScheduleType, RetentionPolicy } from "../store/types";

export const api = {
  sources: {
    getAll: () => invoke<Source[]>("get_sources"),
    create: (name: string, path: string, source_type: "File" | "Directory") =>
      invoke<Source>("create_source", { name, path, sourceType: source_type }),
    update: (id: string, name: string, enabled: boolean) =>
      invoke<void>("update_source", { id, name, enabled }),
    delete: (id: string) => invoke<void>("delete_source", { id }),
  },
  destinations: {
    add: (sourceId: string, path: string, schedule: ScheduleType, retention: RetentionPolicy) =>
      invoke<Destination>("add_destination", { sourceId, path, schedule, retention }),
    update: (id: string, path: string, schedule: ScheduleType, retention: RetentionPolicy, enabled: boolean) =>
      invoke<void>("update_destination", { id, path, schedule, retention, enabled }),
    delete: (id: string) => invoke<void>("delete_destination", { id }),
  },
  jobs: {
    runNow: (destinationId: string) => invoke<void>("run_now", { destinationId }),
    runSourceNow: (sourceId: string) => invoke<void>("run_source_now", { sourceId }),
    pauseAll: () => invoke<void>("pause_all"),
    resumeAll: () => invoke<void>("resume_all"),
  },
  logs: {
    get: (filters?: { sourceId?: string; destinationId?: string; status?: string; limit?: number; offset?: number }) =>
      invoke<LogEntry[]>("get_logs", filters ?? {}),
    count: (sourceId?: string) => invoke<number>("get_log_count", { sourceId: sourceId ?? null }),
    clearOld: (days: number) => invoke<number>("clear_old_logs", { olderThanDays: days }),
  },
  fs: {
    pickDirectory: () => invoke<string | null>("pick_directory"),
    pickFile: () => invoke<string | null>("pick_file"),
    getDiskInfo: (path: string) => invoke<{ total_bytes: number; available_bytes: number; path: string }>("get_disk_info", { path }),
  },
  settings: {
    get: () => invoke<AppSettings>("get_settings"),
    update: (settings: AppSettings) => invoke<void>("update_settings", { settings }),
  },
  updater: {
    check: () => invoke<{ available: boolean; version: string | null; body: string | null }>("check_update"),
    install: () => invoke<void>("install_update"),
  },
  license: {
    getHardwareId: () => invoke<string>("get_hardware_id"),
    store: (key: string) => invoke<void>("store_license", { key }),
    getStored: () => invoke<string | null>("get_stored_license"),
    clear: () => invoke<void>("clear_license"),
  },
  menu: {
    rebuild: (lang: string) => invoke<void>("rebuild_app_menu", { lang }),
  },
};
