import { createSignal, For, Show, createMemo } from "solid-js";
import { TbOutlineClipboardList } from "solid-icons/tb";
import { Badge } from "../ui/Badge";
import { t } from "../../i18n";
import type { LogEntry, Source, JobStatus } from "../../store/types";
import styles from "./LogPanel.module.css";

interface Props {
  logs: LogEntry[];
  sources: Source[];
}

function formatDate(iso: string | null): string {
  if (!iso) return "—";
  return new Date(iso).toLocaleString("tr-TR", { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit", second: "2-digit" });
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

export function LogPanel(props: Props) {
  const [filterSource, setFilterSource] = createSignal("all");
  const [filterStatus, setFilterStatus] = createSignal("all");
  const [expandedId, setExpandedId] = createSignal<number | null>(null);

  const sourceMap = createMemo(() => {
    const m: Record<string, string> = {};
    for (const s of props.sources) m[s.id] = s.name;
    return m;
  });

  const filtered = createMemo(() =>
    props.logs.filter((log) => {
      if (filterSource() !== "all" && log.source_id !== filterSource()) return false;
      if (filterStatus() !== "all" && log.status !== filterStatus()) return false;
      return true;
    })
  );

  return (
    <div class={styles.root}>
      <div class={styles.filters}>
        <span class={styles.filterLabel}>{t("log_filter")}</span>
        <select class={styles.select} value={filterSource()} onChange={(e) => setFilterSource(e.currentTarget.value)}>
          <option value="all">{t("log_all_sources")}</option>
          <For each={props.sources}>{(s) => <option value={s.id}>{s.name}</option>}</For>
        </select>
        <select class={styles.select} value={filterStatus()} onChange={(e) => setFilterStatus(e.currentTarget.value)}>
          <option value="all">{t("log_all_statuses")}</option>
          <option value="Success">{t("status_success")}</option>
          <option value="Failed">{t("status_failed")}</option>
          <option value="Running">{t("status_running")}</option>
          <option value="Skipped">{t("status_skipped")}</option>
          <option value="Cancelled">{t("status_cancelled")}</option>
        </select>
        <span class={styles.count}>{filtered().length} {t("log_records")}</span>
      </div>

      <div class={styles.tableWrapper}>
        <Show when={filtered().length === 0}>
          <div class={styles.empty}>
            <div class={styles.emptyIcon}><TbOutlineClipboardList size={32} /></div>
            {t("log_empty")}
          </div>
        </Show>
        <Show when={filtered().length > 0}>
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
              </tr>
            </thead>
            <tbody>
              <For each={filtered()}>
                {(log) => {
                  const isExpanded = () => expandedId() === log.id;
                  return (
                    <>
                      <tr
                        class={`${styles.tr} ${log.error_message ? styles.trExpandable : ""} ${isExpanded() ? styles.trExpanded : ""}`}
                        onClick={() => log.error_message && setExpandedId(isExpanded() ? null : log.id)}
                      >
                        <td class={styles.td}><Badge variant={statusToVariant(log.status)}>{statusLabel(log.status)}</Badge></td>
                        <td class={styles.td}><span class={styles.sourceName}>{sourceMap()[log.source_id] ?? log.source_path}</span></td>
                        <td class={`${styles.td} ${styles.tdPath}`}>{log.destination_path}</td>
                        <td class={styles.td}>{triggerLabel(log.trigger)}</td>
                        <td class={styles.td}>{formatDate(log.started_at)}</td>
                        <td class={styles.td}>{formatDuration(log.started_at, log.ended_at)}</td>
                        <td class={styles.td}>{formatBytes(log.bytes_copied)}</td>
                      </tr>
                      <Show when={isExpanded() && log.error_message}>
                        <tr class={styles.errorRow}>
                          <td colSpan={7}>
                            <div class={styles.errorBox}>{log.error_message}</div>
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
