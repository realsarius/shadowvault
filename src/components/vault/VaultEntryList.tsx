import { For, onCleanup } from "solid-js";
import { t } from "../../i18n";
import type { VaultEntry } from "../../store/types";
import { FileIcon } from "./FileIcon";
import { api } from "../../api/tauri";
import { toast } from "solid-sonner";
import { formatDateOnly } from "../../utils/dateFormat";
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

type DropTarget =
  | { kind: "entry"; el: HTMLElement; entryId: string; entryKind: string }
  | { kind: "crumb"; el: HTMLElement; parentId: string | null };

function formatSize(bytes: number | null): string {
  if (bytes == null) return "—";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}


export function VaultEntryList(props: Props) {
  let ghost: HTMLDivElement | null = null;

  let dragEntry: VaultEntry | null = null;
  let dragStartX = 0;
  let dragStartY = 0;
  let isDragging = false;
  let currentTarget: HTMLElement | null = null;

  const findDropTarget = (x: number, y: number): DropTarget | null => {
    if (ghost) ghost.style.display = "none";
    const el = document.elementFromPoint(x, y) as HTMLElement | null;
    if (ghost) ghost.style.display = "";
    if (!el) return null;

    const entryEl = el.closest<HTMLElement>("[data-entry-id]");
    if (entryEl) {
      return {
        kind: "entry",
        el: entryEl,
        entryId: entryEl.getAttribute("data-entry-id")!,
        entryKind: entryEl.getAttribute("data-entry-kind") ?? "",
      };
    }

    const crumbEl = el.closest<HTMLElement>("[data-crumb-parent-id]");
    if (crumbEl) {
      const raw = crumbEl.getAttribute("data-crumb-parent-id");
      return { kind: "crumb", el: crumbEl, parentId: raw === "__root__" ? null : raw };
    }

    return null;
  };

  const createGhost = (entry: VaultEntry, x: number, y: number) => {
    ghost = document.createElement("div");
    ghost.style.cssText = `
      position: fixed;
      pointer-events: none;
      z-index: 9999;
      background: var(--bg-secondary, #1e1e2e);
      border: 1px solid var(--accent, #5a90f5);
      border-radius: 6px;
      padding: 4px 10px;
      font-size: 12px;
      color: var(--text-primary, #fff);
      opacity: 0.9;
      white-space: nowrap;
      box-shadow: 0 4px 16px rgba(0,0,0,0.4);
      transform: translate(-50%, -50%);
      left: ${x}px;
      top: ${y}px;
    `;
    ghost.textContent = entry.name;
    document.body.appendChild(ghost);
  };

  const removeGhost = () => {
    ghost?.remove();
    ghost = null;
  };

  const clearHighlight = () => {
    if (currentTarget) {
      currentTarget.setAttribute("data-dragover", "false");
      currentTarget = null;
    }
  };

  const onPointerMove = (e: PointerEvent) => {
    if (!dragEntry) return;

    const dx = e.clientX - dragStartX;
    const dy = e.clientY - dragStartY;

    if (!isDragging) {
      if (Math.sqrt(dx * dx + dy * dy) < 6) return;
      isDragging = true;
      createGhost(dragEntry, e.clientX, e.clientY);
    }

    if (ghost) {
      ghost.style.left = `${e.clientX}px`;
      ghost.style.top = `${e.clientY}px`;
    }

    const target = findDropTarget(e.clientX, e.clientY);

    let newTarget: HTMLElement | null = null;
    if (target?.kind === "entry") {
      const isDir = target.entryKind === "Directory";
      const isSelf = target.entryId === dragEntry.id;
      if (isDir && !isSelf) newTarget = target.el;
    } else if (target?.kind === "crumb") {
      newTarget = target.el;
    }

    if (newTarget !== currentTarget) {
      clearHighlight();
      if (newTarget) {
        currentTarget = newTarget;
        newTarget.setAttribute("data-dragover", "true");
      }
    }
  };

  const onPointerUp = async (e: PointerEvent) => {
    document.removeEventListener("pointermove", onPointerMove);
    document.removeEventListener("pointerup", onPointerUp);

    if (!isDragging || !dragEntry) {
      dragEntry = null;
      isDragging = false;
      removeGhost();
      return;
    }

    const target = findDropTarget(e.clientX, e.clientY);
    clearHighlight();
    removeGhost();

    const sourceEntry = dragEntry;
    dragEntry = null;
    isDragging = false;
    document.body.style.userSelect = "";
    (document.body.style as any).webkitUserSelect = "";
    window.getSelection()?.removeAllRanges();

    if (target?.kind === "entry") {
      const isDir = target.entryKind === "Directory";
      const isSelf = target.entryId === sourceEntry.id;
      if (isDir && !isSelf) {
        try {
          await api.vault.moveEntry(props.vaultId, sourceEntry.id, target.entryId);
          props.onMoved();
        } catch (err) {
          toast.error(String(err));
        }
      }
    } else if (target?.kind === "crumb") {
      try {
        await api.vault.moveEntry(props.vaultId, sourceEntry.id, target.parentId);
        props.onMoved();
      } catch (err) {
        toast.error(String(err));
      }
    }
  };

  const onPointerDown = (e: PointerEvent, entry: VaultEntry) => {
    if (e.button !== 0) return;
    e.stopPropagation();
    dragEntry = entry;
    dragStartX = e.clientX;
    dragStartY = e.clientY;
    isDragging = false;
    document.body.style.userSelect = "none";
    (document.body.style as any).webkitUserSelect = "none";

    document.addEventListener("pointermove", onPointerMove);
    document.addEventListener("pointerup", onPointerUp);
  };

  onCleanup(() => {
    document.removeEventListener("pointermove", onPointerMove);
    document.removeEventListener("pointerup", onPointerUp);
    document.body.style.userSelect = "";
    (document.body.style as any).webkitUserSelect = "";
    removeGhost();
  });

  return (
    <div class={styles.list}>
      <div class={styles.header}>
        <span class={styles.colName}>{t("vault_name")}</span>
        <span class={styles.colSize}>{t("vault_size")}</span>
        <span class={styles.colDate}>Tarih</span>
      </div>
      <div class={styles.body}>
        <For each={props.entries}>
          {(entry) => (
            <div
              class={styles.row}
              data-entry-id={entry.id}
              data-entry-kind={entry.kind}
              data-selected={String(props.selected.has(entry.id))}
              data-dragover="false"
              onClick={(e) => {
                if (!isDragging) props.onSelect(entry.id, e.metaKey || e.ctrlKey);
              }}
              onDblClick={() => { if (!isDragging) props.onDoubleClick(entry); }}
              onContextMenu={(e) => { e.preventDefault(); props.onContextMenu(e, entry); }}
              onPointerDown={(e) => onPointerDown(e, entry)}
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
              <span class={styles.colDate}>{formatDateOnly(entry.modified)}</span>
            </div>
          )}
        </For>
      </div>
    </div>
  );
}
