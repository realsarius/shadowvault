import { createSignal, Show } from "solid-js";
import { Modal } from "../ui/Modal";
import { Button } from "../ui/Button";
import { SchedulePicker } from "../schedule/SchedulePicker";
import { api } from "../../api/tauri";
import type { ScheduleType, RetentionPolicy } from "../../store/types";
import styles from "./AddDestinationModal.module.css";

interface Props {
  open: boolean;
  onClose: () => void;
  sourceId: string;
  onCreated: () => void;
}

export function AddDestinationModal(props: Props) {
  const [destPath, setDestPath] = createSignal("");
  const [schedule, setSchedule] = createSignal<ScheduleType>({ type: "Interval", value: { minutes: 60 } });
  const [maxVersions, setMaxVersions] = createSignal(10);
  const [naming, setNaming] = createSignal<"Timestamp" | "Index" | "Overwrite">("Timestamp");
  const [saving, setSaving] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  const retention = (): RetentionPolicy => ({ max_versions: maxVersions(), naming: naming() });

  const reset = () => {
    setDestPath(""); setSchedule({ type: "Interval", value: { minutes: 60 } });
    setMaxVersions(10); setNaming("Timestamp"); setSaving(false); setError(null);
  };

  const handleClose = () => { reset(); props.onClose(); };

  const pickDest = async () => {
    try {
      const picked = await api.fs.pickDirectory();
      if (picked) setDestPath(picked);
    } catch { setError("Klasör seçilemedi."); }
  };

  const handleSave = async () => {
    setError(null);
    if (!destPath().trim()) { setError("Hedef yolu gerekli."); return; }
    setSaving(true);
    try {
      await api.destinations.add(props.sourceId, destPath(), schedule(), retention());
      props.onCreated();
      handleClose();
    } catch (e: any) {
      setError(e?.message ?? "Bir hata oluştu.");
    } finally { setSaving(false); }
  };

  return (
    <Modal
      open={props.open}
      onClose={handleClose}
      title="Yeni Hedef Ekle"
      footer={
        <div class={styles.footerRow}>
          <Button variant="ghost" onClick={handleClose}>İptal</Button>
          <Button onClick={handleSave} disabled={saving()}>
            {saving() ? "Kaydediliyor..." : "Kaydet"}
          </Button>
        </div>
      }
    >
      <Show when={error()}>
        <div class={styles.error}>{error()}</div>
      </Show>

      <div class={styles.field}>
        <label class={styles.label}>Hedef Klasör</label>
        <div class={styles.inputRow}>
          <input class={`${styles.input} ${styles.inputFlex}`} type="text" placeholder="/backup/hedef"
            value={destPath()} onInput={(e) => setDestPath(e.currentTarget.value)} />
          <Button variant="ghost" size="sm" onClick={pickDest}>Seç</Button>
        </div>
      </div>

      <div class={styles.field}>
        <label class={styles.label}>Zamanlama</label>
        <div class={styles.scheduleBox}>
          <SchedulePicker value={schedule()} onChange={setSchedule} />
        </div>
      </div>

      <div class={styles.retentionRow}>
        <div class={styles.retentionCol}>
          <label class={styles.label}>Maksimum Versiyon</label>
          <input class={styles.input} type="number" min={1} max={999} value={maxVersions()}
            onInput={(e) => setMaxVersions(parseInt(e.currentTarget.value) || 10)} />
        </div>
        <div class={styles.retentionCol}>
          <label class={styles.label}>Versiyon Adlandırma</label>
          <select class={styles.input} value={naming()} onChange={(e) => setNaming(e.currentTarget.value as any)}>
            <option value="Timestamp">Zaman Damgası</option>
            <option value="Index">Sıra No</option>
            <option value="Overwrite">Üzerine Yaz</option>
          </select>
        </div>
      </div>
    </Modal>
  );
}
