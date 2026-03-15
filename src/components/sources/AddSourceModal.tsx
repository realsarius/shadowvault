import { createSignal, Show } from "solid-js";
import { Modal } from "../ui/Modal";
import { Button } from "../ui/Button";
import { SchedulePicker } from "../schedule/SchedulePicker";
import { t } from "../../i18n";
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
    } catch { setError(t("add_src_pick_err")); }
  };

  const pickDest = async () => {
    try {
      const picked = await api.fs.pickDirectory();
      if (picked) setDestPath(picked);
    } catch { setError(t("add_dest_pick_err")); }
  };

  const handleSave = async () => {
    setError(null); setSaving(true);
    try {
      const source = await api.sources.create(name(), sourcePath(), sourceType());
      await api.destinations.add(source.id, destPath(), schedule(), retention());
      props.onCreated();
      handleClose();
    } catch (e: any) {
      setError(e?.message ?? t("add_src_save_err"));
    } finally { setSaving(false); }
  };

  const scheduleDescription = () => {
    const s = schedule();
    if (s.type === "Interval") return `${t("schedule_interval").replace("X", String(s.value.minutes))}`;
    if (s.type === "Cron") return `Cron: ${s.value.expression}`;
    if (s.type === "OnChange") return t("schedule_onchange");
    return t("schedule_manual");
  };

  const stepTitles = () => [t("add_src_step1"), t("add_src_step2"), t("add_src_step3")];

  return (
    <Modal
      open={props.open}
      onClose={handleClose}
      title={`Step ${step()}/3: ${stepTitles()[step() - 1]}`}
      footer={
        <div class={styles.footerRow}>
          <Button variant="ghost" onClick={handleClose}>{t("btn_cancel")}</Button>
          {step() > 1 && <Button variant="ghost" onClick={() => setStep(s => s - 1)}>{t("btn_back")}</Button>}
          {step() < 3 && (
            <Button onClick={() => {
              setError(null);
              if (step() === 1) {
                if (!name().trim()) { setError(t("add_src_name_req")); return; }
                if (!sourcePath().trim()) { setError(t("add_src_path_req")); return; }
              }
              if (step() === 2 && !destPath().trim()) { setError(t("add_src_dest_req")); return; }
              setStep(s => s + 1);
            }}>{t("btn_next")}</Button>
          )}
          {step() === 3 && (
            <Button onClick={handleSave} disabled={saving()}>
              {saving() ? t("btn_saving") : t("btn_save")}
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
          <label class={styles.label}>{t("add_src_name_label")}</label>
          <input class={styles.input} type="text" placeholder={t("add_src_name_ph")}
            value={name()} onInput={(e) => setName(e.currentTarget.value)} />
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
            <input class={`${styles.input} ${styles.inputFlex}`} type="text"
              placeholder={sourceType() === "Directory" ? "/home/user/belgeler" : "/home/user/dosya.txt"}
              value={sourcePath()} onInput={(e) => setSourcePath(e.currentTarget.value)} />
            <Button variant="ghost" size="sm" onClick={pickSource}>{t("btn_browse")}</Button>
          </div>
        </div>
      </Show>

      {/* Step 2 */}
      <Show when={step() === 2}>
        <div class={styles.field}>
          <label class={styles.label}>{t("add_dest_folder")}</label>
          <div class={styles.inputRow}>
            <input class={`${styles.input} ${styles.inputFlex}`} type="text" placeholder="/backup/hedef"
              value={destPath()} onInput={(e) => setDestPath(e.currentTarget.value)} />
            <Button variant="ghost" size="sm" onClick={pickDest}>{t("btn_browse")}</Button>
          </div>
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
      </Show>

      {/* Step 3 */}
      <Show when={step() === 3}>
        <div class={styles.summaryCards}>
          <div class={styles.summaryCard}>
            <div class={styles.summarySection}>{t("sum_source")}</div>
            <div class={styles.summaryRows}>
              <div class={styles.summaryRow}>
                <span class={styles.summaryKey}>{t("sum_name")}</span>
                <span class={styles.summaryVal}>{name()}</span>
              </div>
              <div class={styles.summaryRow}>
                <span class={styles.summaryKey}>{t("sum_type")}</span>
                <span class={styles.summaryVal}>{sourceType() === "Directory" ? t("add_src_folder") : t("add_src_file")}</span>
              </div>
              <div class={styles.summaryRow}>
                <span class={styles.summaryKey}>{t("sum_path")}</span>
                <span class={styles.summaryVal}>{sourcePath()}</span>
              </div>
            </div>
          </div>
          <div class={styles.summaryCard}>
            <div class={styles.summarySection}>{t("sum_target")}</div>
            <div class={styles.summaryRows}>
              <div class={styles.summaryRow}>
                <span class={styles.summaryKey}>{t("sum_path")}</span>
                <span class={styles.summaryVal}>{destPath()}</span>
              </div>
              <div class={styles.summaryRow}>
                <span class={styles.summaryKey}>{t("sum_schedule")}</span>
                <span class={styles.summaryVal}>{scheduleDescription()}</span>
              </div>
              <div class={styles.summaryRow}>
                <span class={styles.summaryKey}>{t("sum_max_ver")}</span>
                <span class={styles.summaryVal}>{maxVersions()}</span>
              </div>
              <div class={styles.summaryRow}>
                <span class={styles.summaryKey}>{t("sum_naming")}</span>
                <span class={styles.summaryVal}>{naming()}</span>
              </div>
            </div>
          </div>
        </div>
      </Show>
    </Modal>
  );
}
