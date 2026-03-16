import { For, createSignal, onMount } from "solid-js";
import type { VaultEntry } from "../../store/types";
import { FileIcon, isImageFile } from "./FileIcon";
import { api } from "../../api/tauri";
import styles from "./VaultEntryGrid.module.css";

interface Props {
  vaultId: string;
  entries: VaultEntry[];
  selected: Set<string>;
  onSelect: (id: string, multi: boolean) => void;
  onDoubleClick: (entry: VaultEntry) => void;
  onContextMenu: (e: MouseEvent, entry: VaultEntry) => void;
  onMoved: () => void;
}

function ThumbnailCell(props: { vaultId: string; entry: VaultEntry }) {
  const [thumb, setThumb] = createSignal<string | null>(null);

  onMount(async () => {
    if (props.entry.kind === "File" && isImageFile(props.entry.name)) {
      try {
        const data = await api.vault.getThumbnail(props.vaultId, props.entry.id);
        setThumb(data);
      } catch {
        // no thumbnail
      }
    }
  });

  return (
    <div class={styles.thumb}>
      {thumb()
        ? <img src={`data:image/jpeg;base64,${thumb()}`} class={styles.thumbImg} alt={props.entry.name} />
        : <FileIcon name={props.entry.name} isDir={props.entry.kind === "Directory"} size={36} />
      }
    </div>
  );
}

let activeDragId: string | null = null;
let activeDragOverEl: HTMLElement | null = null;

function closestCell(el: EventTarget | null): HTMLElement | null {
  return (el as HTMLElement | null)?.closest("[data-entry-id]") ?? null;
}

export function VaultEntryGrid(props: Props) {
  const handleDragStart = (e: DragEvent, entry: VaultEntry) => {
    activeDragId = entry.id;
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = "move";
      e.dataTransfer.setData("text/plain", entry.id);
    }
  };

  const handleDragEnd = () => {
    activeDragId = null;
    activeDragOverEl?.setAttribute("data-dragover", "false");
    activeDragOverEl = null;
  };

  const handleGridDragOver = (e: DragEvent) => {
    if (!activeDragId) return;
    const cell = closestCell(e.target);
    if (!cell) return;
    const kind = cell.getAttribute("data-entry-kind");
    const entryId = cell.getAttribute("data-entry-id");
    if (kind !== "Directory" || entryId === activeDragId) return;

    e.preventDefault();
    e.stopPropagation();

    if (activeDragOverEl !== cell) {
      activeDragOverEl?.setAttribute("data-dragover", "false");
      activeDragOverEl = cell;
      cell.setAttribute("data-dragover", "true");
    }
  };

  const handleGridDragLeave = (e: DragEvent) => {
    const related = e.relatedTarget as Node | null;
    if (!related || !(e.currentTarget as HTMLElement).contains(related)) {
      activeDragOverEl?.setAttribute("data-dragover", "false");
      activeDragOverEl = null;
    }
  };

  const handleGridDrop = async (e: DragEvent) => {
    if (!activeDragId) return;
    const cell = closestCell(e.target);
    if (!cell) return;
    const kind = cell.getAttribute("data-entry-kind");
    const targetId = cell.getAttribute("data-entry-id");

    if (kind !== "Directory" || !targetId || targetId === activeDragId) {
      activeDragOverEl?.setAttribute("data-dragover", "false");
      activeDragOverEl = null;
      return;
    }

    e.preventDefault();
    e.stopPropagation();
    cell.setAttribute("data-dragover", "false");
    activeDragOverEl = null;

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
    <div
      class={styles.grid}
      onDragOver={handleGridDragOver}
      onDragLeave={handleGridDragLeave}
      onDrop={handleGridDrop}
    >
      <For each={props.entries}>
        {(entry) => (
          <div
            class={styles.cell}
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
            <ThumbnailCell vaultId={props.vaultId} entry={entry} />
            <span class={styles.label}>{entry.name}</span>
          </div>
        )}
      </For>
    </div>
  );
}
