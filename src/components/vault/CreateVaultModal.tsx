import { createSignal, Show } from "solid-js";
import { toast } from "solid-sonner";
import { Modal } from "../ui/Modal";
import { Button } from "../ui/Button";
import { api } from "../../api/tauri";
import { t } from "../../i18n";
import type { VaultSummary } from "../../store/types";
import styles from "./VaultModal.module.css";

interface Props {
  open: boolean;
  onClose: () => void;
  onCreated: (v: VaultSummary) => void;
}

const ALGORITHMS = [
  { value: "AES-256-GCM",        nameKey: "vault_algo_aes_name",    descKey: "vault_algo_aes_desc" },
  { value: "ChaCha20-Poly1305",  nameKey: "vault_algo_chacha_name", descKey: "vault_algo_chacha_desc" },
  { value: "XChaCha20-Poly1305", nameKey: "vault_algo_xchacha_name", descKey: "vault_algo_xchacha_desc" },
] as const;

export function CreateVaultModal(props: Props) {
  const [name, setName] = createSignal("");
  const [password, setPassword] = createSignal("");
  const [confirm, setConfirm] = createSignal("");
  const [algorithm, setAlgorithm] = createSignal<string>("AES-256-GCM");
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal("");

  const reset = () => {
    setName("");
    setPassword("");
    setConfirm("");
    setAlgorithm("AES-256-GCM");
    setError("");
    setLoading(false);
  };

  const handleClose = () => {
    reset();
    props.onClose();
  };

  const handleSubmit = async (e: SubmitEvent) => {
    e.preventDefault();
    setError("");

    if (!name().trim()) { setError(t("vault_name")); return; }
    if (password().length < 1) { setError(t("vault_password")); return; }
    if (password() !== confirm()) { setError(t("vault_password_mismatch")); return; }

    setLoading(true);
    try {
      const vault = await api.vault.create(name().trim(), password(), algorithm());
      toast.success(t("vault_create_btn"));
      props.onCreated(vault);
      handleClose();
    } catch (err: any) {
      const msg = String(err);
      if (msg.includes("vault_limit_reached")) {
        setError(t("vault_limit_reached"));
      } else {
        setError(msg);
      }
    } finally {
      setLoading(false);
    }
  };

  return (
    <Modal
      open={props.open}
      onClose={handleClose}
      title={t("vault_create_title")}
      footer={
        <div class={styles.footerRow}>
          <Button variant="ghost" onClick={handleClose} disabled={loading()}>
            {t("btn_cancel")}
          </Button>
          <Button
            type="submit"
            form="create-vault-form"
            disabled={loading()}
          >
            {loading() ? t("vault_creating") : t("vault_create_btn")}
          </Button>
        </div>
      }
    >
      <form id="create-vault-form" onSubmit={handleSubmit}>
        <Show when={error()}>
          <div class={styles.error}>{error()}</div>
        </Show>

        <div class={styles.field}>
          <label class={styles.label}>{t("vault_name")}</label>
          <input
            class={styles.input}
            type="text"
            placeholder={t("vault_name_placeholder")}
            value={name()}
            onInput={(e) => setName(e.currentTarget.value)}
            autofocus
          />
        </div>

        <div class={styles.field}>
          <label class={styles.label}>{t("vault_password")}</label>
          <input
            class={styles.input}
            type="password"
            value={password()}
            onInput={(e) => setPassword(e.currentTarget.value)}
          />
        </div>

        <div class={styles.field}>
          <label class={styles.label}>{t("vault_confirm_password")}</label>
          <input
            class={styles.input}
            type="password"
            value={confirm()}
            onInput={(e) => setConfirm(e.currentTarget.value)}
          />
        </div>

        <div class={styles.field}>
          <label class={styles.label}>{t("vault_algo_label")}</label>
          <div class={styles.algoGrid}>
            {ALGORITHMS.map((algo) => (
              <button
                type="button"
                class={styles.algoCard}
                data-active={String(algorithm() === algo.value)}
                onClick={() => setAlgorithm(algo.value)}
              >
                <span class={styles.algoName}>{t(algo.nameKey as any)}</span>
                <span class={styles.algoDesc}>{t(algo.descKey as any)}</span>
              </button>
            ))}
          </div>
        </div>

        <p class={styles.note}>
          Argon2id · {t("vault_free_limit_note")}
        </p>
      </form>
    </Modal>
  );
}
