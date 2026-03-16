import { createSignal, Show } from "solid-js";
import { TbOutlineShield, TbOutlineFolderPlus, TbOutlineRocket } from "solid-icons/tb";
import { api } from "../../api/tauri";
import { setStore } from "../../store";
import { t } from "../../i18n";
import styles from "./OnboardingModal.module.css";

interface Props {
  open: boolean;
  onClose: () => void;
}

const TOTAL_STEPS = 3;

export function OnboardingModal(props: Props) {
  const [step, setStep] = createSignal(1);
  const [closing, setClosing] = createSignal(false);

  const handleFinish = async () => {
    setClosing(true);
    await api.settings.setValue("onboarding_done", "true").catch(() => {});
    props.onClose();
  };

  const handleGoToSources = async () => {
    await api.settings.setValue("onboarding_done", "true").catch(() => {});
    setStore("activePage", "sources");
    props.onClose();
  };

  if (!props.open) return null;

  return (
    <div class={styles.overlay}>
      <div class={styles.modal}>
        <div class={styles.steps}>
          {[1, 2, 3].map((n) => (
            <div class={`${styles.dot} ${step() >= n ? styles.dotActive : ""}`} />
          ))}
        </div>

        <Show when={step() === 1}>
          <div class={styles.body}>
            <div class={styles.icon}><TbOutlineShield size={48} /></div>
            <div class={styles.title}>{t("onboard_step1_title")}</div>
            <div class={styles.desc}>{t("onboard_step1_desc")}</div>
          </div>
        </Show>

        <Show when={step() === 2}>
          <div class={styles.body}>
            <div class={styles.icon}><TbOutlineFolderPlus size={48} /></div>
            <div class={styles.title}>{t("onboard_step2_title")}</div>
            <div class={styles.desc}>{t("onboard_step2_desc")}</div>
            <ul class={styles.list}>
              <li>{t("onboard_step2_tip1")}</li>
              <li>{t("onboard_step2_tip2")}</li>
              <li>{t("onboard_step2_tip3")}</li>
            </ul>
          </div>
        </Show>

        <Show when={step() === 3}>
          <div class={styles.body}>
            <div class={styles.icon}><TbOutlineRocket size={48} /></div>
            <div class={styles.title}>{t("onboard_step3_title")}</div>
            <div class={styles.desc}>{t("onboard_step3_desc")}</div>
            <div class={styles.shortcuts}>
              <div class={styles.shortcut}><kbd>⌘N</kbd> {t("onboard_shortcut_new")}</div>
              <div class={styles.shortcut}><kbd>⌘R</kbd> {t("onboard_shortcut_run")}</div>
            </div>
          </div>
        </Show>

        <div class={styles.footer}>
          <Show when={step() < TOTAL_STEPS}>
            <button class={styles.btnSkip} onClick={handleFinish} disabled={closing()}>
              {t("onboard_btn_skip")}
            </button>
            <button class={styles.btnNext} onClick={() => setStep(step() + 1)}>
              {t("btn_next")}
            </button>
          </Show>
          <Show when={step() === TOTAL_STEPS}>
            <button class={styles.btnSkip} onClick={handleFinish} disabled={closing()}>
              {t("onboard_btn_later")}
            </button>
            <button class={styles.btnNext} onClick={handleGoToSources} disabled={closing()}>
              {t("onboard_btn_start")}
            </button>
          </Show>
        </div>
      </div>
    </div>
  );
}
