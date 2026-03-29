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
  let menuRef: HTMLDivElement | undefined;

  const handleClick = (action: () => void) => {
    action();
    props.onClose();
  };

  const handleItemClick = (e: MouseEvent, action: () => void) => {
    e.preventDefault();
    e.stopPropagation();
    handleClick(action);
  };

  const onDocMouseDown = (e: MouseEvent) => {
    const target = e.target as Node | null;
    if (menuRef && target && menuRef.contains(target)) return;
    props.onClose();
  };

  createEffect(() => {
    if (props.menu) {
      document.addEventListener("mousedown", onDocMouseDown);
    } else {
      document.removeEventListener("mousedown", onDocMouseDown);
    }
  });
  onCleanup(() => document.removeEventListener("mousedown", onDocMouseDown));

  return (
    <Show when={props.menu}>
      {(menu) => (
        <div
          ref={menuRef}
          class={styles.menu}
          style={{ left: `${menu().x}px`, top: `${menu().y}px` }}
        >
          <Show when={menu().entry.kind === "File"}>
            <button class={styles.item} onClick={(e) => handleItemClick(e, () => props.onOpen(menu().entry))}>
              <TbOutlineFolderOpen size={14} />
              <span>{t("vault_open")}</span>
            </button>
            <button class={styles.item} onClick={(e) => handleItemClick(e, () => props.onExport(menu().entry))}>
              <TbOutlineDownload size={14} />
              <span>{t("vault_export")}</span>
            </button>
          </Show>

          <Show when={menu().entry.kind === "Directory"}>
            <button class={styles.item} onClick={(e) => handleItemClick(e, () => props.onNavigate?.(menu().entry))}>
              <TbOutlineArrowRight size={14} />
              <span>{t("vault_open")}</span>
            </button>
          </Show>

          <div class={styles.divider} />

          <button class={styles.item} onClick={(e) => handleItemClick(e, () => props.onRename(menu().entry))}>
            <TbOutlineEdit size={14} />
            <span>{t("vault_rename")}</span>
          </button>

          <button class={`${styles.item} ${styles.danger}`} onClick={(e) => handleItemClick(e, () => props.onDelete(menu().entry))}>
            <TbOutlineTrash size={14} />
            <span>{t("vault_delete")}</span>
          </button>
        </div>
      )}
    </Show>
  );
}
