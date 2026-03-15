import { createSignal, onMount, Show } from "solid-js";
import {
  TbOutlineCrown,
  TbOutlineCheck,
  TbOutlineShieldCheck,
  TbOutlineLock,
  TbOutlineExternalLink,
  TbOutlineKey,
  TbOutlineDeviceDesktopOff,
} from "solid-icons/tb";
import { store, activateLicense, deactivateLicense } from "../store";
import { api } from "../api/tauri";
import { t, ti } from "../i18n";
import styles from "./LicensePage.module.css";

const FREE_LIMIT = 3;
const KEY_PATTERN = /^SV-[A-Z0-9]{4}-[A-Z0-9]{4}-[A-Z0-9]{4}-[A-Z0-9]{4}$/;

// LemonSqueezy ürün sayfası — sonra gerçek URL ile değiştir
const BUY_URL = "https://berkansozer.lemonsqueezy.com/buy/shadowvault-pro";

function formatKeyInput(raw: string): string {
  const clean = raw.toUpperCase().replace(/[^A-Z0-9]/g, "");
  const parts: string[] = [];
  for (let i = 0; i < Math.min(clean.length, 16); i += 4) {
    parts.push(clean.slice(i, i + 4));
  }
  return parts.length > 0 ? "SV-" + parts.join("-") : "";
}

