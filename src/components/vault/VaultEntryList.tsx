import { For } from "solid-js";
import { t } from "../../i18n";
import type { VaultEntry } from "../../store/types";
import { FileIcon } from "./FileIcon";
import { api } from "../../api/tauri";
import { toast } from "solid-sonner";
import styles from "./VaultEntryList.module.css";

interface Props {
  vaultId: string;
  entries: VaultEntry[];
  selected: Set<string>;
  onSelect: (id: string, multi: boolean) => void;
  onDoubleClick: (entry: VaultEntry) => void;
  onContextMenu: (e: MouseEvent, entry: VaultEntry) => void;
  onMoved: () => void;
}

function formatSize(bytes: number | null): string {
  if (bytes == null) return "—";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

function formatDate(iso: string | null): string {
  if (!iso) return "—";
  return new Date(iso).toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" });
}

let activeDragId: string | null = null;
let activeDragOverRowEl: HTMLElement | null = null;

export function VaultEntryList(props: Props) {
  let bodyRef: HTMLDivElement | undefined;

  const handleDragStart = (e: DragEvent, entry: VaultEntry) => {
    activeDragId = entry.id;
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = "move";
      e.dataTransfer.setData("text/plain", entry.id);
    }
    bodyRef?.setAttribute("data-dragging", "true");
  };

  const handleDragEnd = () => {
    activeDragId = null;
    if (activeDragOverRowEl) {
      activeDragOverRowEl.setAttribute("data-dragover", "false");
      activeDragOverRowEl = null;
    }
    bodyRef?.removeAttribute("data-dragging");
  };

  const handleOverlayDragEnter = (e: DragEvent, entry: VaultEntry) => {
    if (!activeDragId || activeDragId === entry.id) return;
    e.preventDefault();
    e.stopPropagation();
    const row = (e.currentTarget as HTMLElement).parentElement;
    if (row && activeDragOverRowEl !== row) {
      activeDragOverRowEl?.setAttribute("data-dragover", "false");
      activeDragOverRowEl = row;
      row.setAttribute("data-dragover", "true");
    }
  };

  const handleOverlayDragOver = (e: DragEvent, entry: VaultEntry) => {
    if (!activeDragId || activeDragId === entry.id) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.dataTransfer) e.dataTransfer.dropEffect = "move";
  };

  const handleOverlayDragLeave = (e: DragEvent) => {
    const related = e.relatedTarget as Node | null;
    const overlay = e.currentTarget as HTMLElement;
    if (!related || !overlay.contains(related)) {
      const row = overlay.parentElement;
      row?.setAttribute("data-dragover", "false");
      if (activeDragOverRowEl === row) activeDragOverRowEl = null;
    }
  };

  const handleOverlayDrop = async (e: DragEvent, entry: VaultEntry) => {
    e.preventDefault();
    e.stopPropagation();
    const row = (e.currentTarget as HTMLElement).parentElement;
    row?.setAttribute("data-dragover", "false");
    activeDragOverRowEl = null;
    bodyRef?.removeAttribute("data-dragging");

    const id = activeDragId;
    activeDragId = null;

    if (!id || id === entry.id) return;

    try {
      await api.vault.moveEntry(props.vaultId, id, entry.id);
      props.onMoved();
    } catch (err) {
      toast.error(String(err));
    }
  };

  return (
    <div class={styles.list}>
      <div class={styles.header}>
        <span class={styles.colName}>{t("vault_name")}</span>
        <span class={styles.colSize}>{t("vault_size")}</span>
        <span class={styles.colDate}>Tarih</span>
      </div>
      <div class={styles.body} ref={bodyRef}>
        <For each={props.entries}>
          {(entry) => (
            <div
              class={styles.row}
              data-entry-id={entry.id}
              data-entry-kind={entry.kind}
              data-selected={String(props.selected.has(entry.id))}
              data-dragover="false"
              draggable="true"
              onClick={(e) => props.onSelect(entry.id, e.metaKey || e.ctrlKey)}
              onDblClick={() => props.onDoubleClick(entry)}
              onContextMenu={(e) => { e.preventDefault(); props.onContextMenu(e, entry); }}
              onDragStart={(e) => handleDragStart(e, entry)}
              onDragEnd={handleDragEnd}
            >
              {entry.kind === "Directory" && (
                <div
                  class={styles.dropOverlay}
                  onDragEnter={(e) => handleOverlayDragEnter(e, entry)}
                  onDragOver={(e) => handleOverlayDragOver(e, entry)}
                  onDragLeave={handleOverlayDragLeave}
                  onDrop={(e) => handleOverlayDrop(e, entry)}
                />
              )}
              <span class={styles.colName}>
                <span class={styles.icon}>
                  <FileIcon name={entry.name} isDir={entry.kind === "Directory"} />
                </span>
                <span class={styles.name}>{entry.name}</span>
              </span>
              <span class={styles.colSize}>
                {entry.kind === "File" ? formatSize(entry.size) : "—"}
              </span>
              <span class={styles.colDate}>{formatDate(entry.modified)}</span>
            </div>
          )}
        </For>
      </div>
    </div>
  );
}
