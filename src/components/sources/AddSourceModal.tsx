import { createSignal, Show } from "solid-js";
import { Modal } from "../ui/Modal";
import { Button } from "../ui/Button";
import { SchedulePicker } from "../schedule/SchedulePicker";
import { api } from "../../api/tauri";
import type { ScheduleType, RetentionPolicy, SourceType } from "../../store/types";
import styles from "./AddSourceModal.module.css";

interface Props {
  open: boolean;
  onClose: () => void;
  onCreated: () => void;
}

export function AddSourceModal(props: Props) {
  const [step, setStep] = createSignal(1);
  const [name, setName] = createSignal("");
  const [sourcePath, setSourcePath] = createSignal("");
  const [sourceType, setSourceType] = createSignal<SourceType>("Directory");
  const [destPath, setDestPath] = createSignal("");
  const [schedule, setSchedule] = createSignal<ScheduleType>({ type: "Interval", value: { minutes: 60 } });
  const [maxVersions, setMaxVersions] = createSignal(10);
  const [naming, setNaming] = createSignal<"Timestamp" | "Index" | "Overwrite">("Timestamp");
  const [saving, setSaving] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  const retention = (): RetentionPolicy => ({ max_versions: maxVersions(), naming: naming() });

  const reset = () => {
    setStep(1); setName(""); setSourcePath(""); setSourceType("Directory");
    setDestPath(""); setSchedule({ type: "Interval", value: { minutes: 60 } });
    setMaxVersions(10); setNaming("Timestamp"); setSaving(false); setError(null);
  };

  const handleClose = () => { reset(); props.onClose(); };

  const pickSource = async () => {
    try {
      const picked = sourceType() === "Directory"
        ? await api.fs.pickDirectory()
        : await api.fs.pickFile();
      if (picked) setSourcePath(picked);
    } catch { setError("Dosya seçilemedi."); }
  };

  const pickDest = async () => {
    try {
      const picked = await api.fs.pickDirectory();
      if (picked) setDestPath(picked);
    } catch { setError("Klasör seçilemedi."); }
  };

  const handleSave = async () => {
    setError(null); setSaving(true);
    try {
      const source = await api.sources.create(name(), sourcePath(), sourceType());
      await api.destinations.add(source.id, destPath(), schedule(), retention());
      props.onCreated();
      handleClose();
    } catch (e: any) {
      setError(e?.message ?? "Bir hata oluştu.");
    } finally { setSaving(false); }
  };

  const stepTitles = ["Kaynak", "Hedef & Zamanlama", "Özet"];

  const scheduleDescription = () => {
    const s = schedule();
    if (s.type === "Interval") return `Her ${s.value.minutes} dakikada bir`;
    if (s.type === "Cron") return `Cron: ${s.value.expression}`;
    if (s.type === "OnChange") return "Dosya değişince";
    return "Sadece manuel";
  };

  return (
    <Modal
      open={props.open}
      onClose={handleClose}
      title={`Yeni Kaynak — Adım ${step()}/3: ${stepTitles[step() - 1]}`}
      footer={
        <div class={styles.footerRow}>
          <Button variant="ghost" onClick={handleClose}>İptal</Button>
          {step() > 1 && <Button variant="ghost" onClick={() => setStep(s => s - 1)}>Geri</Button>}
          {step() < 3 && (
            <Button onClick={() => {
              setError(null);
              if (step() === 1) {
                if (!name().trim()) { setError("Kaynak adı gerekli."); return; }
                if (!sourcePath().trim()) { setError("Kaynak yolu gerekli."); return; }
              }
              if (step() === 2 && !destPath().trim()) { setError("Hedef yolu gerekli."); return; }
              setStep(s => s + 1);
            }}>İleri</Button>
          )}
          {step() === 3 && (
            <Button onClick={handleSave} disabled={saving()}>
              {saving() ? "Kaydediliyor..." : "Kaydet"}
            </Button>
          )}
        </div>
      }
    >
      <Show when={error()}>
        <div class={styles.error}>{error()}</div>
      </Show>

      {/* Step 1 */}
      <Show when={step() === 1}>
        <div class={styles.field}>
          <label class={styles.label}>Kaynak Adı</label>
          <input class={styles.input} type="text" placeholder="Örn: Proje Dosyaları"
            value={name()} onInput={(e) => setName(e.currentTarget.value)} />
        </div>
        <div class={styles.field}>
          <label class={styles.label}>Kaynak Türü</label>
          <div class={styles.radioGroup}>
            {(["Directory", "File"] as const).map((t) => (
              <label class={styles.radioLabel}>
                <input type="radio" checked={sourceType() === t} onChange={() => setSourceType(t)} />
                {t === "Directory" ? "Klasör" : "Dosya"}
              </label>
            ))}
          </div>
        </div>
        <div class={styles.field}>
          <label class={styles.label}>Kaynak Yolu</label>
          <div class={styles.inputRow}>
            <input class={`${styles.input} ${styles.inputFlex}`} type="text"
              placeholder={sourceType() === "Directory" ? "/home/user/belgeler" : "/home/user/dosya.txt"}
              value={sourcePath()} onInput={(e) => setSourcePath(e.currentTarget.value)} />
            <Button variant="ghost" size="sm" onClick={pickSource}>Seç</Button>
          </div>
        </div>
      </Show>

      {/* Step 2 */}
      <Show when={step() === 2}>
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
      </Show>

      {/* Step 3 */}
      <Show when={step() === 3}>
        <div class={styles.summaryCards}>
          <div class={styles.summaryCard}>
            <div class={styles.summarySection}>Kaynak</div>
            <div class={styles.summaryRows}>
              <div class={styles.summaryRow}>
                <span class={styles.summaryKey}>Ad:</span>
                <span class={styles.summaryVal}>{name()}</span>
              </div>
              <div class={styles.summaryRow}>
                <span class={styles.summaryKey}>Tür:</span>
                <span class={styles.summaryVal}>{sourceType() === "Directory" ? "Klasör" : "Dosya"}</span>
              </div>
              <div class={styles.summaryRow}>
                <span class={styles.summaryKey}>Yol:</span>
                <span class={styles.summaryVal}>{sourcePath()}</span>
              </div>
            </div>
          </div>
          <div class={styles.summaryCard}>
            <div class={styles.summarySection}>Hedef</div>
            <div class={styles.summaryRows}>
              <div class={styles.summaryRow}>
                <span class={styles.summaryKey}>Yol:</span>
                <span class={styles.summaryVal}>{destPath()}</span>
              </div>
              <div class={styles.summaryRow}>
                <span class={styles.summaryKey}>Zamanlama:</span>
                <span class={styles.summaryVal}>{scheduleDescription()}</span>
              </div>
              <div class={styles.summaryRow}>
                <span class={styles.summaryKey}>Maks. Versiyon:</span>
                <span class={styles.summaryVal}>{maxVersions()}</span>
              </div>
              <div class={styles.summaryRow}>
                <span class={styles.summaryKey}>Adlandırma:</span>
                <span class={styles.summaryVal}>{naming()}</span>
              </div>
            </div>
          </div>
        </div>
      </Show>
    </Modal>
  );
}
