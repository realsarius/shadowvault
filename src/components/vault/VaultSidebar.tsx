import { For, createSignal, Show } from "solid-js";
import {
  TbOutlineLock,
  TbOutlineLockOpen,
  TbOutlinePlus,
  TbOutlineTrash,
  TbOutlineKey,
  TbOutlineAlertCircle,
} from "solid-icons/tb";
import { toast } from "solid-sonner";
import { Modal } from "../ui/Modal";
import { Button } from "../ui/Button";
import { api } from "../../api/tauri";
import { t, ti } from "../../i18n";
import type { VaultSummary } from "../../store/types";
import { parseCommandError } from "../../utils/commandError";
import styles from "./VaultSidebar.module.css";
import modalStyles from "./VaultModal.module.css";

interface Props {
  vaults: VaultSummary[];
  activeId: string | null;
  onSelect: (id: string) => void;
  onNew: () => void;
  onVaultsChange: () => void;
}

export function VaultSidebar(props: Props) {
  const [deleteTarget, setDeleteTarget] = createSignal<VaultSummary | null>(null);
  const [deletePassword, setDeletePassword] = createSignal("");
  const [deleteLoading, setDeleteLoading] = createSignal(false);
  const [deleteError, setDeleteError] = createSignal("");

  // Kilitleme onay modalı (açık dosyalar varken)
  const [lockTarget, setLockTarget] = createSignal<VaultSummary | null>(null);
  const [lockOpenFiles, setLockOpenFiles] = createSignal<{ file_name: string }[]>([]);
  const [lockLoading, setLockLoading] = createSignal(false);

  const handleDelete = async () => {
    const v = deleteTarget();
    if (!v) return;
    setDeleteError("");
    setDeleteLoading(true);
    try {
      await api.vault.deleteVault(v.id, deletePassword());
      toast.success(t("vault_delete"));
      setDeleteTarget(null);
      setDeletePassword("");
      props.onVaultsChange();
    } catch (err: any) {
      const parsed = parseCommandError(err);
      const msg = parsed.error_code === "wrong_password" ? t("vault_wrong_password") : parsed.message;
      setDeleteError(msg);
      toast.error(msg);
    } finally {
      setDeleteLoading(false);
    }
  };

  const handleLock = async (e: MouseEvent, vault: VaultSummary) => {
    e.stopPropagation();
    try {
      // Açık dosya var mı kontrol et
      const openFiles = await api.vault.getOpenFiles(vault.id);
      if (openFiles.length > 0) {
        // Onay modalını göster
        setLockTarget(vault);
        setLockOpenFiles(openFiles);
        return;
      }
      await api.vault.lock(vault.id);
      props.onVaultsChange();
    } catch (err: any) {
      toast.error(String(err));
    }
  };

  const handleSyncAndLock = async (save: boolean) => {
    const vault = lockTarget();
    if (!vault) return;
    setLockLoading(true);
    try {
      await api.vault.syncAndLock(vault.id, save);
      setLockTarget(null);
      props.onVaultsChange();
      if (save) toast.success(t("vault_save_and_lock"));
    } catch (err: any) {
      toast.error(String(err));
    } finally {
      setLockLoading(false);
    }
  };

  return (
    <aside class={styles.sidebar}>
      <div class={styles.sidebarHeader}>
        <span class={styles.sidebarTitle}>{t("vault_page_title")}</span>
        <button class={styles.newBtn} onClick={props.onNew} title={t("vault_new")}>
          <TbOutlinePlus size={16} />
        </button>
      </div>

      <div class={styles.vaultList}>
        <For each={props.vaults}>
          {(vault) => (
            <div
              class={styles.vaultItem}
              data-active={String(props.activeId === vault.id)}
              onClick={() => props.onSelect(vault.id)}
            >
              <span class={styles.vaultIcon}>
                {vault.unlocked
                  ? <TbOutlineLockOpen size={15} color="var(--green, #5ce87a)" />
                  : <TbOutlineLock size={15} color="var(--text-secondary)" />}
              </span>
              <span class={styles.vaultName}>{vault.name}</span>
              <span class={styles.vaultActions}>
                <Show when={vault.unlocked}>
                  <button
                    class={styles.actionBtn}
                    title={t("vault_lock")}
                    onClick={(e) => handleLock(e, vault)}
                  >
                    <TbOutlineKey size={13} />
                  </button>
                </Show>
                <button
                  class={`${styles.actionBtn} ${styles.dangerBtn}`}
                  title={t("vault_delete_vault")}
                  onClick={(e) => {
                    e.stopPropagation();
                    setDeleteTarget(vault);
                    setDeletePassword("");
                    setDeleteError("");
                  }}
                >
                  <TbOutlineTrash size={13} />
                </button>
              </span>
            </div>
          )}
        </For>
      </div>

      {/* Lock onay modalı — açık dosyalar var */}
      <Modal
        open={lockTarget() !== null}
        onClose={() => setLockTarget(null)}
        title={t("vault_lock")}
        closeOnBackdrop={false}
        footer={
          <div class={modalStyles.footerRow}>
            <Button variant="ghost" onClick={() => setLockTarget(null)} disabled={lockLoading()}>
              {t("btn_cancel")}
            </Button>
            <Button variant="ghost" onClick={() => handleSyncAndLock(false)} disabled={lockLoading()}>
              {t("vault_discard_and_lock")}
            </Button>
            <Button onClick={() => handleSyncAndLock(true)} disabled={lockLoading()}>
              {lockLoading() ? t("vault_syncing") : t("vault_save_and_lock")}
            </Button>
          </div>
        }
      >
        <div class={styles.openFilesWarning}>
          <TbOutlineAlertCircle size={18} color="var(--yellow)" />
          <p>
            <strong>{ti("vault_open_files_warning", { n: lockOpenFiles().length })}</strong>
            <br />
            {t("vault_open_files_detail")}
          </p>
        </div>
        <ul class={styles.openFilesList}>
          <For each={lockOpenFiles()}>
            {(f) => <li>{f.file_name}</li>}
          </For>
        </ul>
      </Modal>

      {/* Delete vault modal */}
      <Modal
        open={deleteTarget() !== null}
        onClose={() => setDeleteTarget(null)}
        title={t("vault_delete_vault")}
        footer={
          <div class={modalStyles.footerRow}>
            <Button variant="ghost" onClick={() => setDeleteTarget(null)} disabled={deleteLoading()}>
              {t("btn_cancel")}
            </Button>
            <Button variant="danger" onClick={handleDelete} disabled={deleteLoading()}>
              {deleteLoading() ? "..." : t("vault_delete")}
            </Button>
          </div>
        }
      >
        <p class={modalStyles.hint}>{t("vault_confirm_delete_vault")}</p>
        <Show when={deleteError()}>
          <div class={modalStyles.error}>{deleteError()}</div>
        </Show>
        <div class={modalStyles.field}>
          <label class={modalStyles.label}>{t("vault_password")}</label>
          <input
            class={modalStyles.input}
            type="password"
            value={deletePassword()}
            onInput={(e) => setDeletePassword(e.currentTarget.value)}
            autofocus
          />
        </div>
      </Modal>
    </aside>
  );
}
