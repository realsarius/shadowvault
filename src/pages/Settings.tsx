import { createSignal, onMount, Show } from "solid-js";
import { store, loadSettings } from "../store";
import { api } from "../api/tauri";
import { Toggle } from "../components/ui/Toggle";
import { Button } from "../components/ui/Button";
import type { AppSettings } from "../store/types";
import styles from "./Settings.module.css";

export function Settings() {
  const [saving, setSaving] = createSignal(false);
  const [saved, setSaved] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [clearingLogs, setClearingLogs] = createSignal(false);
  const [clearedCount, setClearedCount] = createSignal<number | null>(null);

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
  });

  const handleSave = async () => {
    setSaving(true); setError(null); setSaved(false);
    const settings: AppSettings = {
      run_on_startup: runOnStartup(), minimize_to_tray: minimizeToTray(),
      theme: theme(), log_retention_days: logRetentionDays(), language: language(),
    };
    try {
      await api.settings.update(settings);
      await loadSettings();
      setSaved(true);
      setTimeout(() => setSaved(false), 2500);
    } catch (e: any) {
      setError(e?.message ?? "Ayarlar kaydedilemedi.");
    } finally { setSaving(false); }
  };

  const handleClearLogs = async () => {
    if (!confirm(`${logRetentionDays()} günden eski logları silmek istediğinizden emin misiniz?`)) return;
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
        <div class={styles.pageTitle}>Ayarlar</div>
        <div class={styles.pageSubtitle}>Uygulama davranışını yapılandırın</div>
      </div>

      <Show when={error()}>
        <div class={`${styles.alert} ${styles.alertError}`}>{error()}</div>
      </Show>
      <Show when={saved()}>
        <div class={`${styles.alert} ${styles.alertSuccess}`}>Ayarlar başarıyla kaydedildi.</div>
      </Show>

      {/* Startup & Behavior */}
      <div class={styles.section}>
        <div class={styles.sectionTitle}>Başlangıç ve Davranış</div>
        <div class={styles.row}>
          <div class={styles.rowInfo}>
            <div class={styles.rowLabel}>Sistemle Başlat</div>
            <div class={styles.rowDesc}>Bilgisayar açıldığında uygulamayı otomatik başlat</div>
          </div>
          <Toggle value={runOnStartup()} onChange={setRunOnStartup} />
        </div>
        <div class={styles.rowLast}>
          <div class={styles.rowInfo}>
            <div class={styles.rowLabel}>Sistem Tepsisine Küçült</div>
            <div class={styles.rowDesc}>Pencere kapatıldığında arka planda çalışmaya devam et</div>
          </div>
          <Toggle value={minimizeToTray()} onChange={setMinimizeToTray} />
        </div>
      </div>

      {/* Appearance */}
      <div class={styles.section}>
        <div class={styles.sectionTitle}>Görünüm ve Dil</div>
        <div class={styles.row}>
          <div class={styles.rowInfo}>
            <div class={styles.rowLabel}>Tema</div>
            <div class={styles.rowDesc}>Arayüz rengini seçin</div>
          </div>
          <select class={styles.select} value={theme()} onChange={(e) => setTheme(e.currentTarget.value as any)}>
            <option value="dark">Koyu</option>
            <option value="light">Açık</option>
            <option value="system">Sistem</option>
          </select>
        </div>
        <div class={styles.rowLast}>
          <div class={styles.rowInfo}>
            <div class={styles.rowLabel}>Dil</div>
            <div class={styles.rowDesc}>Arayüz dilini seçin</div>
          </div>
          <select class={styles.select} value={language()} onChange={(e) => setLanguage(e.currentTarget.value as any)}>
            <option value="tr">Türkçe</option>
            <option value="en">English</option>
          </select>
        </div>
      </div>

      {/* Logs */}
      <div class={styles.section}>
        <div class={styles.sectionTitle}>Log Yönetimi</div>
        <div class={styles.rowLast}>
          <div class={styles.rowInfo}>
            <div class={styles.rowLabel}>Log Saklama Süresi</div>
            <div class={styles.rowDesc}>Bu süreden eski loglar silinir</div>
          </div>
          <div class={styles.retentionRow}>
            <input class={styles.numberInput} type="number" min={1} max={365}
              value={logRetentionDays()} onInput={(e) => setLogRetentionDays(parseInt(e.currentTarget.value) || 30)} />
            <span class={styles.retentionUnit}>gün</span>
            <Button variant="danger" size="sm" onClick={handleClearLogs} disabled={clearingLogs()}>
              {clearingLogs() ? "Temizleniyor..." : "Temizle"}
            </Button>
          </div>
        </div>
        <Show when={clearedCount() !== null}>
          <div class={styles.clearedMsg}>{clearedCount()} log kaydı silindi.</div>
        </Show>
      </div>

      <div class={styles.saveRow}>
        <Button onClick={handleSave} disabled={saving()}>
          {saving() ? "Kaydediliyor..." : "Ayarları Kaydet"}
        </Button>
      </div>
    </div>
  );
}
