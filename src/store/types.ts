export type SourceType = "File" | "Directory";

export type DestinationType = "Local" | "S3" | "R2" | "Sftp" | "OneDrive" | "GoogleDrive" | "Dropbox" | "WebDav";

export interface S3Config {
  provider: "S3" | "R2";
  bucket: string;
  region: string;
  access_key_id: string;
  secret_access_key: string;
  endpoint_url?: string;
  prefix: string;
}

export interface WebDavConfig {
  url: string;
  username: string;
  password: string;
  root_path: string;
}

export interface OAuthConfig {
  provider: "onedrive" | "gdrive" | "dropbox";
  client_id: string;
  access_token: string;
  refresh_token: string;
  expires_at: number;
  folder_path: string;
}

export interface SftpConfig {
  host: string;
  port: number;
  username: string;
  auth_type: "password" | "key";
  password?: string;
  private_key?: string;
  remote_path: string;
}

export type ScheduleType =
  | { type: "Interval"; value: { minutes: number } }
  | { type: "Cron"; value: { expression: string } }
  | { type: "OnChange" }
  | { type: "Manual" };

export type VersionNaming = "Timestamp" | "Index" | "Overwrite";

export interface RetentionPolicy {
  max_versions: number;
  naming: VersionNaming;
}

export interface Destination {
  id: string;
  source_id: string;
  path: string;
  schedule: ScheduleType;
  retention: RetentionPolicy;
  enabled: boolean;
  last_run: string | null;
  last_status: JobStatus | null;
  next_run: string | null;
  exclusions: string[];
  incremental: boolean;
  destination_type: DestinationType;
  cloud_config: S3Config | null;
  sftp_config: SftpConfig | null;
  oauth_config: OAuthConfig | null;
  webdav_config: WebDavConfig | null;
  encrypt: boolean;
  level1_enabled: boolean;
  level1_schedule: ScheduleType | null;
  level1_type: string;
  level1_last_run: string | null;
  level1_next_run: string | null;
}

export interface Source {
  id: string;
  name: string;
  path: string;
  source_type: SourceType;
  enabled: boolean;
  created_at: string;
  destinations: Destination[];
}

export type JobStatus = "Running" | "Success" | "Verified" | "Failed" | "Skipped" | "Cancelled";
export type TriggerType = "Scheduled" | "OnChange" | "Manual" | "Verification";

export interface LogEntry {
  id: number;
  source_id: string;
  destination_id: string;
  source_path: string;
  destination_path: string;
  started_at: string;
  ended_at: string | null;
  status: JobStatus;
  bytes_copied: number | null;
  files_copied: number | null;
  error_message: string | null;
  trigger: TriggerType;
  checksum: string | null;
  backup_level: string | null;
  snapshot_id: string | null;
}

export interface AppSettings {
  run_on_startup: boolean;
  minimize_to_tray: boolean;
  theme: "dark" | "light" | "system";
  log_retention_days: number;
  language: "tr" | "en";
}

export interface DiskInfo {
  total_bytes: number;
  available_bytes: number;
  path: string;
}

export interface PreviewFile {
  rel_path: string;
  size_bytes: number;
  will_copy: boolean;
}

export interface BackupPreview {
  files: PreviewFile[];
  copy_count: number;
  copy_bytes: number;
  skip_count: number;
  total_count: number;
  source_name: string;
  dest_path: string;
  incremental: boolean;
}

export interface RestoreDryRunResult {
  mode: string;
  backup_path: string;
  restore_to: string;
  files_to_restore: number;
  bytes_to_restore: number;
  blocked: boolean;
  notes: string[];
  snapshot_id: string | null;
  backup_level: string | null;
  error_code: RestoreErrorCode | null;
}

export interface VerifyBackupResult {
  destination_id: string;
  source_id: string;
  snapshot_id: string;
  source_name: string;
  backup_level: string;
  chain_depth: number;
  files_checked: number;
  blocks_checked: number;
  total_bytes: number;
  snapshot_digest: string;
  verified_at: string;
  error_code: RestoreErrorCode | null;
}

export type RestoreErrorCode =
  | "blocked_path"
  | "missing_snapshot"
  | "wrong_password"
  | "chain_incomplete"
  | "io_failure"
  | "vault_locked"
  | "not_found"
  | "invalid_input"
  | "concurrency_conflict";

export interface CopyProgress {
  destination_id: string;
  files_done: number;
  files_total: number;
  bytes_done: number;
  bytes_total: number;
}

// ─── Vault ──────────────────────────────────────────────────────────────────

export type VaultEntryKind = "File" | "Directory";

export interface VaultEntry {
  id: string;
  name: string;
  parent_id: string | null;
  kind: VaultEntryKind;
  size: number | null;
  modified: string | null;
  nonce: string | null;
}

export interface VaultSummary {
  id: string;
  name: string;
  algorithm: string;
  vault_path: string;
  created_at: string;
  last_opened: string | null;
  unlocked: boolean;
}
