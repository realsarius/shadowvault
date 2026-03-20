import { createMemo, For, Show, createSignal } from "solid-js";
import { toast } from "solid-sonner";
import { TbOutlineClipboardList, TbOutlineRestore, TbOutlineShieldCheck, TbOutlineTrash } from "solid-icons/tb";
import { Badge } from "../ui/Badge";
import { api } from "../../api/tauri";
import { t, ti } from "../../i18n";
import type { JobStatus, LogEntry, Source } from "../../store/types";
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
  if (status === "Failed") return "error";
  if (status === "Running") return "running";
  if (status === "Skipped") return "warning";
  return "neutral";
}

function statusLabel(status: JobStatus): string {
  const map: Record<string, string> = {
    Success: t("status_success"),
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
  };
  return map[trigger] ?? trigger;
}

async function handleRestore(log: LogEntry) {
  const msg = ti("log_restore_confirm", { src: log.source_path, dst: log.destination_path });
  if (!confirm(msg)) return;
  try {
    await api.restore.backup(log.destination_path, log.source_path);
    toast.success(t("log_restore_success"));
  } catch (e: any) {
    toast.error(ti("log_restore_error", { err: e?.message ?? String(e) }));
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
                            <Show when={log.status === "Success"}>
                              <button class={styles.actionBtn} onClick={() => handleRestore(log)} title={t("log_restore")}>
                                <TbOutlineRestore size={14} />
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
