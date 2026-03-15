import { createSignal, createEffect, Show, For } from "solid-js";
import { toast } from "solid-sonner";
import { Modal } from "../ui/Modal";
import { Button } from "../ui/Button";
import { t, ti } from "../../i18n";
import { api } from "../../api/tauri";
import type { BackupPreview } from "../../store/types";
import styles from "./PreviewModal.module.css";

interface Props {
  open: boolean;
  onClose: () => void;
  destinationId: string | null;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

export function PreviewModal(props: Props) {
  const [preview, setPreview] = createSignal<BackupPreview | null>(null);
  const [loading, setLoading] = createSignal(false);

  createEffect(() => {
    if (props.open && props.destinationId) {
      setPreview(null);
      setLoading(true);
      api.preview.backup(props.destinationId)
        .then(setPreview)
        .catch((e: any) => toast.error(e?.message ?? t("prev_error")))
        .finally(() => setLoading(false));
    }
  });

  const hiddenCount = () => {
    const p = preview();
    if (!p) return 0;
    return p.copy_count + p.skip_count - p.files.length;
  };

  return (
    <Modal
      open={props.open}
      onClose={props.onClose}
      closeOnBackdrop={true}
      title={t("prev_title")}
      footer={
        <Button variant="ghost" onClick={props.onClose}>{t("btn_cancel")}</Button>
      }
    >
      <Show when={loading()}>
        <div class={styles.loading}>{t("prev_loading")}</div>
      </Show>

      <Show when={preview()}>
        {(p) => (
          <div class={styles.content}>
            {/* Summary */}
            <div class={styles.summary}>
              <div class={styles.summaryRow}>
                <span class={styles.summaryLabel}>{t("prev_source")}</span>
                <span class={styles.summaryValue}>{p().source_name}</span>
              </div>
              <div class={styles.summaryRow}>
                <span class={styles.summaryLabel}>{t("prev_dest")}</span>
                <span class={`${styles.summaryValue} ${styles.summaryMono}`}>{p().dest_path}</span>
              </div>
              <div class={styles.summaryDivider} />
              <div class={styles.summaryRow}>
                <span class={styles.summaryLabel}>{t("prev_copy_count")}</span>
                <span class={styles.summaryValue}>{p().copy_count} dosya</span>
              </div>
              <div class={styles.summaryRow}>
                <span class={styles.summaryLabel}>{t("prev_copy_size")}</span>
                <span class={styles.summaryValue}>{formatBytes(p().copy_bytes)}</span>
              </div>
              <Show when={p().incremental && p().skip_count > 0}>
                <div class={styles.summaryRow}>
                  <span class={styles.summaryLabel}>{t("prev_skip_count")}</span>
                  <span class={`${styles.summaryValue} ${styles.skipped}`}>{p().skip_count} dosya</span>
                </div>
              </Show>
            </div>

            <Show when={p().incremental}>
              <div class={styles.incrNote}>{t("prev_incremental_note")}</div>
            </Show>

            {/* File list */}
            <Show when={p().copy_count === 0}>
              <div class={styles.empty}>{t("prev_no_files")}</div>
            </Show>

            <Show when={p().files.length > 0}>
              <div class={styles.tableWrap}>
                <table class={styles.table}>
                  <thead>
                    <tr>
                      <th class={styles.thFile}>{t("prev_file")}</th>
                      <th class={styles.thSize}>{t("prev_size")}</th>
                    </tr>
                  </thead>
                  <tbody>
                    <For each={p().files.filter(f => f.will_copy)}>
                      {(file) => (
                        <tr>
                          <td class={styles.tdFile}>{file.rel_path}</td>
                          <td class={styles.tdSize}>{formatBytes(file.size_bytes)}</td>
                        </tr>
                      )}
                    </For>
                  </tbody>
                </table>
                <Show when={hiddenCount() > 0}>
                  <div class={styles.moreNote}>
                    {ti("prev_and_more", { n: hiddenCount() })}
                  </div>
                </Show>
              </div>
            </Show>
          </div>
        )}
      </Show>
    </Modal>
  );
}
