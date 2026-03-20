import { createSignal, For, Show } from "solid-js";
import { TbOutlineFolder, TbOutlineFile, TbOutlineArchive, TbOutlineDotsVertical, TbOutlinePlayerPlay, TbOutlineEye, TbOutlinePencil, TbOutlineTrash, TbOutlineFolderOpen, TbOutlineLock, TbOutlineLockOpen } from "solid-icons/tb";
import { toast } from "solid-sonner";
import { Modal } from "../ui/Modal";
import { ConfirmDialog } from "../ui/ConfirmDialog";
import { Button } from "../ui/Button";
import { Badge } from "../ui/Badge";
import { Toggle } from "../ui/Toggle";
import { EditDestinationModal } from "./EditDestinationModal";
import { PreviewModal } from "./PreviewModal";
import { UpgradeModal } from "../../pages/License";
import { t } from "../../i18n";
import { api } from "../../api/tauri";
import { store } from "../../store";
import type { Source, Destination, JobStatus } from "../../store/types";
import styles from "./DestinationList.module.css";

interface Props {
  source: Source;
  runningJobs: Set<string>;
  onAddDestination: () => void;
  onRefresh: () => void;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function formatSchedule(s: Destination["schedule"]): string {
  if (s.type === "Interval") return `${s.value.minutes}dk`;
  if (s.type === "Cron") return `Cron: ${s.value.expression}`;
  if (s.type === "OnChange") return t("trigger_onchange");
  return t("trigger_manual");
}

function scheduleLabel(dest: Destination): string {
  const l0 = formatSchedule(dest.schedule);
  if (dest.level1_enabled && dest.level1_schedule) {
    const l1 = formatSchedule(dest.level1_schedule);
    const l1Type = dest.level1_type === "Differential" ? "Diff" : "Cum";
    return `L0: ${l0} | L1: ${l1} (${l1Type})`;
  }
  return l0;
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
  const map: Record<string, string> = {
    Success: t("status_success"),
    Failed: t("status_failed"),
    Running: t("status_running"),
    Skipped: t("status_skipped"),
    Cancelled: t("status_cancelled"),
  };
  return map[status] ?? status;
}

export function DestinationList(props: Props) {
  const [deletingId, setDeletingId] = createSignal<string | null>(null);
  const [pendingDeleteId, setPendingDeleteId] = createSignal<string | null>(null);
  const [runningId, setRunningId] = createSignal<string | null>(null);
  const [editingDest, setEditingDest] = createSignal<Destination | null>(null);
  const [previewDestId, setPreviewDestId] = createSignal<string | null>(null);
  const [showUpgrade, setShowUpgrade] = createSignal(false);
  const [openMenuId, setOpenMenuId] = createSignal<string | null>(null);
  const [decryptDest, setDecryptDest] = createSignal<Destination | null>(null);
  const [decryptFolder, setDecryptFolder] = createSignal("");
  const [decryptPassword, setDecryptPassword] = createSignal("");
  const [decrypting, setDecrypting] = createSignal(false);

  const isLicensed = () => store.licenseStatus === "valid";
  const atDestLimit = () => !isLicensed() && props.source.destinations.length >= 1;

  const handleAddDestination = () => {
    if (atDestLimit()) { setShowUpgrade(true); return; }
    props.onAddDestination();
  };

  const [runSubMenu, setRunSubMenu] = createSignal<string | null>(null);

  const handleRunNow = async (destId: string, level?: string) => {
    setOpenMenuId(null);
    setRunSubMenu(null);
    setRunningId(destId);
    try { await api.jobs.runNow(destId, level); }
    catch { /* handled via events */ }
    finally { setRunningId(null); props.onRefresh(); }
  };

  const handleDelete = (destId: string) => {
    setOpenMenuId(null);
    setPendingDeleteId(destId);
  };

  const confirmDelete = async () => {
    const destId = pendingDeleteId();
    if (!destId) return;
    setPendingDeleteId(null);
    setDeletingId(destId);
    try { await api.destinations.delete(destId); props.onRefresh(); }
    finally { setDeletingId(null); }
  };

  const handleToggleEnabled = async (dest: Destination) => {
    try { await api.destinations.update(dest.id, dest.path, dest.schedule, dest.retention, !dest.enabled); props.onRefresh(); }
    catch { /* ignore */ }
  };

  const cloudFolderUrl = (dest: Destination): string | null => {
    const type = dest.destination_type;
    const folderPath = dest.oauth_config?.folder_path ?? "";
    if (type === "Dropbox") {
      const p = folderPath.startsWith("/") ? folderPath.slice(1) : folderPath;
      return `https://www.dropbox.com/home/${encodeURIComponent(p)}`;
    }
    if (type === "GoogleDrive") return "https://drive.google.com";
    if (type === "OneDrive") return "https://onedrive.live.com";
    return null;
  };

  const handleOpenFolder = (dest: Destination) => {
    setOpenMenuId(null);
    const url = cloudFolderUrl(dest);
    if (url) {
      api.fs.openPath(url).catch(() => {});
    } else {
      api.fs.openPath(dest.path).catch(() => {});
    }
  };

  const handleDecryptOpen = (dest: Destination) => {
    setOpenMenuId(null);
    setDecryptDest(dest);
    setDecryptFolder(dest.path);
    setDecryptPassword("");
  };

  const handleDecryptSubmit = async () => {
    const dest = decryptDest();
    if (!dest || !decryptPassword().trim()) return;
    setDecrypting(true);
    try {
      const folder = await api.fs.pickDirectory() ?? dest.path;
      if (!folder) { setDecrypting(false); return; }
      const count = await api.destinations.decryptBackup(folder, decryptPassword().trim());
      toast.success(t("dest_decrypt_success").replace("{n}", String(count)));
      setDecryptDest(null);
    } catch (e: any) {
      toast.error(t("dest_decrypt_error") + ": " + String(e));
    } finally { setDecrypting(false); }
  };

  // Close menu when clicking outside
  const handleDocClick = (e: MouseEvent) => {
    const target = e.target as Element;
    if (!target.closest(`.${styles.menuWrapper}`)) {
      setOpenMenuId(null);
    }
  };

  return (
    <div class={styles.panel} onClick={handleDocClick}>
      <div class={styles.header}>
        <div class={styles.headerLeft}>
          <span class={styles.headerIcon} style={{ color: "var(--text-secondary)" }}>
            {props.source.source_type === "Directory" ? <TbOutlineFolder size={16} /> : <TbOutlineFile size={16} />}
          </span>
          <div>
            <div class={styles.sourceName}>{props.source.name}</div>
            <div class={styles.sourcePath}>{props.source.path}</div>
          </div>
        </div>
        <div class={styles.headerRight}>
          <button
            class={styles.openFolderBtn}
            title={t("open_src_folder")}
            onClick={() => { setOpenMenuId(null); api.fs.openPath(props.source.path).catch(() => {}); }}
          >
            <TbOutlineFolderOpen size={14} />
            <span>{t("open_src_folder")}</span>
          </button>
          <Button size="sm" onClick={handleAddDestination}>{t("dest_add")}</Button>
        </div>
      </div>

      <div class={styles.list}>
        <Show when={props.source.destinations.length === 0}>
          <div class={styles.empty}>
            <div class={styles.emptyIcon}><TbOutlineArchive size={32} /></div>
            {t("dest_empty")}
            <br />
            <span class={styles.emptyHint}>{t("dest_empty_hint")}</span>
          </div>
        </Show>
        <For each={props.source.destinations}>
          {(dest) => {
            const isRunning = () => props.runningJobs.has(dest.id) || runningId() === dest.id;
            const menuOpen = () => openMenuId() === dest.id;
            return (
              <div class={styles.card}>
                <Show when={store.copyProgress[dest.id]}>
                  {(progress) => {
                    const isBytes = () => progress().files_total <= 1 && progress().bytes_total > 0;
                    const pct = () => {
                      if (isBytes()) {
                        return progress().bytes_total > 0
                          ? Math.min(100, Math.round((progress().bytes_done / progress().bytes_total) * 100))
                          : 0;
                      }
                      return progress().files_total > 0
                        ? Math.round((progress().files_done / progress().files_total) * 100)
                        : 0;
                    };
                    const label = () => isBytes()
                      ? `${formatBytes(progress().bytes_done)} / ${formatBytes(progress().bytes_total)}`
                      : `${progress().files_done}/${progress().files_total} dosya`;
                    return (
                      <>
                        <div class={styles.progressBar}>
                          <div class={styles.progressFill} style={{ width: `${pct()}%` }} />
                        </div>
                        <div class={styles.progressText}>{label()}</div>
                      </>
                    );
                  }}
                </Show>
                <div class={styles.cardTop}>
                  <div class={styles.cardInfo}>
                    <div class={styles.destPath}>
                      {dest.path}
                      <Show when={dest.encrypt}>
                        <span title={t("dest_encrypt_label")} style={{ "margin-left": "6px", color: "var(--text-secondary)", display: "inline-flex", "vertical-align": "middle" }}>
                          <TbOutlineLock size={13} />
                        </span>
                      </Show>
                    </div>
                    <div class={styles.metaRow}>
                      <span class={styles.metaItem}>
                        <span class={styles.metaLabel}>{t("dest_schedule_label")} </span>{scheduleLabel(dest)}
                      </span>
                      <span class={styles.metaItem}>
                        <span class={styles.metaLabel}>L0 {t("dest_last_run")} </span>{formatDate(dest.last_run)}
                      </span>
                      <span class={styles.metaItem}>
                        <span class={styles.metaLabel}>L0 {t("dest_next_run")} </span>{formatDate(dest.next_run)}
                      </span>
                    </div>
                    <Show when={dest.level1_enabled}>
                      <div class={styles.metaRow}>
                        <span class={styles.metaItem}>
                          <span class={styles.metaLabel}>L1 {t("dest_last_run")} </span>{formatDate(dest.level1_last_run)}
                        </span>
                        <span class={styles.metaItem}>
                          <span class={styles.metaLabel}>L1 {t("dest_next_run")} </span>{formatDate(dest.level1_next_run)}
                        </span>
                      </div>
                    </Show>
                  </div>
                  <div class={styles.cardActions}>
                    <Show when={dest.last_status}>
                      <Badge variant={isRunning() ? "running" : statusToVariant(dest.last_status)}>
                        {isRunning() ? t("status_running") : statusLabel(dest.last_status)}
                      </Badge>
                    </Show>
                    <div class={styles.menuWrapper}>
                      <button
                        class={styles.menuBtn}
                        title="Menü"
                        onClick={(e) => { e.stopPropagation(); setOpenMenuId(menuOpen() ? null : dest.id); }}
                      >
                        <TbOutlineDotsVertical size={16} />
                      </button>
                      <Show when={menuOpen()}>
                        <div class={styles.dropdown} onClick={(e) => e.stopPropagation()}>
                          <div style={{ position: "relative" }}>
                            <button
                              class={styles.dropItem}
                              disabled={isRunning()}
                              onClick={() => setRunSubMenu(runSubMenu() === dest.id ? null : dest.id)}
                            >
                              <TbOutlinePlayerPlay size={14} />
                              {isRunning() ? t("btn_running") : t("btn_run_now")}
                              <span style={{ "margin-left": "auto", "font-size": "10px", opacity: 0.6 }}>▸</span>
                            </button>
                            <Show when={runSubMenu() === dest.id}>
                              <div class={styles.dropdown} style={{ position: "absolute", right: "100%", top: "0", "min-width": "180px", "z-index": "1001" }}>
                                <button class={styles.dropItem} onClick={() => handleRunNow(dest.id, "Level0")}>
                                  <TbOutlinePlayerPlay size={14} />
                                  Level 0 (Full)
                                </button>
                                <Show when={dest.level1_enabled}>
                                  <button class={styles.dropItem} onClick={() => handleRunNow(dest.id, dest.level1_type === "Differential" ? "Level1Differential" : "Level1Cumulative")}>
                                    <TbOutlinePlayerPlay size={14} />
                                    Level 1 ({dest.level1_type === "Differential" ? "Diff" : "Cum"})
                                  </button>
                                </Show>
                              </div>
                            </Show>
                          </div>
                          <button class={styles.dropItem} onClick={() => { setOpenMenuId(null); setPreviewDestId(dest.id); }}>
                            <TbOutlineEye size={14} />
                            {t("dest_preview")}
                          </button>
                          <button class={styles.dropItem} onClick={() => handleOpenFolder(dest)}>
                            <TbOutlineFolderOpen size={14} />
                            {t("open_dest_folder")}
                          </button>
                          <button class={styles.dropItem} onClick={() => { setOpenMenuId(null); setEditingDest(dest); }}>
                            <TbOutlinePencil size={14} />
                            {t("btn_edit")}
                          </button>
                          <Show when={dest.encrypt && dest.destination_type === "Local"}>
                            <button class={styles.dropItem} onClick={() => handleDecryptOpen(dest)}>
                              <TbOutlineLockOpen size={14} />
                              {t("dest_decrypt_btn")}
                            </button>
                          </Show>
                          <div class={styles.dropDivider} />
                          <button class={`${styles.dropItem} ${styles.dropItemDanger}`} onClick={() => handleDelete(dest.id)} disabled={deletingId() === dest.id}>
                            <TbOutlineTrash size={14} />
                            {t("btn_delete")}
                          </button>
                        </div>
                      </Show>
                    </div>
                  </div>
                </div>
                <div class={styles.cardFooter}>
                  <Toggle value={dest.enabled} onChange={() => handleToggleEnabled(dest)}
                    label={dest.enabled ? t("status_active") : t("status_disabled")} />
                </div>
              </div>
            );
          }}
        </For>
      </div>

      <EditDestinationModal
        open={editingDest() !== null}
        onClose={() => setEditingDest(null)}
        destination={editingDest()}
        onUpdated={() => { setEditingDest(null); props.onRefresh(); }}
      />

      <PreviewModal
        open={previewDestId() !== null}
        onClose={() => setPreviewDestId(null)}
        destinationId={previewDestId()}
      />

      <UpgradeModal
        open={showUpgrade()}
        onClose={() => setShowUpgrade(false)}
        sourceCount={0}
        subtitle={t("pro_dest_sub")}
      />

      <Modal
        open={decryptDest() !== null}
        onClose={() => setDecryptDest(null)}
        title={t("dest_decrypt_modal_title")}
        footer={
          <div style={{ display: "flex", gap: "8px", "justify-content": "flex-end" }}>
            <button onClick={() => setDecryptDest(null)} style={{ padding: "6px 12px", cursor: "pointer" }}>{t("btn_cancel")}</button>
            <button onClick={handleDecryptSubmit} disabled={decrypting()} style={{ padding: "6px 12px", cursor: "pointer" }}>
              {decrypting() ? "..." : t("dest_decrypt_btn")}
            </button>
          </div>
        }
      >
        <div style={{ display: "flex", "flex-direction": "column", gap: "12px" }}>
          <div>
            <label style={{ "font-size": "0.85rem", "margin-bottom": "4px", display: "block" }}>{t("dest_decrypt_password")}</label>
            <input
              type="password"
              value={decryptPassword()}
              onInput={(e) => setDecryptPassword(e.currentTarget.value)}
              onKeyDown={(e) => { if (e.key === "Enter") handleDecryptSubmit(); }}
              style={{ width: "100%", padding: "6px 8px", "box-sizing": "border-box" }}
              autofocus
            />
          </div>
          <div style={{ "font-size": "0.8rem", color: "var(--text-secondary)" }}>
            {t("dest_decrypt_folder")}: {decryptDest()?.path}
          </div>
        </div>
      </Modal>

      <ConfirmDialog
        open={pendingDeleteId() !== null}
        message={t("dest_delete_confirm")}
        onConfirm={confirmDelete}
        onCancel={() => setPendingDeleteId(null)}
      />
    </div>
  );
}
