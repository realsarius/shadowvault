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
  TbOutlineLayoutColumns,
} from "solid-icons/tb";
import { toast } from "solid-sonner";
import { open as dialogOpen, save as dialogSave } from "@tauri-apps/plugin-dialog";
import { Modal } from "../ui/Modal";
import { Button } from "../ui/Button";
import { ConfirmDialog } from "../ui/ConfirmDialog";
import { api } from "../../api/tauri";
import { t } from "../../i18n";
import type { VaultEntry, VaultSummary } from "../../store/types";
import { VaultEntryList } from "./VaultEntryList";
import { VaultEntryGrid } from "./VaultEntryGrid";
import { VaultEntryColumns } from "./VaultEntryColumns";
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

  const [renameTarget, setRenameTarget] = createSignal<VaultEntry | null>(null);
  const [renameName, setRenameName] = createSignal("");
  const [renameLoading, setRenameLoading] = createSignal(false);

  const [newFolderOpen, setNewFolderOpen] = createSignal(false);
  const [newFolderName, setNewFolderName] = createSignal("");
  const [newFolderLoading, setNewFolderLoading] = createSignal(false);
  const [deleteTarget, setDeleteTarget] = createSignal<VaultEntry | null>(null);
  const [deleteLoading, setDeleteLoading] = createSignal(false);

  const [unlockTarget, setUnlockTarget] = createSignal<VaultSummary | null>(null);

  const [viewMode, setViewMode] = createSignal<"list" | "grid" | "columns">("list");
  const [columnsReloadKey, setColumnsReloadKey] = createSignal(0);
  const [columnsNavigateRequest, setColumnsNavigateRequest] = createSignal<{ entry: VaultEntry; requestId: number } | null>(null);
  let columnsNavigateRequestId = 0;

  let directColumnsReload: (() => void) | undefined;
  const bumpColumnsReload = () => {
    setColumnsReloadKey((prev) => prev + 1);
    directColumnsReload?.();
  };
  const requestColumnsOpen = (entry: VaultEntry) => {
    columnsNavigateRequestId += 1;
    setColumnsNavigateRequest({ entry, requestId: columnsNavigateRequestId });
  };

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
      setColumnsNavigateRequest(null);
      setDeleteTarget(null);
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
    void breadcrumb();
    if (props.vault?.unlocked && viewMode() !== "columns") loadEntries();
  });

  const handleColumnsNavigate = (crumbs: VaultEntry[]) => {
    setBreadcrumb(crumbs);
    setSelected(new Set<string>());
  };

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

  const handleDeleteRequest = (entry: VaultEntry) => {
    setDeleteTarget(entry);
  };

  const handleDeleteConfirm = async () => {
    const entry = deleteTarget();
    if (!entry) return;
    setDeleteLoading(true);
    try {
      await api.vault.deleteEntry(props.vault!.id, entry.id);
      setBreadcrumb((prev) => {
        const idx = prev.findIndex((crumb) => crumb.id === entry.id);
        return idx >= 0 ? prev.slice(0, idx) : prev;
      });
      await loadEntries();
      bumpColumnsReload();
      setDeleteTarget(null);
      toast.success(t("vault_delete"));
    } catch (err: any) {
      toast.error(String(err));
    } finally {
      setDeleteLoading(false);
    }
  };

  const handleRename = async () => {
    const entry = renameTarget();
    if (!entry) return;
    const nextName = renameName().trim();
    if (!nextName) {
      toast.error("İsim boş olamaz");
      return;
    }
    setRenameLoading(true);
    try {
      await api.vault.renameEntry(props.vault!.id, entry.id, nextName);
      setBreadcrumb((prev) => prev.map((crumb) => (
        crumb.id === entry.id ? { ...crumb, name: nextName } : crumb
      )));
      await loadEntries();
      bumpColumnsReload();
      setRenameTarget(null);
      toast.success(t("vault_rename"));
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
      bumpColumnsReload();
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
    bumpColumnsReload();
    toast.success(t("vault_import_file"));
  };

  const handleDrop = async (e: DragEvent) => {
    e.preventDefault();
    const files = e.dataTransfer?.files;
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
    bumpColumnsReload();
  };

  return (
    <div
      class={styles.explorer}
      onDragOver={(e) => e.preventDefault()}
      onDrop={handleDrop}
    >
      <div class={styles.toolbar}>
        <div class={styles.breadcrumb}>
          <button
            class={styles.crumb}
            data-crumb-parent-id={breadcrumb().length > 0 ? "__root__" : undefined}
            data-dragover="false"
            onClick={navigateRoot}
          >
            {t("vault_breadcrumb_root")}
          </button>
          <For each={breadcrumb()}>
            {(crumb, i) => (
              <>
                <TbOutlineChevronRight size={12} class={styles.crumbSep} />
                <button
                  class={styles.crumb}
                  data-active={String(i() === breadcrumb().length - 1)}
                  data-crumb-parent-id={i() < breadcrumb().length - 1 ? crumb.id : undefined}
                  data-dragover="false"
                  onClick={() => navigateTo(i())}
                >
                  {crumb.name}
                </button>
              </>
            )}
          </For>
        </div>

        <Show when={props.vault?.unlocked}>
          <div class={styles.toolbarActions}>
            <button class={styles.toolBtn} onClick={handleImportFile} title={t("vault_import_file")}>
              <TbOutlineUpload size={15} />
              <span>{t("vault_import_file")}</span>
            </button>
            <button class={styles.toolBtn} onClick={() => { setNewFolderOpen(true); setNewFolderName(""); }} title={t("vault_new_folder")}>
              <TbOutlineFolderPlus size={15} />
            </button>
            <button class={styles.toolBtn} onClick={() => { void loadEntries(); bumpColumnsReload(); }} title="Yenile">
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
              <button
                class={styles.viewBtn}
                data-active={String(viewMode() === "columns")}
                title="Sütun görünümü"
                onClick={() => setViewMode("columns")}
              >
                <TbOutlineLayoutColumns size={15} />
              </button>
            </div>
          </div>
        </Show>
      </div>

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

        <Show when={props.vault?.unlocked && !loading() && entries().length === 0 && viewMode() !== "columns"}>
          <div class={styles.empty}>{t("vault_empty")}</div>
        </Show>

        <Show when={props.vault?.unlocked && viewMode() === "columns"}>
          <VaultEntryColumns
            vaultId={props.vault!.id}
            onFileOpen={handleOpenFile}
            onContextMenu={(e, entry) => {
              e.preventDefault();
              setContextMenu({ x: e.clientX, y: e.clientY, entry });
            }}
            onNavigate={handleColumnsNavigate}
            reloadKey={columnsReloadKey()}
            navigateRequest={columnsNavigateRequest()}
            onNavigateRequestHandled={() => setColumnsNavigateRequest(null)}
            onReady={(fn) => { directColumnsReload = fn; }}
          />
        </Show>

        <Show when={props.vault?.unlocked && viewMode() !== "columns" && entries().length > 0}>
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

      <VaultContextMenu
        menu={contextMenu()}
        onClose={() => setContextMenu(null)}
        onOpen={(entry) => {
          if (viewMode() === "columns") {
            requestColumnsOpen(entry);
            return;
          }
          void handleOpenFile(entry);
        }}
        onExport={handleExport}
        onRename={(entry) => { setRenameTarget(entry); setRenameName(entry.name); }}
        onDelete={handleDeleteRequest}
        onNavigate={(entry) => {
          if (viewMode() === "columns") {
            requestColumnsOpen(entry);
            return;
          }
          navigateInto(entry);
        }}
      />

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

      <ConfirmDialog
        open={deleteTarget() !== null}
        message={deleteTarget()
          ? `${t("vault_confirm_delete")} (${deleteTarget()!.name})`
          : t("vault_confirm_delete")}
        onConfirm={() => { if (!deleteLoading()) void handleDeleteConfirm(); }}
        onCancel={() => { if (!deleteLoading()) setDeleteTarget(null); }}
      />

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

      <UnlockModal
        vault={unlockTarget()}
        onClose={() => setUnlockTarget(null)}
        onUnlocked={() => { props.onVaultUpdated(); setUnlockTarget(null); }}
      />
    </div>
  );
}
