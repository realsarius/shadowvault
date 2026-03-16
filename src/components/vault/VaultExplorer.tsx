import {
  createSignal, createEffect, Show, For,
} from "solid-js";
import {
  TbOutlineChevronRight,
  TbOutlineFolderPlus,
  TbOutlineUpload,
  TbOutlineRefresh,
  TbOutlineLock,
  TbOutlineLayoutList,
  TbOutlineLayoutGrid,
} from "solid-icons/tb";
import { toast } from "solid-sonner";
import { open as dialogOpen, save as dialogSave } from "@tauri-apps/plugin-dialog";
import { Modal } from "../ui/Modal";
import { Button } from "../ui/Button";
import { api } from "../../api/tauri";
import { t } from "../../i18n";
import type { VaultEntry, VaultSummary } from "../../store/types";
import { VaultEntryList } from "./VaultEntryList";
import { VaultEntryGrid } from "./VaultEntryGrid";
import { VaultContextMenu, type ContextMenuState } from "./VaultContextMenu";
import { UnlockModal } from "./UnlockModal";
import styles from "./VaultExplorer.module.css";
import modalStyles from "./VaultModal.module.css";

interface Props {
  vault: VaultSummary | null;
  onVaultUpdated: () => void;
}

export function VaultExplorer(props: Props) {
  const [entries, setEntries] = createSignal<VaultEntry[]>([]);
  const [breadcrumb, setBreadcrumb] = createSignal<VaultEntry[]>([]);
  const [selected, setSelected] = createSignal<Set<string>>(new Set<string>());
  const [loading, setLoading] = createSignal(false);
  const [contextMenu, setContextMenu] = createSignal<ContextMenuState | null>(null);

  // Rename modal
  const [renameTarget, setRenameTarget] = createSignal<VaultEntry | null>(null);
  const [renameName, setRenameName] = createSignal("");
  const [renameLoading, setRenameLoading] = createSignal(false);

  // New folder modal
  const [newFolderOpen, setNewFolderOpen] = createSignal(false);
  const [newFolderName, setNewFolderName] = createSignal("");
  const [newFolderLoading, setNewFolderLoading] = createSignal(false);

  // Unlock modal
  const [unlockTarget, setUnlockTarget] = createSignal<VaultSummary | null>(null);

  // View mode: list or grid
  const [viewMode, setViewMode] = createSignal<"list" | "grid">("list");

  const currentParentId = () => {
    const bc = breadcrumb();
    return bc.length > 0 ? bc[bc.length - 1].id : null;
  };

  const loadEntries = async () => {
    const v = props.vault;
    if (!v || !v.unlocked) return;
    setLoading(true);
    try {
      const list = await api.vault.listEntries(v.id, currentParentId());
      setEntries(list);
    } catch (err: any) {
      toast.error(String(err));
    } finally {
      setLoading(false);
    }
  };

  createEffect(() => {
    const v = props.vault;
    if (v?.unlocked) {
      loadEntries();
    } else {
      setEntries([]);
      setBreadcrumb([]);
    }
  });

  const navigateInto = (entry: VaultEntry) => {
    if (entry.kind !== "Directory") return;
    setBreadcrumb((prev) => [...prev, entry]);
    setSelected(new Set<string>());
  };

  const navigateTo = (index: number) => {
    setBreadcrumb((prev) => prev.slice(0, index + 1));
    setSelected(new Set<string>());
  };

  const navigateRoot = () => {
    setBreadcrumb([]);
    setSelected(new Set<string>());
  };

  createEffect(() => {
    // breadcrumb değişince entry'leri yenile
    void breadcrumb();
    if (props.vault?.unlocked) loadEntries();
  });

  const handleSelect = (id: string, multi: boolean) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (multi) {
        if (next.has(id)) next.delete(id);
        else next.add(id);
      } else {
        if (next.has(id) && next.size === 1) next.clear();
        else { next.clear(); next.add(id); }
      }
      return next as Set<string>;
    });
  };

  const handleDoubleClick = (entry: VaultEntry) => {
    if (entry.kind === "Directory") {
      navigateInto(entry);
    } else {
      handleOpenFile(entry);
    }
  };

  const handleOpenFile = async (entry: VaultEntry) => {
    try {
      await api.vault.openFile(props.vault!.id, entry.id);
    } catch (err: any) {
      toast.error(String(err));
    }
  };

  const handleExport = async (entry: VaultEntry) => {
    const dest = await dialogSave({ defaultPath: entry.name });
    if (!dest) return;
    try {
      await api.vault.exportFile(props.vault!.id, entry.id, dest);
      toast.success(t("vault_export"));
    } catch (err: any) {
      toast.error(String(err));
    }
  };

  const handleDelete = async (entry: VaultEntry) => {
    if (!confirm(t("vault_confirm_delete"))) return;
    try {
      await api.vault.deleteEntry(props.vault!.id, entry.id);
      await loadEntries();
    } catch (err: any) {
      toast.error(String(err));
    }
  };

  const handleRename = async () => {
    const entry = renameTarget();
    if (!entry) return;
    setRenameLoading(true);
    try {
      await api.vault.renameEntry(props.vault!.id, entry.id, renameName());
      await loadEntries();
      setRenameTarget(null);
    } catch (err: any) {
      toast.error(String(err));
    } finally {
      setRenameLoading(false);
    }
  };

  const handleNewFolder = async () => {
    if (!newFolderName().trim()) return;
    setNewFolderLoading(true);
    try {
      await api.vault.createDirectory(props.vault!.id, newFolderName().trim(), currentParentId());
      await loadEntries();
      setNewFolderOpen(false);
      setNewFolderName("");
    } catch (err: any) {
      toast.error(String(err));
    } finally {
      setNewFolderLoading(false);
    }
  };

  const handleImportFile = async () => {
    const files = await dialogOpen({ multiple: true });
    if (!files) return;
    const paths = Array.isArray(files) ? files : [files];
    for (const p of paths) {
      try {
        await api.vault.importFile(props.vault!.id, p, currentParentId());
      } catch (err: any) {
        toast.error(String(err));
      }
    }
    await loadEntries();
    toast.success(t("vault_import_file"));
  };

  const handleImportFolder = async () => {
    const dir = await dialogOpen({ directory: true });
    if (!dir || Array.isArray(dir)) return;
    try {
      await api.vault.importDirectory(props.vault!.id, dir, currentParentId());
      await loadEntries();
      toast.success(t("vault_import_folder"));
    } catch (err: any) {
      toast.error(String(err));
    }
  };

  // Drag & drop (sadece OS'tan gelen dosyalar; iç sürükleme kendi handler'ında)
  const handleDrop = async (e: DragEvent) => {
    e.preventDefault();
    const files = e.dataTransfer?.files;
    // Dosya yoksa ya da içeride zaten işlendiyse atla
    if (!files || files.length === 0 || !props.vault?.unlocked) return;
    for (let i = 0; i < files.length; i++) {
      const f = files[i] as any;
      const path: string = f.path ?? "";
      if (!path) continue;
      try {
        await api.vault.importFile(props.vault!.id, path, currentParentId());
      } catch (err: any) {
        toast.error(String(err));
      }
    }
    await loadEntries();
  };

  return (
    <div
      class={styles.explorer}
      onDragOver={(e) => e.preventDefault()}
      onDrop={handleDrop}
    >
      {/* Toolbar */}
      <div class={styles.toolbar}>
        {/* Breadcrumb */}
        <div class={styles.breadcrumb}>
          <button class={styles.crumb} onClick={navigateRoot}>
            {t("vault_breadcrumb_root")}
          </button>
          <For each={breadcrumb()}>
            {(crumb, i) => (
              <>
                <TbOutlineChevronRight size={12} class={styles.crumbSep} />
                <button
                  class={styles.crumb}
                  data-active={String(i() === breadcrumb().length - 1)}
                  onClick={() => navigateTo(i())}
                >
                  {crumb.name}
                </button>
              </>
            )}
          </For>
        </div>

        {/* Actions */}
        <Show when={props.vault?.unlocked}>
          <div class={styles.toolbarActions}>
            <button class={styles.toolBtn} onClick={handleImportFile} title={t("vault_import_file")}>
              <TbOutlineUpload size={15} />
              <span>{t("vault_import_file")}</span>
            </button>
            <button class={styles.toolBtn} onClick={() => { setNewFolderOpen(true); setNewFolderName(""); }} title={t("vault_new_folder")}>
              <TbOutlineFolderPlus size={15} />
            </button>
            <button class={styles.toolBtn} onClick={loadEntries} title="Yenile">
              <TbOutlineRefresh size={15} />
            </button>
            <div class={styles.viewToggle}>
              <button
                class={styles.viewBtn}
                data-active={String(viewMode() === "list")}
                title="Liste görünümü"
                onClick={() => setViewMode("list")}
              >
                <TbOutlineLayoutList size={15} />
              </button>
              <button
                class={styles.viewBtn}
                data-active={String(viewMode() === "grid")}
                title="Izgara görünümü"
                onClick={() => setViewMode("grid")}
              >
                <TbOutlineLayoutGrid size={15} />
              </button>
            </div>
          </div>
        </Show>
      </div>

      {/* Content */}
      <div class={styles.content}>
        <Show when={!props.vault}>
          <div class={styles.empty}>{t("vault_select_vault")}</div>
        </Show>

        <Show when={props.vault && !props.vault.unlocked}>
          <div class={styles.lockedState}>
            <TbOutlineLock size={40} color="var(--text-secondary)" />
            <p>{t("vault_locked_msg")}</p>
            <Button onClick={() => setUnlockTarget(props.vault)}>
              {t("vault_unlock")}
            </Button>
          </div>
        </Show>

        <Show when={props.vault?.unlocked && !loading() && entries().length === 0}>
          <div class={styles.empty}>{t("vault_empty")}</div>
        </Show>

        <Show when={props.vault?.unlocked && entries().length > 0}>
          {viewMode() === "grid"
            ? <VaultEntryGrid
                vaultId={props.vault!.id}
                entries={entries()}
                selected={selected()}
                onSelect={handleSelect}
                onDoubleClick={handleDoubleClick}
                onContextMenu={(e, entry) => {
                  e.preventDefault();
                  setContextMenu({ x: e.clientX, y: e.clientY, entry });
                }}
                onMoved={loadEntries}
              />
            : <VaultEntryList
                vaultId={props.vault!.id}
                entries={entries()}
                selected={selected()}
                onSelect={handleSelect}
                onDoubleClick={handleDoubleClick}
                onContextMenu={(e, entry) => {
                  e.preventDefault();
                  setContextMenu({ x: e.clientX, y: e.clientY, entry });
                }}
                onMoved={loadEntries}
              />
          }
        </Show>
      </div>

      {/* Context menu */}
      <VaultContextMenu
        menu={contextMenu()}
        onClose={() => setContextMenu(null)}
        onOpen={handleOpenFile}
        onExport={handleExport}
        onRename={(entry) => { setRenameTarget(entry); setRenameName(entry.name); }}
        onDelete={handleDelete}
        onNavigate={navigateInto}
      />

      {/* Rename modal */}
      <Modal
        open={renameTarget() !== null}
        onClose={() => setRenameTarget(null)}
        title={t("vault_rename_title")}
        footer={
          <div class={modalStyles.footerRow}>
            <Button variant="ghost" onClick={() => setRenameTarget(null)} disabled={renameLoading()}>
              {t("btn_cancel")}
            </Button>
            <Button onClick={handleRename} disabled={renameLoading()}>
              {t("btn_save")}
            </Button>
          </div>
        }
      >
        <div class={modalStyles.field}>
          <label class={modalStyles.label}>{t("vault_new_name")}</label>
          <input
            class={modalStyles.input}
            type="text"
            value={renameName()}
            onInput={(e) => setRenameName(e.currentTarget.value)}
            onKeyDown={(e) => { if (e.key === "Enter") handleRename(); }}
            autofocus
          />
        </div>
      </Modal>

      {/* New folder modal */}
      <Modal
        open={newFolderOpen()}
        onClose={() => setNewFolderOpen(false)}
        title={t("vault_new_folder")}
        footer={
          <div class={modalStyles.footerRow}>
            <Button variant="ghost" onClick={() => setNewFolderOpen(false)} disabled={newFolderLoading()}>
              {t("btn_cancel")}
            </Button>
            <Button onClick={handleNewFolder} disabled={newFolderLoading()}>
              {t("vault_create_btn")}
            </Button>
          </div>
        }
      >
        <div class={modalStyles.field}>
          <label class={modalStyles.label}>{t("vault_new_folder_name")}</label>
          <input
            class={modalStyles.input}
            type="text"
            placeholder={t("vault_new_folder_name")}
            value={newFolderName()}
            onInput={(e) => setNewFolderName(e.currentTarget.value)}
            onKeyDown={(e) => { if (e.key === "Enter") handleNewFolder(); }}
            autofocus
          />
        </div>
      </Modal>

      {/* Unlock modal */}
      <UnlockModal
        vault={unlockTarget()}
        onClose={() => setUnlockTarget(null)}
        onUnlocked={() => { props.onVaultUpdated(); setUnlockTarget(null); }}
      />
    </div>
  );
}
