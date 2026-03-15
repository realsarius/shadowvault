import { createSignal, Show } from "solid-js";
import { Modal } from "../ui/Modal";
import { Button } from "../ui/Button";
import { SchedulePicker } from "../schedule/SchedulePicker";
import { t } from "../../i18n";
import { api } from "../../api/tauri";
import type { ScheduleType, RetentionPolicy } from "../../store/types";
import styles from "./AddDestinationModal.module.css";

interface Props {
  open: boolean;
  onClose: () => void;
  sourceId: string;
  onCreated: () => void;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

export function AddDestinationModal(props: Props) {
  const [destPath, setDestPath] = createSignal("");
  const [schedule, setSchedule] = createSignal<ScheduleType>({ type: "Interval", value: { minutes: 60 } });
  const [maxVersions, setMaxVersions] = createSignal(10);
  const [naming, setNaming] = createSignal<"Timestamp" | "Index" | "Overwrite">("Timestamp");
  const [saving, setSaving] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [availBytes, setAvailBytes] = createSignal<number | null>(null);
  const LOW_SPACE_THRESHOLD = 500 * 1024 * 1024; // 500 MB

  const retention = (): RetentionPolicy => ({ max_versions: maxVersions(), naming: naming() });

  const reset = () => {
    setDestPath(""); setSchedule({ type: "Interval", value: { minutes: 60 } });
    setMaxVersions(10); setNaming("Timestamp"); setSaving(false); setError(null); setAvailBytes(null);
  };

  const handleClose = () => { reset(); props.onClose(); };

  const checkDiskSpace = async (path: string) => {
    if (!path.trim()) { setAvailBytes(null); return; }
    try {
      const info = await api.fs.getDiskInfo(path);
      setAvailBytes(info.available_bytes);
    } catch {
      setAvailBytes(null);
    }
  };

  const pickDest = async () => {
    try {
      const picked = await api.fs.pickDirectory();
      if (picked) {
        setDestPath(picked);
        await checkDiskSpace(picked);
      }
    } catch { setError(t("add_dest_pick_err")); }
  };

  const handleSave = async () => {
    setError(null);
    if (!destPath().trim()) { setError(t("add_dest_path_req")); return; }
    setSaving(true);
    try {
      await api.destinations.add(props.sourceId, destPath(), schedule(), retention());
      props.onCreated();
      handleClose();
    } catch (e: any) {
      setError(e?.message ?? t("add_dest_save_err"));
    } finally { setSaving(false); }
  };

  return (
    <Modal
      open={props.open}
      onClose={handleClose}
      title={t("add_dest_title")}
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
        <label class={styles.label}>{t("add_dest_folder")}</label>
        <div class={styles.inputRow}>
          <input
            class={`${styles.input} ${styles.inputFlex}`}
            type="text"
            placeholder="/backup/target"
            value={destPath()}
            onInput={(e) => {
              setDestPath(e.currentTarget.value);
              checkDiskSpace(e.currentTarget.value);
            }}
          />
          <Button variant="ghost" size="sm" onClick={pickDest}>{t("btn_browse")}</Button>
        </div>
        <Show when={availBytes() !== null}>
          <div class={styles.diskInfo} data-low={String((availBytes() ?? 0) < LOW_SPACE_THRESHOLD)}>
            <Show when={(availBytes() ?? 0) < LOW_SPACE_THRESHOLD}>
              <span>⚠ {t("add_dest_low_space")} </span>
            </Show>
            {t("add_dest_avail_space")} {formatBytes(availBytes()!)}
          </div>
        </Show>
      </div>

      <div class={styles.field}>
        <label class={styles.label}>{t("add_dest_schedule")}</label>
        <div class={styles.scheduleBox}>
          <SchedulePicker value={schedule()} onChange={setSchedule} />
        </div>
      </div>

      <div class={styles.retentionRow}>
        <div class={styles.retentionCol}>
          <label class={styles.label}>{t("add_dest_max_ver")}</label>
          <input class={styles.input} type="number" min={1} max={999} value={maxVersions()}
            onInput={(e) => setMaxVersions(parseInt(e.currentTarget.value) || 10)} />
        </div>
        <div class={styles.retentionCol}>
          <label class={styles.label}>{t("add_dest_naming")}</label>
          <select class={styles.input} value={naming()} onChange={(e) => setNaming(e.currentTarget.value as any)}>
            <option value="Timestamp">{t("naming_timestamp")}</option>
            <option value="Index">{t("naming_index")}</option>
            <option value="Overwrite">{t("naming_overwrite")}</option>
          </select>
        </div>
      </div>
    </Modal>
  );
}
