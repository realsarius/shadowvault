import { For } from "solid-js";
import { t } from "../../i18n";
import type { VaultEntry } from "../../store/types";
import { FileIcon } from "./FileIcon";
import { api } from "../../api/tauri";
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

// Module-level — reactive update'lerden etkilenmez
let activeDragId: string | null = null;
let activeDragOverRowEl: HTMLElement | null = null;

function closestRow(el: EventTarget | null): HTMLElement | null {
  return (el as HTMLElement | null)?.closest("[data-entry-id]") ?? null;
}

export function VaultEntryList(props: Props) {
  // ─── Drag source: sadece satır başlatır ───────────────────────────────────
  const handleDragStart = (e: DragEvent, entry: VaultEntry) => {
    activeDragId = entry.id;
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = "move";
      e.dataTransfer.setData("text/plain", entry.id);
    }
  };

  const handleDragEnd = () => {
    activeDragId = null;
    if (activeDragOverRowEl) {
      activeDragOverRowEl.setAttribute("data-dragover", "false");
      activeDragOverRowEl = null;
    }
  };

  // ─── Drop target: BODY div üzerinde (draggable değil → WebKit uyumlu) ─────
  const handleBodyDragOver = (e: DragEvent) => {
    if (!activeDragId) return;
    const row = closestRow(e.target);
    if (!row) return;
    const kind = row.getAttribute("data-entry-kind");
    const entryId = row.getAttribute("data-entry-id");
    if (kind !== "Directory" || entryId === activeDragId) return;

    e.preventDefault(); // drop'a izin ver
    e.stopPropagation();

    if (activeDragOverRowEl !== row) {
      activeDragOverRowEl?.setAttribute("data-dragover", "false");
      activeDragOverRowEl = row;
      row.setAttribute("data-dragover", "true");
    }
  };

  const handleBodyDragLeave = (e: DragEvent) => {
    // Body'den tamamen çıkınca highlight temizle
    const related = e.relatedTarget as Node | null;
    if (!related || !(e.currentTarget as HTMLElement).contains(related)) {
      activeDragOverRowEl?.setAttribute("data-dragover", "false");
      activeDragOverRowEl = null;
    }
  };

  const handleBodyDrop = async (e: DragEvent) => {
    if (!activeDragId) return;
    const row = closestRow(e.target);
    if (!row) return;
    const kind = row.getAttribute("data-entry-kind");
    const targetId = row.getAttribute("data-entry-id");

    if (kind !== "Directory" || !targetId || targetId === activeDragId) {
      activeDragOverRowEl?.setAttribute("data-dragover", "false");
      activeDragOverRowEl = null;
      return;
    }

    e.preventDefault();
    e.stopPropagation();
    row.setAttribute("data-dragover", "false");
    activeDragOverRowEl = null;

    const id = activeDragId;
    activeDragId = null;

    try {
      await api.vault.moveEntry(props.vaultId, id, targetId);
      props.onMoved();
    } catch {
      // ignore
    }
  };

  return (
    <div class={styles.list}>
      <div class={styles.header}>
        <span class={styles.colName}>{t("vault_name")}</span>
        <span class={styles.colSize}>{t("vault_size")}</span>
        <span class={styles.colDate}>Tarih</span>
      </div>
      {/* Body: draggable DEĞİL → WebKit'te drop olaylarını alır */}
      <div
        class={styles.body}
        onDragOver={handleBodyDragOver}
        onDragLeave={handleBodyDragLeave}
        onDrop={handleBodyDrop}
      >
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
