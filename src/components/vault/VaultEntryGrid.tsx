import { For, createSignal, onMount } from "solid-js";
import type { VaultEntry } from "../../store/types";
import { FileIcon, isImageFile } from "./FileIcon";
import { api } from "../../api/tauri";
import { toast } from "solid-sonner";
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

export function VaultEntryGrid(props: Props) {
  let gridRef: HTMLDivElement | undefined;

  const handleDragStart = (e: DragEvent, entry: VaultEntry) => {
    activeDragId = entry.id;
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = "move";
      e.dataTransfer.setData("text/plain", entry.id);
    }
    // Overlay'leri aktif et — WebKit'te draggable üzerine drop çalışmıyor,
    // overlay (draggable değil) drop target olarak kullanılıyor.
    gridRef?.setAttribute("data-dragging", "true");
  };

  const handleDragEnd = () => {
    activeDragId = null;
    activeDragOverEl?.setAttribute("data-dragover", "false");
    activeDragOverEl = null;
    gridRef?.removeAttribute("data-dragging");
  };

  const handleOverlayDragEnter = (e: DragEvent, entry: VaultEntry) => {
    if (!activeDragId || activeDragId === entry.id) return;
    e.preventDefault();
    e.stopPropagation();
    const cell = (e.currentTarget as HTMLElement).parentElement;
    if (cell && activeDragOverEl !== cell) {
      activeDragOverEl?.setAttribute("data-dragover", "false");
      activeDragOverEl = cell;
      cell.setAttribute("data-dragover", "true");
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
      const cell = overlay.parentElement;
      cell?.setAttribute("data-dragover", "false");
      if (activeDragOverEl === cell) activeDragOverEl = null;
    }
  };

  const handleOverlayDrop = async (e: DragEvent, entry: VaultEntry) => {
    e.preventDefault();
    e.stopPropagation();
    const cell = (e.currentTarget as HTMLElement).parentElement;
    cell?.setAttribute("data-dragover", "false");
    activeDragOverEl = null;
    gridRef?.removeAttribute("data-dragging");

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
    <div class={styles.grid} ref={gridRef}>
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
            {/* Saydam overlay: sadece klasörler için, drag sırasında CSS ile aktif olur.
                draggable değil → WebKit bu div üzerine drop'a izin verir. */}
            {entry.kind === "Directory" && (
              <div
                class={styles.dropOverlay}
                onDragEnter={(e) => handleOverlayDragEnter(e, entry)}
                onDragOver={(e) => handleOverlayDragOver(e, entry)}
                onDragLeave={handleOverlayDragLeave}
                onDrop={(e) => handleOverlayDrop(e, entry)}
              />
            )}
            <ThumbnailCell vaultId={props.vaultId} entry={entry} />
            <span class={styles.label}>{entry.name}</span>
          </div>
        )}
      </For>
    </div>
  );
}
