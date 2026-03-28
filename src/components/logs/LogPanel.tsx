import { createMemo, For, Show, createSignal } from "solid-js";
import { toast } from "solid-sonner";
import { TbOutlineClipboardList, TbOutlineRestore, TbOutlineShieldCheck, TbOutlineTrash } from "solid-icons/tb";
import { Badge } from "../ui/Badge";
import { api } from "../../api/tauri";
import { t, ti } from "../../i18n";
import type { JobStatus, LogEntry, Source } from "../../store/types";
import { parseCommandError, type RestoreErrorCode } from "../../utils/commandError";
import styles from "./LogPanel.module.css";

interface Props {
  logs: LogEntry[];
  sources: Source[];
  onDelete: (log: LogEntry) => Promise<void>;
}

function formatDate(iso: string | null): string {
  if (!iso) return "—";
  return new Date(iso).toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

function formatDuration(started: string, ended: string | null): string {
  if (!ended) return "—";
  const ms = new Date(ended).getTime() - new Date(started).getTime();
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  return `${Math.floor(ms / 60000)}m ${Math.floor((ms % 60000) / 1000)}s`;
}

function formatBytes(bytes: number | null): string {
  if (bytes === null || bytes === undefined) return "—";
  if (bytes === 0) return "0 B";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function statusToVariant(status: JobStatus): "success" | "error" | "warning" | "running" | "neutral" {
  if (status === "Success") return "success";
  if (status === "Verified") return "success";
  if (status === "Failed") return "error";
  if (status === "Running") return "running";
  if (status === "Skipped") return "warning";
  return "neutral";
}

function statusLabel(status: JobStatus): string {
  const map: Record<string, string> = {
    Success: t("status_success"),
    Verified: t("status_verified"),
    Failed: t("status_failed"),
    Running: t("status_running"),
    Skipped: t("status_skipped"),
    Cancelled: t("status_cancelled"),
  };
  return map[status] ?? status;
}

function triggerLabel(trigger: string): string {
  const map: Record<string, string> = {
    Scheduled: t("trigger_scheduled"),
    OnChange: t("trigger_onchange"),
    Manual: t("trigger_manual"),
    Verification: t("trigger_verification"),
  };
  return map[trigger] ?? trigger;
}

function actionHintByCode(code: RestoreErrorCode | null): string {
  if (code === "wrong_password") return t("err_action_check_password");
  if (code === "missing_snapshot") return t("err_action_select_snapshot");
  if (code === "chain_incomplete") return t("err_action_check_chain");
  if (code === "blocked_path") return t("err_action_retry_path");
  if (code === "io_failure") return t("err_action_retry");
  return "";
}

async function handleRestore(log: LogEntry) {
  let dryRunSummary = "";
  let dryRun: Awaited<ReturnType<typeof api.restore.dryRun>> | Awaited<ReturnType<typeof api.restore.blockDryRun>> | null = null;
  try {
    dryRun = log.snapshot_id
      ? await api.restore.blockDryRun(log.destination_path, log.snapshot_id, log.source_path)
      : await api.restore.dryRun(log.destination_path, log.source_path);

    dryRunSummary = `\n\n${t("log_restore_estimate")}: ${dryRun.files_to_restore} ${t("log_files_short")}, ${formatBytes(dryRun.bytes_to_restore)}`;
  } catch (e: any) {
    const parsed = parseCommandError(e);
    const hint = actionHintByCode(parsed.error_code);
    toast.error(
      ti("log_restore_dry_run_error", {
        err: hint ? `${parsed.message} - ${hint}` : parsed.message,
      }),
    );
    return;
  }

  if (!dryRun) return;
  if (dryRun.error_code === "blocked_path" || dryRun.blocked) {
    toast.error(`${t("log_restore_blocked")} ${t("err_action_retry_path")}`);
    return;
  }

  const msg = `${ti("log_restore_confirm", { src: log.source_path, dst: log.destination_path })}${dryRunSummary}`;
  if (!confirm(msg)) return;
  try {
    if (log.snapshot_id) {
      await api.restore.blockBackup(log.destination_path, log.snapshot_id, log.source_path, null);
    } else {
      await api.restore.backup(log.destination_path, log.source_path);
    }
    toast.success(t("log_restore_success"));
  } catch (e: any) {
    const parsed = parseCommandError(e);
    const hint = actionHintByCode(parsed.error_code);
    toast.error(
      ti("log_restore_error", {
        err: hint ? `${parsed.message} - ${hint}` : parsed.message,
      }),
    );
  }
}

async function handleVerify(log: LogEntry) {
  try {
    const result = await api.restore.verify(log.destination_id, log.snapshot_id, null);
    toast.success(ti("log_verify_success", { n: result.files_checked }));
  } catch (e: any) {
    const parsed = parseCommandError(e);
    const hint = actionHintByCode(parsed.error_code);
    toast.error(
      ti("log_verify_error", {
        err: hint ? `${parsed.message} - ${hint}` : parsed.message,
      }),
    );
  }
}

export function LogPanel(props: Props) {
  const [expandedId, setExpandedId] = createSignal<number | null>(null);

  const sourceMap = createMemo(() => {
    const m: Record<string, string> = {};
    for (const s of props.sources) m[s.id] = s.name;
    return m;
  });

  return (
    <div class={styles.root}>
      <div class={styles.tableWrapper}>
        <Show when={props.logs.length === 0}>
          <div class={styles.empty}>
            <div class={styles.emptyIcon}><TbOutlineClipboardList size={32} /></div>
            {t("log_empty")}
          </div>
        </Show>
        <Show when={props.logs.length > 0}>
          <table class={styles.table}>
            <thead>
              <tr>
                <th class={styles.th}>{t("log_col_status")}</th>
                <th class={styles.th}>{t("log_col_source")}</th>
                <th class={styles.th}>{t("log_col_dest")}</th>
                <th class={styles.th}>{t("log_col_trigger")}</th>
                <th class={styles.th}>{t("log_col_start")}</th>
                <th class={styles.th}>{t("log_col_duration")}</th>
                <th class={styles.th}>{t("log_col_data")}</th>
                <th class={styles.th}></th>
              </tr>
            </thead>
            <tbody>
              <For each={props.logs}>
                {(log) => {
                  const isExpanded = () => expandedId() === log.id;
                  const isExpandable = () => !!(log.error_message || log.checksum);
                  const canRestore = () =>
                    (log.status === "Success" || log.status === "Verified") &&
                    (!!log.snapshot_id || !log.destination_path.includes("://"));
                  return (
                    <>
                      <tr
                        class={`${styles.tr} ${isExpandable() ? styles.trExpandable : ""} ${isExpanded() ? styles.trExpanded : ""}`}
                        onClick={() => isExpandable() && setExpandedId(isExpanded() ? null : log.id)}
                      >
                        <td class={styles.td}>
                          <Badge variant={statusToVariant(log.status)}>{statusLabel(log.status)}</Badge>
                          <Show when={log.backup_level}>
                            <Badge variant="neutral">{log.backup_level === "Level0" ? "L0" : log.backup_level === "Level1Cumulative" ? "L1C" : log.backup_level === "Level1Differential" ? "L1D" : log.backup_level}</Badge>
                          </Show>
                        </td>
                        <td class={styles.td}><span class={styles.sourceName}>{sourceMap()[log.source_id] ?? log.source_path}</span></td>
                        <td class={`${styles.td} ${styles.tdPath}`}>{log.destination_path}</td>
                        <td class={styles.td}>{triggerLabel(log.trigger)}</td>
                        <td class={styles.td}>{formatDate(log.started_at)}</td>
                        <td class={styles.td}>{formatDuration(log.started_at, log.ended_at)}</td>
                        <td class={styles.td}>
                          {formatBytes(log.bytes_copied)}
                          <Show when={log.files_copied !== null}>
                            <span class={styles.filesBadge}>{log.files_copied} {t("log_files_short")}</span>
                          </Show>
                          <Show when={log.checksum}>
                            <span class={styles.checksumBadge} title={log.checksum!}>
                              <TbOutlineShieldCheck size={12} /> {t("log_checksum_ok")}
                            </span>
                          </Show>
                        </td>
                        <td class={styles.td} onClick={(e) => e.stopPropagation()}>
                          <div class={styles.actions}>
                            <Show when={canRestore()}>
                              <button class={styles.actionBtn} onClick={() => handleRestore(log)} title={t("log_restore")}>
                                <TbOutlineRestore size={14} />
                              </button>
                            </Show>
                            <Show when={!!log.snapshot_id}>
                              <button class={styles.actionBtn} onClick={() => handleVerify(log)} title={t("log_verify")}>
                                <TbOutlineShieldCheck size={14} />
                              </button>
                            </Show>
                            <button class={`${styles.actionBtn} ${styles.deleteBtn}`} onClick={() => props.onDelete(log)} title={t("log_delete_one")}>
                              <TbOutlineTrash size={14} />
                            </button>
                          </div>
                        </td>
                      </tr>
                      <Show when={isExpanded()}>
                        <tr class={styles.errorRow}>
                          <td colSpan={8}>
                            <Show when={log.error_message}>
                              <div class={styles.errorBox}>{log.error_message}</div>
                            </Show>
                            <Show when={log.checksum}>
                              <div class={styles.checksumBox}>
                                <TbOutlineShieldCheck size={13} /> {log.checksum}
                              </div>
                            </Show>
                          </td>
                        </tr>
                      </Show>
                    </>
                  );
                }}
              </For>
            </tbody>
          </table>
        </Show>
      </div>
    </div>
  );
}
