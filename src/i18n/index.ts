import { store } from "../store";

const tr = {
  // Navigation
  nav_dashboard: "Genel Bakış",
  nav_sources: "Kaynaklar",
  nav_logs: "Loglar",
  nav_settings: "Ayarlar",

  // TopBar
  topbar_brand: "ShadowVault",
  topbar_pause: "⏸ Duraklat",
  topbar_resume: "▶ Devam",

  // Buttons / common
  btn_save: "Kaydet",
  btn_saving: "Kaydediliyor...",
  btn_cancel: "İptal",
  btn_delete: "Sil",
  btn_next: "İleri",
  btn_back: "Geri",
  btn_browse: "Seç",
  btn_run_now: "▶ Şimdi Çalıştır",
  btn_running: "Çalışıyor...",
  btn_refresh: "↻ Yenile",
  btn_clear: "Temizle",
  btn_clearing: "Temizleniyor...",
  btn_add_new: "+ Yeni",

  // Status
  status_active: "Aktif",
  status_disabled: "Devre Dışı",
  status_success: "Başarılı",
  status_failed: "Hata",
  status_running: "Çalışıyor",
  status_skipped: "Atlandı",
  status_cancelled: "İptal",
  status_no_error: "Hata Yok",

  // Trigger
  trigger_scheduled: "Zamanlı",
  trigger_onchange: "Değişince",
  trigger_manual: "Manuel",

  // Schedule picker
  schedule_interval: "Her X dakikada bir",
  schedule_cron: "Cron ifadesi",
  schedule_onchange: "Dosya değişince",
  schedule_manual: "Sadece manuel",

  // Retention naming
  naming_timestamp: "Zaman Damgası",
  naming_index: "Sıra No",
  naming_overwrite: "Üzerine Yaz",

  // Source list
  src_list_title: "Kaynaklar",
  src_empty: "Henüz kaynak yok.",
  src_empty_hint: "Yeni bir kaynak ekleyin.",
  src_targets: "hedef",

  // Destination list
  dest_empty: "Bu kaynak için henüz hedef yok.",
  dest_empty_hint: "Hedef ekleyerek yedeklemeyi başlatın.",
  dest_schedule_label: "Zamanlama:",
  dest_last_run: "Son çalışma:",
  dest_next_run: "Sonraki:",
  dest_delete_confirm: "Bu hedefi silmek istediğinizden emin misiniz?",
  dest_add: "+ Hedef Ekle",

  // Source placeholder
  src_select_hint: "Soldaki listeden bir kaynak seçin",
  src_select_hint2: "veya yeni bir kaynak ekleyin.",

  // Add source modal
  add_src_step1: "Kaynak",
  add_src_step2: "Hedef & Zamanlama",
  add_src_step3: "Özet",
  add_src_name_label: "Kaynak Adı",
  add_src_name_ph: "Örn: Proje Dosyaları",
  add_src_type_label: "Kaynak Türü",
  add_src_folder: "Klasör",
  add_src_file: "Dosya",
  add_src_path_label: "Kaynak Yolu",
  add_src_name_req: "Kaynak adı gerekli.",
  add_src_path_req: "Kaynak yolu gerekli.",
  add_src_dest_req: "Hedef yolu gerekli.",
  add_src_pick_err: "Dosya seçilemedi.",
  add_src_save_err: "Bir hata oluştu.",

  // Add destination modal
  add_dest_title: "Yeni Hedef Ekle",
  add_dest_folder: "Hedef Klasör",
  add_dest_schedule: "Zamanlama",
  add_dest_max_ver: "Maksimum Versiyon",
  add_dest_naming: "Versiyon Adlandırma",
  add_dest_path_req: "Hedef yolu gerekli.",
  add_dest_pick_err: "Klasör seçilemedi.",
  add_dest_save_err: "Bir hata oluştu.",
  add_dest_avail_space: "Kullanılabilir alan:",
  add_dest_low_space: "Uyarı: Düşük disk alanı!",

  // Summary
  sum_source: "Kaynak",
  sum_target: "Hedef",
  sum_name: "Ad:",
  sum_type: "Tür:",
  sum_path: "Yol:",
  sum_schedule: "Zamanlama:",
  sum_max_ver: "Maks. Versiyon:",
  sum_naming: "Adlandırma:",

  // Log panel
  log_title: "Log Kayıtları",
  log_filter: "Filtre:",
  log_all_sources: "Tüm Kaynaklar",
  log_all_statuses: "Tüm Durumlar",
  log_records: "kayıt",
  log_empty: "Gösterilecek log kaydı yok.",
  log_col_status: "Durum",
  log_col_source: "Kaynak",
  log_col_dest: "Hedef",
  log_col_trigger: "Tetikleyici",
  log_col_start: "Başlangıç",
  log_col_duration: "Süre",
  log_col_data: "Veri",

  // Dashboard
  dash_total_sources: "Toplam Kaynak",
  dash_success_today: "Bugün Başarılı",
  dash_copied_today: "Bugün Kopyalanan",
  dash_last_error: "Son Hata",
  dash_active: "aktif",
  dash_copies: "kopya",
  dash_total_data: "toplam veri",
  dash_sources_card: "Kaynaklar",
  dash_no_sources: "Henüz kaynak yok. Kaynaklar sayfasından ekleyin.",
  dash_recent: "Son Aktivite",
  dash_no_logs: "Henüz log kaydı yok.",
  dash_last_run: "Son çalışma",
  dash_next_run: "Sonraki",
  dash_run_now: "▶ Çalıştır",
  dash_running: "Çalışıyor...",
  dash_unknown: "Bilinmeyen",
  dash_targets: "hedef",

  // Settings
  set_title: "Ayarlar",
  set_subtitle: "Uygulama davranışını yapılandırın",
  set_startup_section: "Başlangıç ve Davranış",
  set_run_on_startup: "Sistemle Başlat",
  set_run_on_startup_desc: "Bilgisayar açıldığında uygulamayı otomatik başlat",
  set_minimize_tray: "Sistem Tepsisine Küçült",
  set_minimize_tray_desc: "Pencere kapatıldığında arka planda çalışmaya devam et",
  set_appearance_section: "Görünüm ve Dil",
  set_theme: "Tema",
  set_theme_desc: "Arayüz rengini seçin",
  set_theme_dark: "Koyu",
  set_theme_light: "Açık",
  set_theme_system: "Sistem",
  set_language: "Dil",
  set_language_desc: "Arayüz dilini seçin",
  set_log_section: "Log Yönetimi",
  set_log_retention: "Log Saklama Süresi",
  set_log_retention_desc: "Bu süreden eski loglar silinir",
  set_days: "gün",
  set_save: "Ayarları Kaydet",
  set_saved: "Ayarlar başarıyla kaydedildi.",
  set_save_err: "Ayarlar kaydedilemedi.",

  // About
  set_about_section: "Hakkında",
  set_about_version: "Sürüm",
  set_about_framework: "Framework",
  set_watcher_warning_title: "Dosya İzleyici Uyarısı",

  // Updater
  set_update_section: "Güncellemeler",
  set_update_check: "Güncelleme Kontrol Et",
  set_update_checking: "Kontrol ediliyor...",
  set_update_available: "Yeni sürüm mevcut",
  set_update_none: "Uygulama güncel.",
  set_update_install: "Yükle ve Yeniden Başlat",
  set_update_installing: "Yükleniyor...",
  set_update_err: "Güncelleme kontrol edilemedi.",
} as const;

