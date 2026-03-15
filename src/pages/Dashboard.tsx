import { createMemo, For, Show } from "solid-js";
import { TbOutlineFolder, TbOutlineFile, TbOutlineArrowRight } from "solid-icons/tb";
import { store } from "../store";
import { api } from "../api/tauri";
import { Badge } from "../components/ui/Badge";
import { Button } from "../components/ui/Button";
import { t } from "../i18n";
import type { JobStatus } from "../store/types";
import styles from "./Dashboard.module.css";

function formatBytes(bytes: number | null): string {
  if (!bytes) return "0 B";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function timeAgo(iso: string | null): string {
  if (!iso) return "—";
  const diff = Date.now() - new Date(iso).getTime();
  const minutes = Math.floor(diff / 60000);
  if (minutes < 1) return "Az önce";
  if (minutes < 60) return `${minutes} dakika önce`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours} saat önce`;
  return `${Math.floor(hours / 24)} gün önce`;
}

function formatDate(iso: string | null): string {
  if (!iso) return "—";
  return new Date(iso).toLocaleString("tr-TR", { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });
}

function isToday(iso: string): boolean {
  const d = new Date(iso), now = new Date();
  return d.getFullYear() === now.getFullYear() && d.getMonth() === now.getMonth() && d.getDate() === now.getDate();
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

export function Dashboard() {
  const todayLogs = createMemo(() => store.logs.filter((l) => isToday(l.started_at)));
  const successToday = createMemo(() => todayLogs().filter((l) => l.status === "Success").length);
  const bytesToday = createMemo(() => todayLogs().reduce((sum, l) => sum + (l.bytes_copied ?? 0), 0));
  const lastError = createMemo(() => store.logs.find((l) => l.status === "Failed") ?? null);
  const recentLogs = createMemo(() => store.logs.slice(0, 10));
  const sourceMap = createMemo(() => {
    const m: Record<string, string> = {};
    for (const s of store.sources) m[s.id] = s.name;
    return m;
  });

  const handleRunSourceNow = async (sourceId: string) => {
    try { await api.jobs.runSourceNow(sourceId); } catch { /* handled via events */ }
  };

  return (
    <div class={styles.root}>
      {/* Stats */}
      <div class={styles.statsGrid}>
        <div class={styles.statCard}>
          <div class={styles.statLabel}>{t("dash_total_sources")}</div>
          <div class={styles.statValue}>{store.sources.length}</div>
          <div class={styles.statSub}>{store.sources.filter((s) => s.enabled).length} {t("dash_active")}</div>
        </div>
        <div class={styles.statCard}>
          <div class={styles.statLabel}>{t("dash_success_today")}</div>
          <div class={styles.statValueGreen}>{successToday()}</div>
          <div class={styles.statSub}>{t("dash_copies")}</div>
        </div>
        <div class={styles.statCard}>
          <div class={styles.statLabel}>{t("dash_copied_today")}</div>
          <div class={styles.statValueAccent}>{formatBytes(bytesToday())}</div>
          <div class={styles.statSub}>{t("dash_total_data")}</div>
        </div>
        <div class={styles.statCard}>
          <div class={styles.statLabel}>{t("dash_last_error")}</div>
          <Show when={lastError()} fallback={<div class={styles.statNoError}>{t("status_no_error")}</div>}>
            <div class={styles.statErrorName}>{sourceMap()[lastError()!.source_id] ?? t("dash_unknown")}</div>
            <div class={styles.statErrorTime}>{timeAgo(lastError()!.started_at)}</div>
          </Show>
        </div>
      </div>

      {/* Sources overview */}
      <div class={styles.card}>
        <div class={styles.cardTitle}>{t("dash_sources_card")}</div>
        <Show when={store.sources.length === 0}>
          <div class={styles.empty}>{t("dash_no_sources")}</div>
        </Show>
        <div class={styles.sourcesList}>
          <For each={store.sources}>
            {(source) => {
              const lastDest = source.destinations[0] ?? null;
              const isRunning = () => source.destinations.some((d) => store.runningJobs.has(d.id));
              return (
                <div class={styles.sourceRow}>
                  <span class={styles.sourceIcon}>{source.source_type === "Directory" ? <TbOutlineFolder size={16} /> : <TbOutlineFile size={16} />}</span>
                  <div class={styles.sourceInfo}>
                    <div class={styles.sourceName}>{source.name}</div>
                    <div class={styles.sourcePath}>{source.path}</div>
                  </div>
                  <div class={styles.sourceMeta}>
                    <div class={styles.metaItem}>
                      <div class={styles.metaItemLabel}>{t("dash_last_run")}</div>
                      <div class={styles.metaItemVal}>{timeAgo(lastDest?.last_run ?? null)}</div>
                    </div>
                    <div class={styles.metaItem}>
                      <div class={styles.metaItemLabel}>{t("dash_next_run")}</div>
                      <div class={styles.metaItemVal}>{formatDate(lastDest?.next_run ?? null)}</div>
                    </div>
                    <div class={styles.destCount}>{source.destinations.length} {t("dash_targets")}</div>
                    <Button variant="ghost" size="sm" onClick={() => handleRunSourceNow(source.id)} disabled={isRunning()}>
                      {isRunning() ? t("dash_running") : t("dash_run_now")}
                    </Button>
                  </div>
                </div>
              );
            }}
          </For>
        </div>
      </div>

      {/* Recent activity */}
      <div class={styles.card}>
        <div class={styles.cardTitle}>{t("dash_recent")}</div>
        <Show when={recentLogs().length === 0}>
          <div class={styles.empty}>{t("dash_no_logs")}</div>
        </Show>
        <div class={styles.activityList}>
          <For each={recentLogs()}>
            {(log) => (
              <div class={styles.activityRow}>
                <Badge variant={statusToVariant(log.status)}>{statusLabel(log.status)}</Badge>
                <span class={styles.activitySource}>{sourceMap()[log.source_id] ?? "—"}</span>
                <span class={styles.activityDest} style={{ display: "inline-flex", "align-items": "center", gap: "4px" }}><TbOutlineArrowRight size={13} /> {log.destination_path}</span>
                <span class={styles.activityTime}>{timeAgo(log.started_at)}</span>
              </div>
            )}
          </For>
        </div>
      </div>
    </div>
  );
}
