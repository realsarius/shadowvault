import { createSignal, Show } from "solid-js";
import { toast } from "solid-sonner";
import { Modal } from "../ui/Modal";
import { Button } from "../ui/Button";
import { api } from "../../api/tauri";
import { t } from "../../i18n";
import type { VaultSummary } from "../../store/types";
import { parseCommandError } from "../../utils/commandError";
import styles from "./VaultModal.module.css";

interface Props {
  vault: VaultSummary | null;
  onClose: () => void;
  onUnlocked: (vaultId: string) => void;
}

export function UnlockModal(props: Props) {
  const [password, setPassword] = createSignal("");
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal("");

  const handleClose = () => {
    setPassword("");
    setError("");
    setLoading(false);
    props.onClose();
  };

  const handleSubmit = async (e: SubmitEvent) => {
    e.preventDefault();
    if (!props.vault) return;
    setError("");
    setLoading(true);
    try {
      await api.vault.unlock(props.vault.id, password());
      setPassword("");
      props.onUnlocked(props.vault.id);
      props.onClose();
    } catch (err: any) {
      const parsed = parseCommandError(err);
      const msg = parsed.error_code === "wrong_password" ? t("vault_wrong_password") : parsed.message;
      setError(msg);
      toast.error(msg);
    } finally {
      setLoading(false);
    }
  };

  return (
    <Modal
      open={props.vault !== null}
      onClose={handleClose}
      title={t("vault_unlock_title")}
      footer={
        <div class={styles.footerRow}>
          <Button variant="ghost" onClick={handleClose} disabled={loading()}>
            {t("btn_cancel")}
          </Button>
          <Button type="submit" form="unlock-vault-form" disabled={loading()}>
            {loading() ? "..." : t("vault_unlock")}
          </Button>
        </div>
      }
    >
      <form id="unlock-vault-form" onSubmit={handleSubmit}>
        <p class={styles.hint}>{props.vault?.name}</p>

        <Show when={error()}>
          <div class={styles.error}>{error()}</div>
        </Show>

        <div class={styles.field}>
          <label class={styles.label}>{t("vault_password")}</label>
          <input
            class={styles.input}
            type="password"
            value={password()}
            onInput={(e) => setPassword(e.currentTarget.value)}
            autofocus
          />
        </div>
      </form>
    </Modal>
  );
}
