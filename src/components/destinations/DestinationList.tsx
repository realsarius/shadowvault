import { createSignal, For, Show } from "solid-js";
import { Button } from "../ui/Button";
import { Badge } from "../ui/Badge";
import { Toggle } from "../ui/Toggle";
import { api } from "../../api/tauri";
import type { Source, Destination, JobStatus } from "../../store/types";
import styles from "./DestinationList.module.css";

interface Props {
  source: Source;
  runningJobs: Set<string>;
  onAddDestination: () => void;
  onRefresh: () => void;
}

function scheduleLabel(dest: Destination): string {
  const s = dest.schedule;
  if (s.type === "Interval") return `Her ${s.value.minutes} dakikada`;
  if (s.type === "Cron") return `Cron: ${s.value.expression}`;
  if (s.type === "OnChange") return "Değişince";
  return "Manuel";
}

function formatDate(iso: string | null): string {
  if (!iso) return "—";
  return new Date(iso).toLocaleString("tr-TR", { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });
}

function statusToVariant(status: JobStatus | null): "success" | "error" | "warning" | "running" | "neutral" {
  if (!status) return "neutral";
  if (status === "Success") return "success";
  if (status === "Failed") return "error";
  if (status === "Running") return "running";
  if (status === "Skipped") return "warning";
  return "neutral";
}

function statusLabel(status: JobStatus | null): string {
  if (!status) return "—";
  const map: Record<string, string> = { Success: "Başarılı", Failed: "Hata", Running: "Çalışıyor", Skipped: "Atlandı", Cancelled: "İptal" };
  return map[status] ?? status;
}

export function DestinationList(props: Props) {
  const [deletingId, setDeletingId] = createSignal<string | null>(null);
  const [runningId, setRunningId] = createSignal<string | null>(null);

  const handleRunNow = async (destId: string) => {
    setRunningId(destId);
    try { await api.jobs.runNow(destId); }
    catch { /* handled via events */ }
    finally { setRunningId(null); props.onRefresh(); }
  };

  const handleDelete = async (destId: string) => {
    if (!confirm("Bu hedefi silmek istediğinizden emin misiniz?")) return;
    setDeletingId(destId);
    try { await api.destinations.delete(destId); props.onRefresh(); }
    finally { setDeletingId(null); }
  };

  const handleToggleEnabled = async (dest: Destination) => {
    try { await api.destinations.update(dest.id, dest.path, dest.schedule, dest.retention, !dest.enabled); props.onRefresh(); }
    catch { /* ignore */ }
  };

  return (
    <div class={styles.panel}>
      <div class={styles.header}>
        <div class={styles.headerLeft}>
          <span class={styles.headerIcon}>{props.source.source_type === "Directory" ? "📁" : "📄"}</span>
          <div>
            <div class={styles.sourceName}>{props.source.name}</div>
            <div class={styles.sourcePath}>{props.source.path}</div>
          </div>
        </div>
        <Button size="sm" onClick={props.onAddDestination}>+ Hedef Ekle</Button>
      </div>

      <div class={styles.list}>
        <Show when={props.source.destinations.length === 0}>
          <div class={styles.empty}>
            <div class={styles.emptyIcon}>🗂️</div>
            Bu kaynak için henüz hedef yok.
            <br />
            <span class={styles.emptyHint}>Hedef ekleyerek yedeklemeyi başlatın.</span>
          </div>
        </Show>
        <For each={props.source.destinations}>
          {(dest) => {
            const isRunning = () => props.runningJobs.has(dest.id) || runningId() === dest.id;
            return (
              <div class={styles.card}>
                <div class={styles.cardTop}>
                  <div class={styles.cardInfo}>
                    <div class={styles.destPath}>{dest.path}</div>
                    <div class={styles.metaRow}>
                      <span class={styles.metaItem}>
                        <span class={styles.metaLabel}>Zamanlama: </span>{scheduleLabel(dest)}
                      </span>
                      <span class={styles.metaItem}>
                        <span class={styles.metaLabel}>Son çalışma: </span>{formatDate(dest.last_run)}
                      </span>
                      <span class={styles.metaItem}>
                        <span class={styles.metaLabel}>Sonraki: </span>{formatDate(dest.next_run)}
                      </span>
                    </div>
                  </div>
                  <div class={styles.cardActions}>
                    <Show when={dest.last_status}>
                      <Badge variant={isRunning() ? "running" : statusToVariant(dest.last_status)}>
                        {isRunning() ? "Çalışıyor" : statusLabel(dest.last_status)}
                      </Badge>
                    </Show>
                  </div>
                </div>
                <div class={styles.cardFooter}>
                  <Toggle value={dest.enabled} onChange={() => handleToggleEnabled(dest)}
                    label={dest.enabled ? "Aktif" : "Devre Dışı"} />
                  <div class={styles.footerButtons}>
                    <Button variant="ghost" size="sm" onClick={() => handleRunNow(dest.id)} disabled={isRunning()}>
                      {isRunning() ? "Çalışıyor..." : "▶ Şimdi Çalıştır"}
                    </Button>
                    <Button variant="danger" size="sm" onClick={() => handleDelete(dest.id)} disabled={deletingId() === dest.id}>
                      Sil
                    </Button>
                  </div>
                </div>
              </div>
            );
          }}
        </For>
      </div>
    </div>
  );
}
