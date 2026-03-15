import { createSignal, Show, createEffect } from "solid-js";
import { Modal } from "../ui/Modal";
import { Button } from "../ui/Button";
import { t } from "../../i18n";
import { api } from "../../api/tauri";
import type { Source, SourceType } from "../../store/types";
import styles from "./AddSourceModal.module.css";

interface Props {
  open: boolean;
  onClose: () => void;
  source: Source | null;
  onUpdated: () => void;
}

export function EditSourceModal(props: Props) {
  const [name, setName] = createSignal("");
  const [sourcePath, setSourcePath] = createSignal("");
  const [sourceType, setSourceType] = createSignal<SourceType>("Directory");
  const [enabled, setEnabled] = createSignal(true);
  const [saving, setSaving] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  createEffect(() => {
    if (props.source) {
      setName(props.source.name);
      setSourcePath(props.source.path);
      setSourceType(props.source.source_type);
      setEnabled(props.source.enabled);
      setError(null);
    }
  });

  const handleClose = () => { setError(null); props.onClose(); };

  const pickSource = async () => {
    try {
      const picked = sourceType() === "Directory"
        ? await api.fs.pickDirectory()
        : await api.fs.pickFile();
      if (picked) setSourcePath(picked);
    } catch { setError(t("add_src_pick_err")); }
  };

  const handleSave = async () => {
    setError(null);
    if (!name().trim()) { setError(t("add_src_name_req")); return; }
    if (!sourcePath().trim()) { setError(t("add_src_path_req")); return; }
    setSaving(true);
    try {
      await api.sources.update(props.source!.id, name().trim(), sourcePath().trim(), sourceType(), enabled());
      props.onUpdated();
      handleClose();
    } catch (e: any) {
      setError(e?.message ?? t("add_src_save_err"));
    } finally { setSaving(false); }
  };

  return (
    <Modal
      open={props.open}
      onClose={handleClose}
      title={t("edit_src_title")}
      footer={
        <div class={styles.footerRow}>
          <Button variant="ghost" onClick={handleClose}>{t("btn_cancel")}</Button>
          <Button onClick={handleSave} disabled={saving()}>
            {saving() ? t("btn_saving") : t("btn_save")}
          </Button>
        </div>
      }
    >
      <Show when={error()}>
        <div class={styles.error}>{error()}</div>
      </Show>

      <div class={styles.field}>
        <label class={styles.label}>{t("add_src_name_label")}</label>
        <input
          class={styles.input}
          type="text"
          value={name()}
          onInput={(e) => setName(e.currentTarget.value)}
        />
      </div>

      <div class={styles.field}>
        <label class={styles.label}>{t("add_src_type_label")}</label>
        <div class={styles.radioGroup}>
          {(["Directory", "File"] as const).map((tp) => (
            <label class={styles.radioLabel}>
              <input type="radio" checked={sourceType() === tp} onChange={() => setSourceType(tp)} />
              {tp === "Directory" ? t("add_src_folder") : t("add_src_file")}
            </label>
          ))}
        </div>
      </div>

      <div class={styles.field}>
        <label class={styles.label}>{t("add_src_path_label")}</label>
        <div class={styles.inputRow}>
          <input
            class={`${styles.input} ${styles.inputFlex}`}
            type="text"
            value={sourcePath()}
            onInput={(e) => setSourcePath(e.currentTarget.value)}
          />
          <Button variant="ghost" size="sm" onClick={pickSource}>{t("btn_browse")}</Button>
        </div>
      </div>

      <div class={styles.field}>
        <label class={styles.radioLabel}>
          <input
            type="checkbox"
            checked={enabled()}
            onChange={(e) => setEnabled(e.currentTarget.checked)}
          />
          {t("edit_src_enabled_label")}
        </label>
      </div>
    </Modal>
  );
}
