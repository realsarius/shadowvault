import { Show, createEffect, onCleanup } from "solid-js";
import {
  TbOutlineFolderOpen,
  TbOutlineDownload,
  TbOutlineEdit,
  TbOutlineTrash,
  TbOutlineArrowRight,
} from "solid-icons/tb";
import { t } from "../../i18n";
import type { VaultEntry } from "../../store/types";
import styles from "./VaultContextMenu.module.css";

export interface ContextMenuState {
  x: number;
  y: number;
  entry: VaultEntry;
}

interface Props {
  menu: ContextMenuState | null;
  onClose: () => void;
  onOpen: (entry: VaultEntry) => void;
  onExport: (entry: VaultEntry) => void;
  onRename: (entry: VaultEntry) => void;
  onDelete: (entry: VaultEntry) => void;
  onNavigate?: (entry: VaultEntry) => void;
}

export function VaultContextMenu(props: Props) {
  const handleClick = (action: () => void) => {
    action();
    props.onClose();
  };

  // Dışarı tıklayınca kapat
  const onDocClick = () => props.onClose();
  createEffect(() => {
    if (props.menu) {
      document.addEventListener("mousedown", onDocClick);
    } else {
      document.removeEventListener("mousedown", onDocClick);
    }
  });
  onCleanup(() => document.removeEventListener("mousedown", onDocClick));

  return (
    <Show when={props.menu}>
      {(menu) => (
        <div
          class={styles.menu}
          style={{ left: `${menu().x}px`, top: `${menu().y}px` }}
          onMouseDown={(e) => e.stopPropagation()}
        >
          <Show when={menu().entry.kind === "File"}>
            <button class={styles.item} onClick={() => handleClick(() => props.onOpen(menu().entry))}>
              <TbOutlineFolderOpen size={14} />
              <span>{t("vault_open")}</span>
            </button>
            <button class={styles.item} onClick={() => handleClick(() => props.onExport(menu().entry))}>
              <TbOutlineDownload size={14} />
              <span>{t("vault_export")}</span>
            </button>
          </Show>

          <Show when={menu().entry.kind === "Directory"}>
            <button class={styles.item} onClick={() => handleClick(() => props.onNavigate?.(menu().entry))}>
              <TbOutlineArrowRight size={14} />
              <span>{t("vault_open")}</span>
            </button>
          </Show>

          <div class={styles.divider} />

          <button class={styles.item} onClick={() => handleClick(() => props.onRename(menu().entry))}>
            <TbOutlineEdit size={14} />
            <span>{t("vault_rename")}</span>
          </button>

          <button class={`${styles.item} ${styles.danger}`} onClick={() => handleClick(() => props.onDelete(menu().entry))}>
            <TbOutlineTrash size={14} />
            <span>{t("vault_delete")}</span>
          </button>
        </div>
      )}
    </Show>
  );
}
