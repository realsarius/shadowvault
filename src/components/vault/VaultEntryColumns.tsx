import { For, createSignal, createEffect, onMount, onCleanup } from "solid-js";
import { TbOutlineChevronRight } from "solid-icons/tb";
import type { VaultEntry } from "../../store/types";
import { FileIcon } from "./FileIcon";
import { api } from "../../api/tauri";
import { toast } from "solid-sonner";
import styles from "./VaultEntryColumns.module.css";

interface Column {
  parentId: string | null;
  entries: VaultEntry[];
  selectedId: string | null;
}

type DropTarget =
  | { kind: "entry"; el: HTMLElement; entryId: string; entryKind: string; colIndex: number }
  | { kind: "crumb"; el: HTMLElement; parentId: string | null };

interface Props {
  vaultId: string;
  onFileOpen: (entry: VaultEntry) => void;
  onContextMenu: (e: MouseEvent, entry: VaultEntry) => void;
  onNavigate: (crumbs: VaultEntry[]) => void;
  reloadKey?: number;
  navigateRequest?: { entry: VaultEntry; requestId: number } | null;
  onNavigateRequestHandled?: () => void;
  onReady?: (reload: () => void) => void;
}

const DEFAULT_COLUMN_WIDTH = 210;
const MIN_COLUMN_WIDTH = 170;
const MAX_COLUMN_WIDTH = 620;

