import { createMemo, For, Show } from "solid-js";
import { store } from "../store";
import { api } from "../api/tauri";
import { Badge } from "../components/ui/Badge";
import { Button } from "../components/ui/Button";
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
  const map: Record<string, string> = { Success: "Başarılı", Failed: "Hata", Running: "Çalışıyor", Skipped: "Atlandı", Cancelled: "İptal" };
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
          <div class={styles.statLabel}>Toplam Kaynak</div>
          <div class={styles.statValue}>{store.sources.length}</div>
          <div class={styles.statSub}>{store.sources.filter((s) => s.enabled).length} aktif</div>
        </div>
        <div class={styles.statCard}>
          <div class={styles.statLabel}>Bugün Başarılı</div>
          <div class={styles.statValueGreen}>{successToday()}</div>
          <div class={styles.statSub}>kopya</div>
        </div>
        <div class={styles.statCard}>
          <div class={styles.statLabel}>Bugün Kopyalanan</div>
          <div class={styles.statValueAccent}>{formatBytes(bytesToday())}</div>
          <div class={styles.statSub}>toplam veri</div>
        </div>
        <div class={styles.statCard}>
          <div class={styles.statLabel}>Son Hata</div>
          <Show when={lastError()} fallback={<div class={styles.statNoError}>Hata Yok</div>}>
            <div class={styles.statErrorName}>{sourceMap()[lastError()!.source_id] ?? "Bilinmeyen"}</div>
            <div class={styles.statErrorTime}>{timeAgo(lastError()!.started_at)}</div>
          </Show>
        </div>
      </div>

      {/* Sources overview */}
      <div class={styles.card}>
        <div class={styles.cardTitle}>Kaynaklar</div>
        <Show when={store.sources.length === 0}>
          <div class={styles.empty}>Henüz kaynak yok. Kaynaklar sayfasından ekleyin.</div>
        </Show>
        <div class={styles.sourcesList}>
          <For each={store.sources}>
            {(source) => {
              const lastDest = source.destinations[0] ?? null;
              const isRunning = () => source.destinations.some((d) => store.runningJobs.has(d.id));
              return (
                <div class={styles.sourceRow}>
                  <span class={styles.sourceIcon}>{source.source_type === "Directory" ? "📁" : "📄"}</span>
                  <div class={styles.sourceInfo}>
                    <div class={styles.sourceName}>{source.name}</div>
                    <div class={styles.sourcePath}>{source.path}</div>
                  </div>
                  <div class={styles.sourceMeta}>
                    <div class={styles.metaItem}>
                      <div class={styles.metaItemLabel}>Son çalışma</div>
                      <div class={styles.metaItemVal}>{timeAgo(lastDest?.last_run ?? null)}</div>
                    </div>
                    <div class={styles.metaItem}>
                      <div class={styles.metaItemLabel}>Sonraki</div>
                      <div class={styles.metaItemVal}>{formatDate(lastDest?.next_run ?? null)}</div>
                    </div>
                    <div class={styles.destCount}>{source.destinations.length} hedef</div>
                    <Button variant="ghost" size="sm" onClick={() => handleRunSourceNow(source.id)} disabled={isRunning()}>
                      {isRunning() ? "Çalışıyor..." : "▶ Çalıştır"}
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
        <div class={styles.cardTitle}>Son Aktivite</div>
        <Show when={recentLogs().length === 0}>
          <div class={styles.empty}>Henüz log kaydı yok.</div>
        </Show>
        <div class={styles.activityList}>
          <For each={recentLogs()}>
            {(log) => (
              <div class={styles.activityRow}>
                <Badge variant={statusToVariant(log.status)}>{statusLabel(log.status)}</Badge>
                <span class={styles.activitySource}>{sourceMap()[log.source_id] ?? "—"}</span>
                <span class={styles.activityDest}>→ {log.destination_path}</span>
                <span class={styles.activityTime}>{timeAgo(log.started_at)}</span>
              </div>
            )}
          </For>
        </div>
      </div>
    </div>
  );
}
