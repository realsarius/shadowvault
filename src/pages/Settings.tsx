import { createSignal, onMount, Show } from "solid-js";
import { getVersion } from "@tauri-apps/api/app";
import { emit } from "@tauri-apps/api/event";
import { store, loadSettings } from "../store";
import { api } from "../api/tauri";
import { Toggle } from "../components/ui/Toggle";
import { Button } from "../components/ui/Button";
import { t, ti } from "../i18n";
import type { AppSettings } from "../store/types";
import styles from "./Settings.module.css";

export function Settings() {
  const [saving, setSaving] = createSignal(false);
  const [saved, setSaved] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [clearingLogs, setClearingLogs] = createSignal(false);
  const [clearedCount, setClearedCount] = createSignal<number | null>(null);
  const [exporting, setExporting] = createSignal(false);
  const [importing, setImporting] = createSignal(false);
  const [configMsg, setConfigMsg] = createSignal<{ type: "ok" | "err"; text: string } | null>(null);

  const [notifEmail, setNotifEmail] = createSignal("");
  const [savingEmail, setSavingEmail] = createSignal(false);
  const [testingEmail, setTestingEmail] = createSignal(false);
  const [emailMsg, setEmailMsg] = createSignal<{ type: "ok" | "err"; text: string } | null>(null);

  const [appVersion, setAppVersion] = createSignal("0.1.0");
  const [checkingUpdate, setCheckingUpdate] = createSignal(false);
  const [installingUpdate, setInstallingUpdate] = createSignal(false);
  const [updateInfo, setUpdateInfo] = createSignal<{ available: boolean; version: string | null; body: string | null } | null>(null);
  const [updateError, setUpdateError] = createSignal<string | null>(null);

  const [runOnStartup, setRunOnStartup] = createSignal(false);
  const [minimizeToTray, setMinimizeToTray] = createSignal(false);
  const [theme, setTheme] = createSignal<"dark" | "light" | "system">("dark");
  const [logRetentionDays, setLogRetentionDays] = createSignal(30);
  const [language, setLanguage] = createSignal<"tr" | "en">("tr");

  onMount(async () => {
    await loadSettings();
    const s = store.settings;
    if (s) {
      setRunOnStartup(s.run_on_startup);
      setMinimizeToTray(s.minimize_to_tray);
      setTheme(s.theme);
      setLogRetentionDays(s.log_retention_days);
      setLanguage(s.language);
    }
    const savedEmail = await api.settings.getValue("notification_email").catch(() => null);
    if (savedEmail) setNotifEmail(savedEmail);
    try { setAppVersion(await getVersion()); } catch { /* ignore in dev */ }
  });

  const handleSaveEmail = async () => {
    setSavingEmail(true); setEmailMsg(null);
    try {
      await api.settings.setValue("notification_email", notifEmail().trim());
      setEmailMsg({ type: "ok", text: t("set_notif_saved") });
    } catch {
      setEmailMsg({ type: "err", text: t("set_notif_save_err") });
    } finally { setSavingEmail(false); }
  };

  const handleTestEmail = async () => {
    if (!notifEmail().trim()) return;
    setTestingEmail(true); setEmailMsg(null);
    try {
      await api.notifications.sendTest(notifEmail().trim());
      setEmailMsg({ type: "ok", text: t("set_notif_test_ok") });
    } catch (e: any) {
      setEmailMsg({ type: "err", text: `${t("set_notif_test_fail")} ${e?.message ?? ""}`.trim() });
    } finally { setTestingEmail(false); }
  };

  const handleSave = async () => {
    setSaving(true); setError(null); setSaved(false);
    const settings: AppSettings = {
      run_on_startup: runOnStartup(), minimize_to_tray: minimizeToTray(),
      theme: theme(), log_retention_days: logRetentionDays(), language: language(),
    };
    try {
      await api.settings.update(settings);
      await loadSettings();
      // Rebuild native menu so language change is reflected immediately
      api.menu.rebuild(language()).catch(() => {});
      setSaved(true);
      setTimeout(() => setSaved(false), 2500);
    } catch (e: any) {
      setError(e?.message ?? t("set_save_err"));
    } finally { setSaving(false); }
  };

  const handleCheckUpdate = async () => {
    setCheckingUpdate(true); setUpdateInfo(null); setUpdateError(null);
    try {
      const info = await api.updater.check();
      setUpdateInfo(info);
    } catch (e: any) {
      setUpdateError(e?.message ?? t("set_update_err"));
    } finally { setCheckingUpdate(false); }
  };

  const handleInstallUpdate = async () => {
    setInstallingUpdate(true);
    try {
      await api.updater.install();
    } catch (e: any) {
      setUpdateError(e?.message ?? t("set_update_err"));
      setInstallingUpdate(false);
    }
  };

  const handleExport = async () => {
    setExporting(true); setConfigMsg(null);
    try {
      await api.config.export();
      setConfigMsg({ type: "ok", text: t("set_config_exported") });
    } catch (e: any) {
      if (e?.message !== "cancelled") setConfigMsg({ type: "err", text: e?.message ?? t("set_config_err") });
    } finally { setExporting(false); }
  };

  const handleImport = async () => {
    setImporting(true); setConfigMsg(null);
    try {
      const result = await api.config.import();
      setConfigMsg({ type: "ok", text: ti("set_config_imported_ok", { s: result.sources_imported, d: result.destinations_imported }) });
    } catch (e: any) {
      if (e?.message !== "cancelled") setConfigMsg({ type: "err", text: e?.message ?? t("set_config_err") });
    } finally { setImporting(false); }
  };

  const handleClearLogs = async () => {
    if (!confirm(`${logRetentionDays()} ${t("set_days")} ${t("set_log_retention_desc")}`)) return;
    setClearingLogs(true); setClearedCount(null);
    try {
      const count = await api.logs.clearOld(logRetentionDays());
      setClearedCount(count);
    } catch { /* ignore */ }
    finally { setClearingLogs(false); }
  };

  return (
    <div class={styles.root}>
      <div class={styles.pageHeader}>
        <div class={styles.pageTitle}>{t("set_title")}</div>
        <div class={styles.pageSubtitle}>{t("set_subtitle")}</div>
      </div>

      <Show when={error()}>
        <div class={`${styles.alert} ${styles.alertError}`}>{error()}</div>
      </Show>
      <Show when={saved()}>
        <div class={`${styles.alert} ${styles.alertSuccess}`}>{t("set_saved")}</div>
      </Show>

      {/* Startup & Behavior */}
      <div class={styles.section}>
        <div class={styles.sectionTitle}>{t("set_startup_section")}</div>
        <div class={styles.row}>
          <div class={styles.rowInfo}>
            <div class={styles.rowLabel}>{t("set_run_on_startup")}</div>
            <div class={styles.rowDesc}>{t("set_run_on_startup_desc")}</div>
          </div>
          <Toggle value={runOnStartup()} onChange={setRunOnStartup} />
        </div>
        <div class={styles.rowLast}>
          <div class={styles.rowInfo}>
            <div class={styles.rowLabel}>{t("set_minimize_tray")}</div>
            <div class={styles.rowDesc}>{t("set_minimize_tray_desc")}</div>
          </div>
          <Toggle value={minimizeToTray()} onChange={setMinimizeToTray} />
        </div>
      </div>

      {/* Appearance */}
      <div class={styles.section}>
        <div class={styles.sectionTitle}>{t("set_appearance_section")}</div>
        <div class={styles.row}>
          <div class={styles.rowInfo}>
            <div class={styles.rowLabel}>{t("set_theme")}</div>
            <div class={styles.rowDesc}>{t("set_theme_desc")}</div>
          </div>
          <select class={styles.select} value={theme()} onChange={(e) => setTheme(e.currentTarget.value as any)}>
            <option value="dark">{t("set_theme_dark")}</option>
            <option value="light">{t("set_theme_light")}</option>
            <option value="system">{t("set_theme_system")}</option>
          </select>
        </div>
        <div class={styles.rowLast}>
          <div class={styles.rowInfo}>
            <div class={styles.rowLabel}>{t("set_language")}</div>
            <div class={styles.rowDesc}>{t("set_language_desc")}</div>
          </div>
          <select class={styles.select} value={language()} onChange={(e) => setLanguage(e.currentTarget.value as any)}>
            <option value="tr">Türkçe</option>
            <option value="en">English</option>
          </select>
        </div>
      </div>

      {/* Email Notifications */}
      <div class={styles.section}>
        <div class={styles.sectionTitle}>{t("set_notif_section")}</div>
        <div class={styles.rowLast}>
          <div class={styles.rowInfo}>
            <div class={styles.rowLabel}>{t("set_notif_email")}</div>
            <div class={styles.rowDesc}>{t("set_notif_email_desc")}</div>
          </div>
          <div class={styles.notifEmailRow}>
            <input
              class={styles.notifEmailInput}
              type="email"
              placeholder={t("set_notif_email_ph")}
              value={notifEmail()}
              onInput={(e) => { setNotifEmail(e.currentTarget.value); setEmailMsg(null); }}
            />
            <Button variant="ghost" size="sm" onClick={handleSaveEmail} disabled={savingEmail()}>
              {savingEmail() ? t("btn_saving") : t("btn_save")}
            </Button>
            <Button variant="ghost" size="sm" onClick={handleTestEmail} disabled={testingEmail() || !notifEmail().trim()}>
              {testingEmail() ? t("set_notif_testing") : t("set_notif_test")}
            </Button>
          </div>
        </div>
        <Show when={emailMsg()}>
          <div class={`${styles.alert} ${emailMsg()!.type === "ok" ? styles.alertSuccess : styles.alertError}`}>
            {emailMsg()!.text}
          </div>
        </Show>
      </div>

      {/* Config Export / Import */}
      <div class={styles.section}>
        <div class={styles.sectionTitle}>{t("set_config_section")}</div>
        <div class={styles.row}>
          <div class={styles.rowInfo}>
            <div class={styles.rowLabel}>{t("set_config_export")}</div>
            <div class={styles.rowDesc}>{t("set_config_export_desc")}</div>
          </div>
          <Button variant="ghost" size="sm" onClick={handleExport} disabled={exporting()}>
            {exporting() ? t("set_config_exporting") : t("set_config_export")}
          </Button>
        </div>
        <div class={styles.rowLast}>
          <div class={styles.rowInfo}>
            <div class={styles.rowLabel}>{t("set_config_import")}</div>
            <div class={styles.rowDesc}>{t("set_config_import_desc")}</div>
          </div>
          <Button variant="ghost" size="sm" onClick={handleImport} disabled={importing()}>
            {importing() ? t("set_config_importing") : t("set_config_import")}
          </Button>
        </div>
        <Show when={configMsg()}>
          <div class={`${styles.alert} ${configMsg()!.type === "ok" ? styles.alertSuccess : styles.alertError}`}>
            {configMsg()!.text}
          </div>
        </Show>
      </div>

      {/* Logs */}
      <div class={styles.section}>
        <div class={styles.sectionTitle}>{t("set_log_section")}</div>
        <div class={styles.rowLast}>
          <div class={styles.rowInfo}>
            <div class={styles.rowLabel}>{t("set_log_retention")}</div>
            <div class={styles.rowDesc}>{t("set_log_retention_desc")}</div>
          </div>
          <div class={styles.retentionRow}>
            <input class={styles.numberInput} type="number" min={1} max={365}
              value={logRetentionDays()} onInput={(e) => setLogRetentionDays(parseInt(e.currentTarget.value) || 30)} />
            <span class={styles.retentionUnit}>{t("set_days")}</span>
            <Button variant="danger" size="sm" onClick={handleClearLogs} disabled={clearingLogs()}>
              {clearingLogs() ? t("btn_clearing") : t("btn_clear")}
            </Button>
          </div>
        </div>
        <Show when={clearedCount() !== null}>
          <div class={styles.clearedMsg}>{clearedCount()} {t("set_records_deleted")}</div>
        </Show>
      </div>

      {/* Watcher warning (Linux inotify) */}
      <Show when={store.watcherWarning}>
        <div class={`${styles.alert} ${styles.alertWarning}`}>
          <strong>{t("set_watcher_warning_title")}:</strong> {store.watcherWarning}
        </div>
      </Show>

      {/* About */}
      <div class={styles.section}>
        <div class={styles.sectionTitle}>{t("set_about_section")}</div>
        <div class={styles.rowLast}>
          <div class={styles.rowInfo}>
            <div class={styles.rowLabel}>{t("set_about_version")}</div>
          </div>
          <div class={styles.aboutVersionRow}>
            <span class={styles.aboutValue}>v{appVersion()}</span>
            <Button variant="ghost" size="sm" onClick={() => emit("show-about")}>
              {t("set_about_section")}
            </Button>
          </div>
        </div>
      </div>

      {/* Updates */}
      <div class={styles.section}>
        <div class={styles.sectionTitle}>{t("set_update_section")}</div>
        <div class={styles.rowLast}>
          <div class={styles.rowInfo}>
            <div class={styles.rowLabel}>{t("set_update_check")}</div>
            <Show when={updateInfo()}>
              <div class={styles.rowDesc}>
                {updateInfo()!.available
                  ? `${t("set_update_available")}: v${updateInfo()!.version}`
                  : t("set_update_none")}
              </div>
            </Show>
            <Show when={updateError()}>
              <div class={`${styles.rowDesc} ${styles.errorText}`}>{updateError()}</div>
            </Show>
          </div>
          <div class={styles.updateActions}>
            <Button variant="ghost" size="sm" onClick={handleCheckUpdate} disabled={checkingUpdate()}>
              {checkingUpdate() ? t("set_update_checking") : t("set_update_check")}
            </Button>
            <Show when={updateInfo()?.available}>
              <Button size="sm" onClick={handleInstallUpdate} disabled={installingUpdate()}>
                {installingUpdate() ? t("set_update_installing") : t("set_update_install")}
              </Button>
            </Show>
          </div>
        </div>
      </div>

      <div class={styles.saveRow}>
        <Button onClick={handleSave} disabled={saving()}>
          {saving() ? t("btn_saving") : t("set_save")}
        </Button>
      </div>
    </div>
  );
}
