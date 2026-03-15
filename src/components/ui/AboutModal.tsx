import { Show } from "solid-js";
import { getVersion } from "@tauri-apps/api/app";
import { createSignal, onMount } from "solid-js";
import { TbOutlineX, TbOutlineCrown, TbOutlineLock, TbOutlineExternalLink } from "solid-icons/tb";
import { store } from "../../store";
import { t } from "../../i18n";
import styles from "./AboutModal.module.css";

interface AboutModalProps {
  open: boolean;
  onClose: () => void;
}

const BUY_URL = "https://berkansozer.lemonsqueezy.com/buy/shadowvault-pro";

export function AboutModal(props: AboutModalProps) {
  const [version, setVersion] = createSignal("0.1.0");

  onMount(async () => {
    try { setVersion(await getVersion()); } catch { /* dev */ }
  });

  const isLicensed = () => store.licenseStatus === "valid";

  const openBuyUrl = async () => {
    try {
      const { open } = await import("@tauri-apps/plugin-shell");
      await open(BUY_URL);
    } catch {
      window.open(BUY_URL, "_blank");
    }
  };

  return (
    <Show when={props.open}>
      <div class={styles.overlay} onClick={props.onClose}>
        <div class={styles.modal} onClick={(e) => e.stopPropagation()}>
          <button class={styles.closeBtn} onClick={props.onClose}>
            <TbOutlineX size={16} />
          </button>

          {/* Icon */}
          <div class={styles.iconWrap}>
            <svg viewBox="0 0 100 100" class={styles.shieldIcon} xmlns="http://www.w3.org/2000/svg">
              <defs>
                <linearGradient id="aboutShield" x1="0.2" y1="0" x2="0.8" y2="1">
                  <stop offset="0%" stop-color="#6b9bff"/>
                  <stop offset="100%" stop-color="#2f5dd8"/>
                </linearGradient>
              </defs>
              <path d="M50 8 L88 22 L88 53 Q88 74 50 90 Q12 74 12 53 L12 22 Z" fill="url(#aboutShield)"/>
              <polyline points="30,50 44,65 70,37" stroke="white" stroke-width="7" stroke-linecap="round" stroke-linejoin="round" fill="none"/>
            </svg>
          </div>

          {/* Name & version */}
          <div class={styles.appName}>ShadowVault</div>
          <div class={styles.appDesc}>{t("about_desc")}</div>
          <div class={styles.versionBadge}>v{version()}</div>

          {/* License badge */}
          <div class={styles.licenseBadge} data-pro={String(isLicensed())}>
            {isLicensed()
              ? <><TbOutlineCrown size={13} /> {t("lic_badge_pro")}</>
              : <><TbOutlineLock size={13} /> {t("lic_badge_free")}</>}
          </div>

          <div class={styles.divider} />

          {/* Copyright */}
          <div class={styles.copyright}>© 2025 Berkan Sözer</div>
          <div class={styles.license}>MIT License</div>

          {/* Upgrade link */}
          <Show when={!isLicensed()}>
            <button class={styles.upgradeBtn} onClick={openBuyUrl}>
              <TbOutlineExternalLink size={13} />
              {t("lic_btn_upgrade")}
            </button>
          </Show>
        </div>
      </div>
    </Show>
  );
}