export function LicensePage() {
  const [key, setKey] = createSignal("");
  const [hardwareId, setHardwareId] = createSignal<string | null>(null);
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [success, setSuccess] = createSignal(false);
  const [deactivating, setDeactivating] = createSignal(false);
  const [deactivateError, setDeactivateError] = createSignal<string | null>(null);
  const [confirmDeactivate, setConfirmDeactivate] = createSignal(false);

  onMount(async () => {
    try {
      setHardwareId(await api.license.getHardwareId());
    } catch {
      setHardwareId("—");
    }
  });

  const isLicensed = () => store.licenseStatus === "valid";
  const sourceCount = () => store.sources.length;
  const usagePercent = () => Math.min((sourceCount() / FREE_LIMIT) * 100, 100);

  const handleInput = (e: Event) => {
    const input = e.currentTarget as HTMLInputElement;
    const formatted = formatKeyInput(input.value);
    setKey(formatted);
    requestAnimationFrame(() => {
      input.value = formatted;
      input.setSelectionRange(formatted.length, formatted.length);
    });
    setError(null);
    setSuccess(false);
  };

  const handleActivate = async () => {
    const k = key().trim();
    if (!KEY_PATTERN.test(k)) {
      setError(t("lic_key_invalid"));
      return;
    }
    setLoading(true);
    setError(null);
    const result = await activateLicense(k);
    setLoading(false);
    if (result.success) {
      setSuccess(true);
      setKey("");
    } else {
      setError(result.error ?? t("lic_activation_failed"));
    }
  };

  const handleDeactivate = async () => {
    if (!confirmDeactivate()) { setConfirmDeactivate(true); return; }
    setDeactivating(true);
    setDeactivateError(null);
    const result = await deactivateLicense();
    setDeactivating(false);
    if (!result.success) {
      setDeactivateError(result.error ?? t("lic_deactivate_fail"));
      setConfirmDeactivate(false);
    }
  };

  const openBuyUrl = async () => {
    try {
      const { open } = await import("@tauri-apps/plugin-shell");
      await open(BUY_URL);
    } catch {
      window.open(BUY_URL, "_blank");
    }
  };

  return (
    <div class={styles.root}>
      {/* Başlık */}
      <div class={styles.pageHeader}>
        <div class={styles.pageTitle}>{t("lic_title")}</div>
        <div class={styles.pageSubtitle}>{t("lic_subtitle")}</div>
      </div>

      {/* Mevcut Plan Kartı */}
      <div class={styles.planCard} data-pro={String(isLicensed())}>
        <div class={styles.planLeft}>
          <div class={styles.planBadge} data-pro={String(isLicensed())}>
            {isLicensed()
              ? <><TbOutlineCrown size={14} /> {t("lic_badge_pro")}</>
              : <><TbOutlineLock size={14} /> {t("lic_badge_free")}</>}
          </div>
          <div class={styles.planName}>
            {isLicensed() ? t("lic_plan_pro_name") : t("lic_plan_free_name")}
          </div>
          <div class={styles.planDesc}>
            {isLicensed()
              ? t("lic_plan_pro_desc")
              : `${sourceCount()} / ${FREE_LIMIT} ${t("lic_sources_using")}`}
          </div>
        </div>
        <Show when={isLicensed()}>
          <div class={styles.planRight}>
            <TbOutlineShieldCheck size={40} />
          </div>
        </Show>
      </div>

      {/* Kullanım çubuğu — sadece ücretsiz planda */}
      <Show when={!isLicensed()}>
        <div class={styles.usageCard}>
          <div class={styles.usageHeader}>
            <span class={styles.usageLabel}>{t("lic_usage_title")}</span>
            <span class={styles.usageCount}>{sourceCount()} / {FREE_LIMIT}</span>
          </div>
          <div class={styles.usageBar}>
            <div
              class={styles.usageFill}
              style={{ width: `${usagePercent()}%` }}
              data-full={String(sourceCount() >= FREE_LIMIT)}
            />
          </div>
          <div class={styles.usageHint}>
            {sourceCount() >= FREE_LIMIT
              ? t("lic_usage_limit_full")
              : ti("lic_usage_upgrade", { n: FREE_LIMIT - sourceCount() })}
          </div>
        </div>
      </Show>

      <div class={styles.columns}>
        {/* Sol: Özellikler + Satın Al */}
        <div class={styles.col}>
          <div class={styles.section}>
            <div class={styles.sectionTitle}>{t("lic_features_title")}</div>
            <div class={styles.featureList}>
              {(["lic_feature_1", "lic_feature_2", "lic_feature_3", "lic_feature_4", "lic_feature_5"] as const).map((key) => (
                <div class={styles.featureRow}>
                  <span class={styles.featureCheck}><TbOutlineCheck size={14} /></span>
                  <span>{t(key)}</span>
                </div>
              ))}
            </div>

            <Show when={!isLicensed()}>
              <button class={styles.buyBtn} onClick={openBuyUrl}>
                <TbOutlineExternalLink size={15} />
                {t("lic_btn_upgrade")}
              </button>
            </Show>
          </div>
        </div>

        {/* Sağ: Aktivasyon */}
        <div class={styles.col}>
          <Show
            when={!isLicensed()}
            fallback={
              <div class={styles.section}>
                <div class={styles.sectionTitle}>{t("lic_info_title")}</div>
                <div class={styles.activatedMsg}>
                  <TbOutlineShieldCheck size={20} />
                  <span>{t("lic_info_active")}</span>
                </div>
                <div class={styles.hwBlock}>
                  <div class={styles.hwLabel}>{t("lic_hw_short")}</div>
                  <code class={styles.hwValue}>{hardwareId()}</code>
                </div>
                <div class={styles.deactivateBlock}>
                  <p class={styles.deactivateHint}>{t("lic_deactivate_hint")}</p>
                  <Show when={deactivateError()}>
                    <div class={styles.errorMsg}>{deactivateError()}</div>
                  </Show>
                  <button
                    class={styles.deactivateBtn}
                    disabled={deactivating()}
                    onClick={handleDeactivate}
                  >
                    <TbOutlineDeviceDesktopOff size={14} />
                    {deactivating()
                      ? t("lic_deactivating")
                      : confirmDeactivate()
                        ? t("lic_deactivate_confirm")
                        : t("lic_deactivate_btn")}
                  </button>
                  <Show when={confirmDeactivate() && !deactivating()}>
                    <button class={styles.cancelDeactivateBtn} onClick={() => setConfirmDeactivate(false)}>
                      {t("btn_cancel")}
                    </button>
                  </Show>
                </div>
              </div>
            }
          >
            <div class={styles.section}>
              <div class={styles.sectionTitle}>
                <TbOutlineKey size={14} /> {t("lic_activation_title")}
              </div>
              <p class={styles.activationHint}>{t("lic_activation_hint")}</p>

              <div class={styles.field}>
                <label class={styles.fieldLabel}>{t("lic_key_label")}</label>
                <input
                  class={styles.keyInput}
                  type="text"
                  placeholder="SV-XXXX-XXXX-XXXX-XXXX"
                  value={key()}
                  onInput={handleInput}
                  maxLength={22}
                  spellcheck={false}
                  autocomplete="off"
                  onKeyDown={(e) => e.key === "Enter" && !loading() && handleActivate()}
                />
              </div>

              <Show when={error()}>
                <div class={styles.errorMsg}>{error()}</div>
              </Show>
              <Show when={success()}>
                <div class={styles.successMsg}>{t("lic_activated")}</div>
              </Show>

              <button
                class={styles.activateBtn}
                disabled={!KEY_PATTERN.test(key().trim()) || loading()}
                onClick={handleActivate}
              >
                {loading() ? t("lic_btn_activating") : t("lic_btn_activate")}
              </button>

              <div class={styles.hwBlock}>
                <div class={styles.hwLabel}>{t("lic_hw_label")}</div>
                <code class={styles.hwValue}>{hardwareId() ?? "..."}</code>
              </div>
            </div>
          </Show>
        </div>
      </div>
    </div>
  );
}
