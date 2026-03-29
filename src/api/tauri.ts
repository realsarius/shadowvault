import { invoke } from "@tauri-apps/api/core";
import type {
  Source,
  Destination,
  LogEntry,
  AppSettings,
  ScheduleType,
  RetentionPolicy,
  BackupPreview,
  DestinationType,
  S3Config,
  SftpConfig,
  OAuthConfig,
  WebDavConfig,
  VaultSummary,
  VaultEntry,
  RestoreDryRunResult,
  VerifyBackupResult,
} from "../store/types";

export interface LogQueryFilters {
  sourceId?: string;
  destinationId?: string;
  status?: string;
  startedAfter?: string;
  startedBefore?: string;
  searchText?: string;
  limit?: number;
  offset?: number;
}

export type LogExportFormat = "csv" | "json";

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
    add: (sourceId: string, path: string, schedule: ScheduleType, retention: RetentionPolicy, exclusions?: string[], incremental?: boolean, destinationType?: DestinationType, cloudConfig?: S3Config | null, sftpConfig?: SftpConfig | null, oauthConfig?: OAuthConfig | null, encrypt?: boolean, encryptPassword?: string | null, webdavConfig?: WebDavConfig | null, level1Enabled?: boolean, level1Schedule?: ScheduleType | null, level1Type?: string) =>
      invoke<Destination>("add_destination", { sourceId, path, schedule, retention, exclusions: exclusions ?? [], incremental: incremental ?? false, destinationType: destinationType ?? "Local", cloudConfig: cloudConfig ?? null, sftpConfig: sftpConfig ?? null, oauthConfig: oauthConfig ?? null, webdavConfig: webdavConfig ?? null, encrypt: encrypt ?? false, encryptPassword: encryptPassword ?? null, level1Enabled: level1Enabled ?? false, level1Schedule: level1Schedule ?? null, level1Type: level1Type ?? "Cumulative" }),
    update: (id: string, path: string, schedule: ScheduleType, retention: RetentionPolicy, enabled: boolean, exclusions?: string[], incremental?: boolean, destinationType?: DestinationType, cloudConfig?: S3Config | null, sftpConfig?: SftpConfig | null, oauthConfig?: OAuthConfig | null, encrypt?: boolean, encryptPassword?: string | null, webdavConfig?: WebDavConfig | null, level1Enabled?: boolean, level1Schedule?: ScheduleType | null, level1Type?: string) =>
      invoke<void>("update_destination", { id, path, schedule, retention, enabled, exclusions: exclusions ?? [], incremental: incremental ?? false, destinationType: destinationType ?? "Local", cloudConfig: cloudConfig ?? null, sftpConfig: sftpConfig ?? null, oauthConfig: oauthConfig ?? null, webdavConfig: webdavConfig ?? null, encrypt: encrypt ?? false, encryptPassword: encryptPassword ?? null, level1Enabled: level1Enabled ?? false, level1Schedule: level1Schedule ?? null, level1Type: level1Type ?? "Cumulative" }),
    delete: (id: string) => invoke<void>("delete_destination", { id }),
    decryptBackup: (folderPath: string, password: string) =>
      invoke<number>("decrypt_backup", { folderPath, password }),
  },
  cloud: {
    testConnection: (provider: string, bucket: string, region: string, accessKeyId: string, secretAccessKey: string, endpointUrl: string | null, prefix: string) =>
      invoke<void>("test_cloud_connection", { provider, bucket, region, accessKeyId, secretAccessKey, endpointUrl, prefix }),
    testSftpConnection: (host: string, port: number, username: string, authType: string, password: string | null, privateKey: string | null, remotePath: string) =>
      invoke<void>("test_sftp_connection", { host, port, username, authType, password, privateKey, remotePath }),
    testWebDavConnection: (url: string, username: string, password: string, rootPath: string) =>
      invoke<void>("test_webdav_connection", { url, username, password, rootPath }),
  },
  oauth: {
    runFlow: (provider: string, folderPath: string) =>
      invoke<OAuthConfig>("run_oauth_flow", { provider, folderPath }),
    testConnection: (oauthConfig: OAuthConfig) =>
      invoke<void>("test_oauth_connection", { oauthConfig }),
  },
  restore: {
    backup: (backupPath: string, restoreTo: string) =>
      invoke<void>("restore_backup", { backupPath, restoreTo }),
    blockBackup: (destinationPath: string, snapshotId: string, restoreTo: string, password?: string | null) =>
      invoke<void>("restore_block_backup", { destinationPath, snapshotId, restoreTo, password: password ?? null }),
    dryRun: (backupPath: string, restoreTo: string) =>
      invoke<RestoreDryRunResult>("restore_dry_run", { backupPath, restoreTo }),
    blockDryRun: (destinationPath: string, snapshotId: string, restoreTo: string) =>
      invoke<RestoreDryRunResult>("restore_block_dry_run", { destinationPath, snapshotId, restoreTo }),
    verify: (destinationId?: string | null, snapshotId?: string | null, password?: string | null) =>
      invoke<VerifyBackupResult>("verify_backup", {
        destinationId: destinationId ?? null,
        snapshotId: snapshotId ?? null,
        password: password ?? null,
      }),
  },
  jobs: {
    runNow: (destinationId: string, backupLevel?: string) => invoke<void>("run_now", { destinationId, backupLevel: backupLevel ?? null }),
    runSourceNow: (sourceId: string) => invoke<void>("run_source_now", { sourceId }),
    pauseAll: () => invoke<void>("pause_all"),
    resumeAll: () => invoke<void>("resume_all"),
  },
  logs: {
    get: (filters?: LogQueryFilters) =>
      invoke<LogEntry[]>("get_logs", (filters ?? {}) as Record<string, unknown>),
    count: (filters?: Omit<LogQueryFilters, "limit" | "offset">) =>
      invoke<number>("get_log_count", (filters ?? {}) as Record<string, unknown>),
    clearOld: (days: number) => invoke<number>("clear_old_logs", { olderThanDays: days }),
    deleteEntry: (logId: number) => invoke<number>("delete_log_entry", { logId }),
    clear: (filters?: Omit<LogQueryFilters, "limit" | "offset">) =>
      invoke<number>("clear_logs", (filters ?? {}) as Record<string, unknown>),
    export: (format: LogExportFormat, filters?: Omit<LogQueryFilters, "limit" | "offset">) =>
      invoke<string>("export_logs", { format, ...((filters ?? {}) as Record<string, unknown>) }),
  },
  fs: {
    pickDirectory: () => invoke<string | null>("pick_directory"),
    pickFile: () => invoke<string | null>("pick_file"),
    getDiskInfo: (path: string) => invoke<{ total_bytes: number; available_bytes: number; path: string }>("get_disk_info", { path }),
    checkPathType: (path: string) => invoke<string>("check_path_type", { path }),
    openPath: (path: string) => invoke<void>("open_path", { path }),
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
    deactivate: () => invoke<void>("deactivate_license"),
  },
  config: {
    export: () => invoke<string>("export_config"),
    import: () => invoke<{ sources_imported: number; destinations_imported: number; settings_applied: number }>("import_config"),
  },
  preview: {
    backup: (destinationId: string) => invoke<BackupPreview>("preview_backup", { destinationId }),
  },
  notifications: {
    sendTest: (to: string) => invoke<void>("send_test_email", { to }),
  },
  diagnostics: {
    export: () => invoke<string>("export_diagnostics"),
  },
  menu: {
    rebuild: (lang: string) => invoke<void>("rebuild_app_menu", { lang }),
  },
  vault: {
    create: (name: string, password: string, algorithm?: string) =>
      invoke<VaultSummary>("create_vault", { name, password, algorithm: algorithm ?? null }),
    list: () => invoke<VaultSummary[]>("list_vaults"),
    unlock: (vaultId: string, password: string) =>
      invoke<void>("unlock_vault", { vaultId, password }),
    lock: (vaultId: string) => invoke<void>("lock_vault", { vaultId }),
    listEntries: (vaultId: string, parentId?: string | null) =>
      invoke<VaultEntry[]>("list_entries", { vaultId, parentId: parentId ?? null }),
    importFile: (vaultId: string, srcPath: string, parentId?: string | null) =>
      invoke<VaultEntry>("import_file_cmd", { vaultId, srcPath, parentId: parentId ?? null }),
    importDirectory: (vaultId: string, srcPath: string, parentId?: string | null) =>
      invoke<VaultEntry>("import_directory_cmd", { vaultId, srcPath, parentId: parentId ?? null }),
    exportFile: (vaultId: string, entryId: string, destPath: string) =>
      invoke<void>("export_file_cmd", { vaultId, entryId, destPath }),
    openFile: (vaultId: string, entryId: string) =>
      invoke<void>("open_file_cmd", { vaultId, entryId }),
    renameEntry: (vaultId: string, entryId: string, newName: string) =>
      invoke<void>("rename_entry_cmd", { vaultId, entryId, newName }),
    moveEntry: (vaultId: string, entryId: string, newParentId?: string | null) =>
      invoke<void>("move_entry_cmd", { vaultId, entryId, newParentId: newParentId ?? null }),
    deleteEntry: (vaultId: string, entryId: string) =>
      invoke<void>("delete_entry_cmd", { vaultId, entryId }),
    createDirectory: (vaultId: string, name: string, parentId?: string | null) =>
      invoke<VaultEntry>("create_directory_cmd", { vaultId, name, parentId: parentId ?? null }),
    getThumbnail: (vaultId: string, entryId: string) =>
      invoke<string>("get_thumbnail", { vaultId, entryId }),
    deleteVault: (vaultId: string, password: string) =>
      invoke<void>("delete_vault", { vaultId, password }),
    exportVault: (vaultId: string, destDir: string) =>
      invoke<string>("export_vault_cmd", { vaultId, destDir }),
    importVault: (srcDir: string) =>
      invoke<VaultSummary>("import_vault_cmd", { srcDir }),
    changePassword: (vaultId: string, oldPassword: string, newPassword: string) =>
      invoke<void>("change_vault_password", { vaultId, oldPassword, newPassword }),
    getOpenFiles: (vaultId: string) =>
      invoke<{ entry_id: string; file_name: string; tmp_path: string }[]>("get_open_files", { vaultId }),
    syncAndLock: (vaultId: string, save: boolean) =>
      invoke<void>("sync_and_lock_vault", { vaultId, save }),
  },
};
