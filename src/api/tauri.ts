import { invoke } from "@tauri-apps/api/core";
import type { Source, Destination, LogEntry, AppSettings, ScheduleType, RetentionPolicy } from "../store/types";

export const api = {
  sources: {
    getAll: () => invoke<Source[]>("get_sources"),
    create: (name: string, path: string, source_type: "File" | "Directory") =>
      invoke<Source>("create_source", { name, path, sourceType: source_type }),
    update: (id: string, name: string, path: string, sourceType: "File" | "Directory", enabled: boolean) =>
      invoke<void>("update_source", { id, name, path, sourceType, enabled }),
    delete: (id: string) => invoke<void>("delete_source", { id }),
  },
  destinations: {
    add: (sourceId: string, path: string, schedule: ScheduleType, retention: RetentionPolicy, exclusions?: string[]) =>
      invoke<Destination>("add_destination", { sourceId, path, schedule, retention, exclusions: exclusions ?? [] }),
    update: (id: string, path: string, schedule: ScheduleType, retention: RetentionPolicy, enabled: boolean, exclusions?: string[]) =>
      invoke<void>("update_destination", { id, path, schedule, retention, enabled, exclusions: exclusions ?? [] }),
    delete: (id: string) => invoke<void>("delete_destination", { id }),
  },
  restore: {
    backup: (backupPath: string, restoreTo: string) =>
      invoke<void>("restore_backup", { backupPath, restoreTo }),
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
    checkPathType: (path: string) => invoke<string>("check_path_type", { path }),
  },
  settings: {
    get: () => invoke<AppSettings>("get_settings"),
    update: (settings: AppSettings) => invoke<void>("update_settings", { settings }),
    getValue: (key: string) => invoke<string | null>("get_setting_value", { key }),
    setValue: (key: string, value: string) => invoke<void>("set_setting_value", { key, value }),
  },
  updater: {
    check: () => invoke<{ available: boolean; version: string | null; body: string | null }>("check_update"),
    install: () => invoke<void>("install_update"),
  },
  license: {
    getHardwareId: () => invoke<string>("get_hardware_id"),
    activate: (key: string) => invoke<{ success: boolean; error?: string }>("activate_license", { key }),
    validate: () => invoke<{ status: "valid" | "invalid"; offline?: boolean }>("validate_license"),
    store: (key: string) => invoke<void>("store_license", { key }),
    getStored: () => invoke<string | null>("get_stored_license"),
    clear: () => invoke<void>("clear_license"),
  },
  menu: {
    rebuild: (lang: string) => invoke<void>("rebuild_app_menu", { lang }),
  },
};