const en: Record<keyof typeof tr, string> = {
  // Navigation
  nav_dashboard: "Overview",
  nav_sources: "Sources",
  nav_logs: "Logs",
  nav_settings: "Settings",

  // TopBar
  topbar_brand: "ShadowVault",
  topbar_pause: "⏸ Pause",
  topbar_resume: "▶ Resume",

  // Buttons / common
  btn_save: "Save",
  btn_saving: "Saving...",
  btn_cancel: "Cancel",
  btn_delete: "Delete",
  btn_next: "Next",
  btn_back: "Back",
  btn_browse: "Browse",
  btn_run_now: "▶ Run Now",
  btn_running: "Running...",
  btn_refresh: "↻ Refresh",
  btn_clear: "Clear",
  btn_clearing: "Clearing...",
  btn_add_new: "+ New",

  // Status
  status_active: "Active",
  status_disabled: "Disabled",
  status_success: "Success",
  status_failed: "Error",
  status_running: "Running",
  status_skipped: "Skipped",
  status_cancelled: "Cancelled",
  status_no_error: "No Error",

  // Trigger
  trigger_scheduled: "Scheduled",
  trigger_onchange: "On Change",
  trigger_manual: "Manual",

  // Schedule picker
  schedule_interval: "Every X minutes",
  schedule_cron: "Cron expression",
  schedule_onchange: "On file change",
  schedule_manual: "Manual only",

  // Retention naming
  naming_timestamp: "Timestamp",
  naming_index: "Index",
  naming_overwrite: "Overwrite",

  // Source list
  src_list_title: "Sources",
  src_empty: "No sources yet.",
  src_empty_hint: "Add a new source.",
  src_targets: "target(s)",

  // Destination list
  dest_empty: "No targets for this source yet.",
  dest_empty_hint: "Start backup by adding a target.",
  dest_schedule_label: "Schedule:",
  dest_last_run: "Last run:",
  dest_next_run: "Next:",
  dest_delete_confirm: "Are you sure you want to delete this target?",
  dest_add: "+ Add Target",

  // Source placeholder
  src_select_hint: "Select a source from the list on the left",
  src_select_hint2: "or add a new source.",

  // Add source modal
  add_src_step1: "Source",
  add_src_step2: "Target & Schedule",
  add_src_step3: "Summary",
  add_src_name_label: "Source Name",
  add_src_name_ph: "E.g. Project Files",
  add_src_type_label: "Source Type",
  add_src_folder: "Folder",
  add_src_file: "File",
  add_src_path_label: "Source Path",
  add_src_name_req: "Source name is required.",
  add_src_path_req: "Source path is required.",
  add_src_dest_req: "Target path is required.",
  add_src_pick_err: "Could not select file.",
  add_src_save_err: "An error occurred.",

  // Add destination modal
  add_dest_title: "Add New Target",
  add_dest_folder: "Target Folder",
  add_dest_schedule: "Schedule",
  add_dest_max_ver: "Maximum Versions",
  add_dest_naming: "Version Naming",
  add_dest_path_req: "Target path is required.",
  add_dest_pick_err: "Could not select folder.",
  add_dest_save_err: "An error occurred.",
  add_dest_avail_space: "Available space:",
  add_dest_low_space: "Warning: Low disk space!",

  // Summary
  sum_source: "Source",
  sum_target: "Target",
  sum_name: "Name:",
  sum_type: "Type:",
  sum_path: "Path:",
  sum_schedule: "Schedule:",
  sum_max_ver: "Max Versions:",
  sum_naming: "Naming:",

  // Log panel
  log_title: "Log Records",
  log_filter: "Filter:",
  log_all_sources: "All Sources",
  log_all_statuses: "All Statuses",
  log_records: "records",
  log_empty: "No log records to display.",
  log_col_status: "Status",
  log_col_source: "Source",
  log_col_dest: "Target",
  log_col_trigger: "Trigger",
  log_col_start: "Start",
  log_col_duration: "Duration",
  log_col_data: "Data",

  // Dashboard
  dash_total_sources: "Total Sources",
  dash_success_today: "Successful Today",
  dash_copied_today: "Copied Today",
  dash_last_error: "Last Error",
  dash_active: "active",
  dash_copies: "copies",
  dash_total_data: "total data",
  dash_sources_card: "Sources",
  dash_no_sources: "No sources yet. Add from the Sources page.",
  dash_recent: "Recent Activity",
  dash_no_logs: "No log records yet.",
  dash_last_run: "Last run",
  dash_next_run: "Next",
  dash_run_now: "▶ Run",
  dash_running: "Running...",
  dash_unknown: "Unknown",
  dash_targets: "target(s)",

  // Settings
  set_title: "Settings",
  set_subtitle: "Configure application behavior",
  set_startup_section: "Startup & Behavior",
  set_run_on_startup: "Start with System",
  set_run_on_startup_desc: "Automatically start the app when the computer starts",
  set_minimize_tray: "Minimize to System Tray",
  set_minimize_tray_desc: "Continue running in background when window is closed",
  set_appearance_section: "Appearance & Language",
  set_theme: "Theme",
  set_theme_desc: "Select interface color",
  set_theme_dark: "Dark",
  set_theme_light: "Light",
  set_theme_system: "System",
  set_language: "Language",
  set_language_desc: "Select interface language",
  set_log_section: "Log Management",
  set_log_retention: "Log Retention Period",
  set_log_retention_desc: "Logs older than this will be deleted",
  set_days: "days",
  set_save: "Save Settings",
  set_saved: "Settings saved successfully.",
  set_save_err: "Settings could not be saved.",

  // About
  set_about_section: "About",
  set_about_version: "Version",
  set_about_framework: "Framework",
  set_watcher_warning_title: "File Watcher Warning",

  // Updater
  set_update_section: "Updates",
  set_update_check: "Check for Updates",
  set_update_checking: "Checking...",
  set_update_available: "New version available",
  set_update_none: "App is up to date.",
  set_update_install: "Install & Restart",
  set_update_installing: "Installing...",
  set_update_err: "Could not check for updates.",
};

export type TKey = keyof typeof tr;

export function t(key: TKey): string {
  const lang = store.settings?.language ?? "tr";
  return (lang === "en" ? en : tr)[key];
}