export function VaultEntryColumns(props: Props) {
  const [columns, setColumns] = createSignal<Column[]>([]);
  const [columnWidths, setColumnWidths] = createSignal<number[]>([]);
  let containerRef: HTMLDivElement | undefined;

  let ghost: HTMLDivElement | null = null;
  let dragEntry: VaultEntry | null = null;
  let dragStartX = 0;
  let dragStartY = 0;
  let isDragging = false;
  let currentTarget: HTMLElement | null = null;
  let lastReloadKey: number | undefined;
  let lastNavigateRequestId = 0;

  let resizingColIndex: number | null = null;
  let resizeStartX = 0;
  let resizeStartWidth = DEFAULT_COLUMN_WIDTH;

  const loadColumn = async (index: number, parentId: string | null) => {
    try {
      const entries = await api.vault.listEntries(props.vaultId, parentId);
      setColumns((prev) => {
        const next = [...prev.slice(0, index)];
        const existing = prev[index];
        next.push({ parentId, entries, selectedId: existing?.selectedId ?? null });
        return next;
      });
    } catch (err) {
      toast.error(String(err));
    }
  };

  const reloadAllColumns = async () => {
    const cols = columns();
    if (cols.length === 0) {
      await loadColumn(0, null);
      return;
    }
    for (let i = 0; i < cols.length; i++) {
      await loadColumn(i, cols[i].parentId);
    }
  };

  onMount(() => {
    props.onReady?.(() => { void reloadAllColumns(); });
    void loadColumn(0, null);
  });

  createEffect(() => {
    const len = columns().length;
    setColumnWidths((prev) => {
      if (prev.length === len) return prev;
      const next = prev.slice(0, len);
      while (next.length < len) next.push(DEFAULT_COLUMN_WIDTH);
      return next;
    });
  });

  createEffect(() => {
    const key = props.reloadKey;
    if (key === undefined) return;
    if (lastReloadKey === undefined) {
      lastReloadKey = key;
      return;
    }
    if (key === lastReloadKey) return;
    lastReloadKey = key;
    void reloadAllColumns();
  });

  const buildCrumbs = (cols: Column[]): VaultEntry[] => {
    const crumbs: VaultEntry[] = [];
    for (const col of cols) {
      if (!col.selectedId) break;
      const entry = col.entries.find((e) => e.id === col.selectedId);
      if (!entry || entry.kind !== "Directory") break;
      crumbs.push(entry);
    }
    return crumbs;
  };

  const openDirectoryAtColumn = async (colIndex: number, entry: VaultEntry) => {
    setColumns((prev) => {
      const next = prev.map((col, i) =>
        i === colIndex ? { ...col, selectedId: entry.id } : col
      );
      return next.slice(0, colIndex + 1);
    });
    await loadColumn(colIndex + 1, entry.id);
    props.onNavigate(buildCrumbs(columns()));
    setTimeout(() => {
      containerRef?.scrollTo({ left: containerRef.scrollWidth, behavior: "smooth" });
    }, 50);
  };

  const openById = async (entry: VaultEntry) => {
    if (entry.kind === "File") {
      props.onFileOpen(entry);
      return;
    }

    const cols = columns();
    const colIndex = cols.findIndex((col) => col.entries.some((e) => e.id === entry.id));
    if (colIndex < 0) return;

    const target = cols[colIndex].entries.find((e) => e.id === entry.id);
    if (!target || target.kind !== "Directory") return;
    await openDirectoryAtColumn(colIndex, target);
  };

  createEffect(() => {
    const req = props.navigateRequest;
    if (!req || req.requestId === lastNavigateRequestId) return;
    lastNavigateRequestId = req.requestId;
    void openById(req.entry).finally(() => {
      props.onNavigateRequestHandled?.();
    });
  });

  const handleClick = async (colIndex: number, entry: VaultEntry) => {
    if (isDragging || resizingColIndex !== null) return;

    if (entry.kind === "Directory") {
      await openDirectoryAtColumn(colIndex, entry);
    } else {
      setColumns((prev) =>
        prev.map((col, i) =>
          i === colIndex ? { ...col, selectedId: entry.id } : col
        )
      );
      props.onNavigate(buildCrumbs(columns()));
    }
  };

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
        colIndex: Number(entryEl.getAttribute("data-col-index") ?? -1),
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
      document.body.style.userSelect = "";
      (document.body.style as any).webkitUserSelect = "";
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
          await reloadAllColumns();
        } catch (err) {
          toast.error(String(err));
        }
      }
    } else if (target?.kind === "crumb") {
      try {
        await api.vault.moveEntry(props.vaultId, sourceEntry.id, target.parentId);
        await reloadAllColumns();
      } catch (err) {
        toast.error(String(err));
      }
    }
  };

  const onPointerDown = (e: PointerEvent, entry: VaultEntry) => {
    if (e.button !== 0 || resizingColIndex !== null) return;
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

  const onResizePointerMove = (e: PointerEvent) => {
    if (resizingColIndex === null) return;
    const delta = e.clientX - resizeStartX;
    const nextWidth = Math.max(
      MIN_COLUMN_WIDTH,
      Math.min(MAX_COLUMN_WIDTH, resizeStartWidth + delta),
    );
    setColumnWidths((prev) => {
      if (!prev[resizingColIndex!]) return prev;
      const next = [...prev];
      next[resizingColIndex!] = nextWidth;
      return next;
    });
  };

  const stopResizing = () => {
    resizingColIndex = null;
    document.removeEventListener("pointermove", onResizePointerMove);
    document.removeEventListener("pointerup", stopResizing);
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
    (document.body.style as any).webkitUserSelect = "";
  };

  const onResizePointerDown = (e: PointerEvent, colIndex: number) => {
    if (e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();
    resizingColIndex = colIndex;
    resizeStartX = e.clientX;
    resizeStartWidth = columnWidths()[colIndex] ?? DEFAULT_COLUMN_WIDTH;
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
    (document.body.style as any).webkitUserSelect = "none";
    document.addEventListener("pointermove", onResizePointerMove);
    document.addEventListener("pointerup", stopResizing);
  };

  onCleanup(() => {
    document.removeEventListener("pointermove", onPointerMove);
    document.removeEventListener("pointerup", onPointerUp);
    document.removeEventListener("pointermove", onResizePointerMove);
    document.removeEventListener("pointerup", stopResizing);
    document.body.style.userSelect = "";
    (document.body.style as any).webkitUserSelect = "";
    document.body.style.cursor = "";
    removeGhost();
  });

  return (
    <div class={styles.container} ref={containerRef}>
      <For each={columns()}>
        {(col, colI) => (
          <div
            class={styles.column}
            style={{ width: `${columnWidths()[colI()] ?? DEFAULT_COLUMN_WIDTH}px` }}
          >
            <For each={col.entries}>
              {(entry) => (
                <div
                  class={styles.row}
                  data-entry-id={entry.id}
                  data-entry-kind={entry.kind}
                  data-col-index={colI()}
                  data-selected={String(col.selectedId === entry.id && colI() === columns().length - 1)}
                  data-parent-selected={String(col.selectedId === entry.id && colI() < columns().length - 1)}
                  data-dragover="false"
                  onClick={(e) => {
                    void handleClick(colI(), entry);
                    if (entry.kind === "File" && e.detail === 2) {
                      props.onFileOpen(entry);
                    }
                  }}
                  onContextMenu={(e) => { e.preventDefault(); props.onContextMenu(e, entry); }}
                  onPointerDown={(e) => onPointerDown(e, entry)}
                >
                  <FileIcon name={entry.name} isDir={entry.kind === "Directory"} size={16} />
                  <span class={styles.name}>{entry.name}</span>
                  {entry.kind === "Directory" && (
                    <TbOutlineChevronRight size={12} class={styles.chevron} />
                  )}
                </div>
              )}
            </For>
            <div class={styles.resizeHandle} onPointerDown={(e) => onResizePointerDown(e, colI())} />
          </div>
        )}
      </For>
    </div>
  );
}
