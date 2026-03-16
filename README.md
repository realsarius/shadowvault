# ShadowVault

## İçindekiler

- [0. Hızlı Kurulum](#0-hızlı-kurulum)
- [1. Kapsam](#1-kapsam)
- [2. Teknoloji Yığını](#2-teknoloji-yığını-technology-stack)
- [3. Veritabanı Tasarımı](#3-veritabanı-tasarımı-database-design)
- [4. API Tasarımı](#4-api-tasarımı-tauri-komutları)
- [5. Loglama ve Hata Yönetimi](#5-loglama-izlenebilirlik-ve-hata-yönetimi)
- [6. Güvenlik Mimarisi](#6-güvenlik-mimarisi)
- [7. Test Stratejisi](#7-test-stratejisi)
- [8. Kurulum ve Çalıştırma](#8-kurulum-ve-çalıştırma)
- [9. Frontend](#9-frontend)
- [10. Production Notes](#10-production-notes)
- [11. Lisans ve Kullanım Notu](#11-lisans-ve-kullanım-notu)
- [Ek Dokümanlar](#ek-dokümanlar)

---

## 0. Hızlı Kurulum

ShadowVault, SolidJS frontend ve Tauri + Rust backend olmak üzere iki katmandan oluşur.

### Gereksinimler

- Node.js 20+ (LTS önerilir)
- Rust toolchain (`rustup` ile kurulur)
- Tauri CLI: `cargo install tauri-cli`
- Platform bağımlılıkları: macOS için Xcode Command Line Tools; Windows için WebView2

### Geliştirme Ortamı

```bash
# 1) Node bağımlılıklarını yükle
npm install

# 2) Tauri geliştirme modunu başlat (Vite + Rust backend)
npm run tauri dev
```

Bu komut Vite sunucusunu ve Tauri Rust backend'ini birlikte başlatır. Hem frontend hot-reload hem de native komutlar gerçek platform üzerinde çalışır.

### Üretim Paketleme

```bash
# 1) Frontend derle
npm run build

# 2) Platforma özgü masaüstü paketi oluştur (.dmg / .exe / .deb)
npm run tauri build
```

### TypeScript Binding'lerini Yenileme

Rust komutlarında değişiklik olduğunda bindings'i yeniden üret:

```bash
cargo test export_bindings --manifest-path src-tauri/Cargo.toml
```

Bu komut `src/generated/bindings.ts` dosyasını günceller.

### Ek Notlar

- Onboarding modunu tekrar görmek için `settings` tablosundaki `onboarding_done` anahtarını sıfırlayın.
- Lisans, `initLicense` içinde 3 deneme yapan retry mantığıyla doğrulanır.
- Veritabanı dosyası uygulama data dizininde (`app_data_dir/shadowvault.db`) otomatik oluşturulur.

---

## 1. Kapsam

**Yedekleme Yönetimi:** Kaynak (dosya/klasör) ve hedef (local, S3/R2, SFTP, OneDrive, Google Drive, Dropbox, WebDAV) yönetimi; artımlı kopyalama, dışlama deseni, retention politikası ve schedule tanımları her hedefte ayrı ayrı yapılandırılabilir.

**Zamanlayıcı ve İzleme:** Cron ifadeleri, aralık tabanlı (Interval), dosya sistemi değişikliği (OnChange) ve manuel tetikleme desteklenir. Scheduler her destination için bağımsız Tokio task'ı çalıştırır; dosya izleyici (notify crate) 500ms debounce ile anlık değişiklikleri yakalar.

**Kopyalama Motoru:** 64 MB chunk'lı büyük dosya aktarımı, SHA-256 bütünlük doğrulaması, AES-256-GCM ile yedek şifrelemesi (Argon2id + HKDF anahtar türetmesi), versiyon yönetimi (Timestamp / Index / Overwrite) ve retention temizliği motor tarafından yönetilir.

**Şifreli Kasa (Vault):** Kullanıcılar hassas dosyalarını birden fazla şifreli kasada saklayabilir. Kasa içindeki her dosya HKDF ile türetilmiş bağımsız anahtarla AES-256-GCM / ChaCha20 / XChaCha20 ile şifrelenir. Vault explorer grid/liste görünümü, thumbnail önizleme, dosya içinde açma ve güvenli silme destekler.

**Bulut Entegrasyonları:** S3/R2 (`object_store`), SFTP (`ssh2`), OneDrive / Google Drive / Dropbox (`opendal` + OAuth2 PKCE) ve WebDAV (`opendal`) desteği; bağlantı test komutları ile entegrasyon doğrulanabilir.

**Lisanslama:** Donanıma özgü (hardware-ID) lisans sistemi; LemonSqueezy deep link (`shadowvault://activate?key=SV-XXXX`) desteği, rate-limited çevrimiçi doğrulama ve offline grace modu.

**UI/UX:** Onboarding sihirbazı, dark/light/system tema, Türkçe/İngilizce çoklu dil, sistem tepsisi entegrasyonu, native bildirimler, klavye kısayolları (Cmd/Ctrl+N, Cmd/Ctrl+R) ve gerçek zamanlı kopyalama ilerleme göstergesi.

---

## 2. Teknoloji Yığını (Technology Stack)

| Kategori | Teknoloji / Kütüphane | Kullanım Amacı |
|---|---|---|
| **Desktop Shell** | Tauri 2, Rust, `@tauri-apps/api` | Native komutlar, event bus, pencere/tray yönetimi |
| **Frontend** | SolidJS, TypeScript, Vite, `solid-sonner`, `solid-icons` | Reaktif UI, toast bildirimleri, ikon seti |
| **State Yönetimi** | `solid-js/store` | Kaynak, log, vault, lisans ve scheduler durumu |
| **API Köprüsü** | `src/api/tauri.ts` | Tipli `invoke` çağrıları, event dinleyiciler |
| **Async Runtime** | Tokio (full, multi-threaded) | Zamanlayıcı, kopyalama task'ları, file watcher |
| **Veritabanı** | SQLx 0.7 + SQLite | Compile-time SQL doğrulama, migration yönetimi |
| **Şifreleme** | `aes-gcm`, `chacha20poly1305`, `argon2`, `sha2`, `hkdf` | Yedek ve vault şifrelemesi, anahtar türetme |
| **Bulut Depolama** | `object_store`, `opendal` | S3/R2, OneDrive, Google Drive, Dropbox, WebDAV |
| **Dosya Sistemi** | `notify`, `walkdir`, `fs_extra` | Anlık değişiklik izleme, dosya traversal |
| **SSH / SFTP** | `ssh2` | SFTP üzerinden dosya aktarımı |
| **HTTP** | `reqwest` (rustls-tls) | Lisans API çağrıları |
| **Zamanlayıcı** | `cron` v0.12, `chrono` | Cron ifade parse, timezone-aware scheduling |
| **Sistem Bilgisi** | `sysinfo` v0.30 | Hardware-ID için donanım bilgisi |
| **Tip Güvenliği** | `specta` rc.22, `tauri-specta` rc.21 | Rust → TypeScript binding otomatik üretimi |
| **Günlükleme** | `log`, `env_logger` | Backend structured logging |
| **i18n** | `src/i18n/index.ts` | Türkçe/İngilizce çeviri sistemi |
| **Plugin'ler** | `tauri-plugin-autostart`, `tauri-plugin-updater`, `tauri-plugin-notification`, `tauri-plugin-deep-link` | Otomatik başlatma, güncelleme, bildirim, derin bağlantı |
| **Test** | Vitest, `@solidjs/testing-library`, `jsdom` | API katmanı ve schedule mantığı unit testleri |

---

## 3. Veritabanı Tasarımı (Database Design)

SQLite kullanılır; şema `src-tauri/migrations/` altında sıralı SQL dosyalarıyla yönetilir ve uygulama başlangıcında `sqlx::migrate!()` ile otomatik uygulanır.

### 3.1 Tablo Listesi

| # | Tablo | Açıklama |
|---|-------|----------|
| 1 | **sources** | Yedekleme kaynakları (dosya/klasör) |
| 2 | **destinations** | Her kaynağa ait hedefler; schedule, retention, bulut config, şifreleme bilgisi |
| 3 | **copy_logs** | Yedekleme çalıştırma günlüğü; boyut, dosya sayısı, checksum, tetikleyici |
| 4 | **settings** | Anahtar-değer uygulama ayarları |
| 5 | **vaults** | Şifreli kasa meta bilgisi (id, name, algorithm, vault_path) |
| 6 | **schema_versions** | Migration sürüm takibi |

### 3.2 Şema Detayları

#### sources
```sql
CREATE TABLE sources (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    path        TEXT NOT NULL,
    source_type TEXT NOT NULL,       -- "File" | "Directory"
    enabled     INTEGER NOT NULL DEFAULT 1,
    created_at  TEXT NOT NULL
);
```

#### destinations
```sql
CREATE TABLE destinations (
    id                   TEXT PRIMARY KEY,
    source_id            TEXT NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
    path                 TEXT NOT NULL,
    schedule_json        TEXT NOT NULL,     -- {"Interval":{"minutes":60}} | {"Cron":{"expression":"0 * * * *"}} | "OnChange" | "Manual"
    retention_json       TEXT NOT NULL,     -- {"max_versions":5,"naming":"Timestamp"}
    enabled              INTEGER NOT NULL DEFAULT 1,
    last_run             TEXT,
    last_status          TEXT,              -- "Success" | "Failed" | "Running" | "Skipped" | "Cancelled"
    next_run             TEXT,
    exclusions_json      TEXT DEFAULT '[]', -- ["*.tmp", "node_modules/**"]
    incremental          INTEGER DEFAULT 0,
    destination_type     TEXT DEFAULT 'Local',
    cloud_config_enc     TEXT,              -- AES-256-GCM şifreli S3Config JSON
    sftp_config_enc      TEXT,              -- AES-256-GCM şifreli SftpConfig JSON
    oauth_config_enc     TEXT,              -- AES-256-GCM şifreli OAuthConfig JSON
    webdav_config_enc    TEXT,              -- AES-256-GCM şifreli WebDavConfig JSON
    encrypt              INTEGER DEFAULT 0,
    encrypt_password_enc TEXT,              -- hw-key ile şifreli yedek şifresi
    encrypt_salt         TEXT               -- Argon2 salt (base64)
);
```

#### copy_logs
```sql
CREATE TABLE copy_logs (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    source_id        TEXT NOT NULL,
    destination_id   TEXT NOT NULL,
    source_path      TEXT NOT NULL,
    destination_path TEXT NOT NULL,
    started_at       TEXT NOT NULL,
    ended_at         TEXT,
    status           TEXT NOT NULL,
    bytes_copied     INTEGER,
    files_copied     INTEGER,
    error_message    TEXT,
    trigger          TEXT NOT NULL,     -- "Scheduled" | "OnChange" | "Manual"
    checksum         TEXT                -- "SHA-256: <hash>" | "N files verified"
);
-- İndeksler: source_id, destination_id, started_at, (source_id, started_at)
```

#### settings
```sql
CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT);
-- Anahtarlar: run_on_startup, minimize_to_tray, theme, log_retention_days,
--             language, sidebar_collapsed, onboarding_done,
--             license_key, license_validated_at
```

### 3.3 Migration Geçmişi

| Dosya | İçerik |
|-------|--------|
| `001_init.sql` | sources, destinations, copy_logs, settings tabloları + temel indeksler |
| `002_backup_quality.sql` | copy_logs'a `checksum` kolonu |
| `003_incremental.sql` | destinations'a `incremental` ve `exclusions_json` |
| `004_cloud.sql` | S3/SFTP şifreli config kolonları |
| `005_oauth.sql` | OAuth2 config kolonları |
| `006_vaults.sql` | vaults tablosu |
| `007_encryption.sql` | `encrypt`, `encrypt_password_enc`, `encrypt_salt` |
| `008_indexes.sql` | Performans indeksleri |
| `009_schema_versions.sql` | Schema sürüm takip tablosu |

---

## 4. API Tasarımı (Tauri Komutları)

Tüm frontend–backend iletişimi Tauri IPC üzerinden `invoke()` ile gerçekleşir. Rust tarafındaki komutlar `#[tauri::command]` + `#[specta::specta]` ile tanımlanır; TypeScript binding'leri `src/generated/bindings.ts` dosyasına otomatik üretilir.

> **Not:** `add_destination` ve `update_destination` komutları 15+ parametre içerdiğinden specta'nın SpectaFn limitini aştığı için `collect_commands!` dışında bırakılmış, `generate_handler!` ile çalışmaya devam etmektedir.

### 4.1 Komut Kategorileri (61 Toplam)

#### Sources (4 komut)
| Komut | Açıklama |
|-------|----------|
| `get_sources()` | Tüm kaynakları destinasyonlarıyla birlikte getirir |
| `create_source(name, path, source_type)` | Yeni kaynak oluşturur |
| `update_source(id, name, path, source_type, enabled)` | Kaynağı günceller, file watcher'ı yeniden başlatır |
| `delete_source(id)` | Kaynağı ve tüm destinasyonlarını siler |

#### Destinations (3 komut)
| Komut | Açıklama |
|-------|----------|
| `add_destination(source_id, path, schedule, retention, ...)` | Yeni hedef ekler |
| `update_destination(id, path, schedule, ...)` | Hedefi günceller, scheduler'ı iptal edip yeniler |
| `delete_destination(id)` | Hedefi siler, OnChange ise watcher'ı yeniden başlatır |

#### Jobs / Zamanlayıcı (4 komut)
| Komut | Açıklama |
|-------|----------|
| `run_now(destination_id)` | Tek hedef için yedeklemeyi anında tetikler |
| `run_source_now(source_id)` | Kaynağın tüm hedeflerini çalıştırır |
| `pause_all()` | Tüm zamanlanmış görevleri duraklatır |
| `resume_all()` | Duraklatılan görevleri devam ettirir |

#### Logs (3 komut)
| Komut | Açıklama |
|-------|----------|
| `get_logs(source_id?, destination_id?, status?, limit, offset)` | Sayfalı log getirir (50/sayfa) |
| `get_log_count(source_id?)` | Toplam log sayısı |
| `clear_old_logs(older_than_days)` | Eski logları temizler |

#### Settings (5 komut)
| Komut | Açıklama |
|-------|----------|
| `get_settings()` | Tüm uygulama ayarlarını döner |
| `update_settings(settings)` | Tüm ayarları günceller |
| `get_setting_value(key)` | Tek ayar değeri okur |
| `set_setting_value(key, value)` | Tek ayar yazar |
| `get_schema_version()` | Veritabanı şema versiyonu |

#### Dosya Sistemi (5 komut)
| Komut | Açıklama |
|-------|----------|
| `pick_directory()` | Sistem klasör seçici dialog'u |
| `pick_file()` | Sistem dosya seçici dialog'u |
| `get_disk_info(path)` | Disk kapasitesi bilgisi (total, used, free) |
| `check_path_type(path)` | Yolun dosya mı klasör mü olduğunu döner |
| `open_path(path)` | OS varsayılan uygulamasıyla açar |

#### Lisans (8 komut)
| Komut | Açıklama |
|-------|----------|
| `get_hardware_id()` | Cihaza özgü ID üretir (HW-XXXXXXXX) |
| `activate_license(key)` | Lisans anahtarını sunucuda aktifleştirir |
| `validate_license()` | Lisansın geçerliliğini doğrular (rate-limited, 60s) |
| `store_license(key)` | Şifreli olarak depolar |
| `get_stored_license()` | Kayıtlı lisans anahtarını döner |
| `clear_license()` | Lisans kaydını siler |
| `deactivate_license()` | Sunucuda deaktive eder, yerel kaydı siler |

#### Güncelleme (2 komut)
| Komut | Açıklama |
|-------|----------|
| `check_update()` | GitHub Releases'dan güncelleme kontrolü |
| `install_update()` | Mevcut güncellemeyi indirir ve yükler |

#### Config Export/Import (2 komut)
| Komut | Açıklama |
|-------|----------|
| `export_config()` | Tüm kaynakları, hedefleri ve ayarları JSON olarak dışa aktarır |
| `import_config()` | JSON config dosyasını içe aktarır |

#### Bulut Bağlantı Testleri (4 komut)
| Komut | Açıklama |
|-------|----------|
| `test_cloud_connection(config)` | S3/R2 bağlantısını doğrular |
| `test_sftp_connection(config)` | SFTP sunucusunu test eder |
| `test_webdav_connection(config)` | WebDAV endpoint'ini test eder |
| `test_oauth_connection(config)` | OAuth2 token geçerliliğini doğrular |

#### OAuth2 Akışı (2 komut)
| Komut | Açıklama |
|-------|----------|
| `run_oauth_flow(provider, client_id, folder_path)` | PKCE akışını başlatır, token alır |
| `test_oauth_connection(config)` | Mevcut token ile bağlantıyı doğrular |

#### Yedek İşlemleri (3 komut)
| Komut | Açıklama |
|-------|----------|
| `preview_backup(destination_id)` | Kopyalanacak dosyaları listeler |
| `restore_backup(backup_path, restore_to)` | Yedekten geri yükler |
| `decrypt_backup(folder_path, password)` | Şifreli yedeği çözer |

#### Vault (17 komut)
| Komut | Açıklama |
|-------|----------|
| `create_vault(name, password, algorithm?)` | Yeni şifreli kasa oluşturur |
| `list_vaults()` | Tüm kasaları listeler |
| `unlock_vault(vault_id, password)` | Kasayı açar, anahtarı hafızada saklar |
| `lock_vault(vault_id)` | Kasayı kilitler, hafızadaki anahtarı temizler |
| `list_entries(vault_id, parent_id?)` | Klasör içeriğini listeler |
| `import_file_cmd(vault_id, src_path, parent_id?)` | Dosyayı kasaya şifreli olarak ekler |
| `import_directory_cmd(vault_id, src_path, parent_id?)` | Klasörü yinelemeli olarak içe aktarır |
| `export_file_cmd(vault_id, entry_id, dest_path)` | Dosyayı çözer ve dışa aktarır |
| `open_file_cmd(vault_id, entry_id)` | Geçici dizinde çözer, OS uygulamasıyla açar |
| `create_directory_cmd(vault_id, name, parent_id?)` | Kasa içinde klasör oluşturur |
| `rename_entry_cmd(vault_id, entry_id, new_name)` | Dosya/klasör adını değiştirir |
| `move_entry_cmd(vault_id, entry_id, new_parent_id?)` | Taşır |
| `delete_entry_cmd(vault_id, entry_id)` | Güvenli siler |
| `get_thumbnail(vault_id, entry_id)` | Görsel önizleme (base64) döner |
| `change_vault_password(vault_id, old, new)` | Şifreyi değiştirir, tüm dosyaları yeniden şifreler |
| `delete_vault(vault_id, password)` | Kasayı ve tüm içeriği siler |
| `get_open_files(vault_id)` | Geçici açık dosyaları listeler |
| `sync_and_lock_vault(vault_id, save)` | Değişiklikleri kaydeder ve kilitler |

#### Diğer (3 komut)
| Komut | Açıklama |
|-------|----------|
| `send_test_email(to)` | Test e-posta bildirimi gönderir |
| `rebuild_app_menu(lang)` | Dil değişikliği sonrası native menüyü yeniler |

### 4.2 TypeScript Binding Kullanımı

`src/generated/bindings.ts` otomatik üretilir ve tüm komutları güçlü tiplerle sarmalar:

```typescript
import * as commands from "./generated/bindings";

// Tüm kaynakları getir
const sources = await commands.getSources();

// Lisans aktifleştir
const result = await commands.activateLicense("SV-XXXX-XXXX");
// result: { success: boolean, error: string | null }

// Vault oluştur
const vault = await commands.createVault("Personal", "my-password", "AES-256-GCM");
```

### 4.3 Event Bus

Kopyalama motorundan frontend'e gönderilen Tauri event'leri:

| Event | Payload | Açıklama |
|-------|---------|----------|
| `copy-started` | `{ destination_id }` | Kopyalama başladı |
| `copy-progress` | `{ destination_id, files_done, total_files, bytes_done }` | İlerleme güncellemesi |
| `copy-completed` | `{ destination_id, files_copied, bytes_copied }` | Başarıyla tamamlandı |
| `copy-error` | `{ destination_id, error }` | Hata oluştu |
| `scheduler-status` | `{ paused: bool }` | Scheduler duraklatıldı/devam etti |
| `watcher-warning` | `{ message }` | Dosya izleyici uyarısı |
| `license-activated` | `{ key }` | Deep link ile lisans aktifleştirildi |

---

## 5. Loglama, İzlenebilirlik ve Hata Yönetimi

### 5.1 Backend Logging

Rust tarafında `log` + `env_logger` crate'leri ile yapılandırılmış günlükleme:

```
[INFO]  Using database at: /Users/.../shadowvault.db
[INFO]  Scheduler: loaded 3 destinations
[WARN]  FileWatcher: could not watch path /Volumes/... (no such file)
[ERROR] Copy failed for destination abc123: permission denied
```

Log seviyesi `RUST_LOG` ortam değişkeniyle ayarlanır.

### 5.2 Kopyalama Motoru Olayları

Motor her önemli adımda Tauri event gönderir. Bu event'ler `src/store/index.ts`'deki `listen` çağrılarıyla yakalanır:

```typescript
listen("copy-progress", (event) => {
  setStore("copyProgress", event.payload.destination_id, event.payload);
});

listen("copy-error", (event) => {
  setStore("runningJobs", prev => { prev.delete(event.payload.destination_id); return prev; });
  toast.error(`Yedekleme hatası: ${event.payload.error}`);
});
```

### 5.3 Frontend Hata Yönetimi

- **ErrorBoundary:** `App.tsx` tüm bileşen ağacını sarmalar; beklenmedik hatalar `ErrorFallback` bileşeni ile kullanıcıya gösterilir.
- **Toast bildirimleri:** `solid-sonner` ile tüm başarı, uyarı ve hata durumları anlık olarak gösterilir.
- **Watcher uyarıları:** `watcherWarning` store alanı UI'da bannerlı uyarı olarak gösterilir.

### 5.4 Yedekleme Günlükleri

Her yedekleme çalıştırması `copy_logs` tablosuna kayıt düşer:

```
[2026-03-16 14:30:00] SUCCESS  Documents → /Volumes/Backup/docs  42 dosya, 128 MB  SHA-256: a3f2...
[2026-03-16 14:30:01] FAILED   Photos → s3://my-bucket/photos    Hata: Connection timeout
```

Loglar `LOG_PAGE_SIZE=50` ile sayfalandırılır ve `Logs` sayfasında kaynak, hedef, durum ve tarih aralığı filtreleriyle görüntülenebilir.

---

## 6. Güvenlik Mimarisi

### 6.1 Hardware-ID Tabanlı Lisanslama

Lisans, cihaza özgü bir donanım kimliğine bağlıdır:

```rust
// hostname + total_memory + cpu_count → SHA-256 → UUID v5
pub fn hw_id_raw() -> String {
    format!("shadowvault:{}:{}:{}", hostname, total_memory, cpu_count)
}
// Kullanıcıya gösterilen format: HW-XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX
```

### 6.2 Şifreli Depolama (crypto_utils.rs)

Tüm hassas veriler (lisans anahtarı, bulut kimlik bilgileri, yedek şifresi) donanım anahtarıyla AES-256-GCM şifreli olarak SQLite'a yazılır:

```
hw_id_raw() → SHA-256 → 32-byte master key
    ↓
hw_encrypt(plaintext) → random nonce + AES-256-GCM → Base64
```

### 6.3 Vault Şifreleme Akışı

```
Kullanıcı şifresi
    ↓ Argon2id(salt, m=65536, t=3, p=4)
derived_key (32 byte)
    ├── meta_key = HKDF(derived_key, "meta")   → .shadow_meta şifreler
    └── file_key = HKDF(derived_key, file_id)  → Her dosya bağımsız anahtar
        ↓ AES-256-GCM | ChaCha20-Poly1305 | XChaCha20-Poly1305
       ciphertext + auth_tag
```

### 6.4 CSP (Content Security Policy)

`tauri.conf.json` ile kısıtlı CSP:

```
default-src 'self' ipc: http://ipc.localhost
script-src 'self'
style-src 'self' 'unsafe-inline'
img-src 'self' data: blob:
connect-src ipc: http://ipc.localhost
```

### 6.5 Diğer Güvenlik Önlemleri

- Sistem dizinlerine yedekleme engeli (`/System`, `C:\Windows`, vb.)
- Symlink saldırı koruması
- Güvenli geçici dosya silme (vault dosyaları için)
- Rate-limited lisans doğrulama (60 saniye önbellek)
- Offline grace mode (ağ erişilemez olduğunda mevcut lisans geçerli sayılır)
- Lisans anahtarı loglara yazılmaz

---

## 7. Test Stratejisi

### 7.1 Frontend Testleri

`Vitest` + `@solidjs/testing-library` + `jsdom` ile:

```bash
npm run test          # Tüm testleri çalıştır
npm run test:watch    # Değişikliğe duyarlı mod
```

**`src/test/schedule.test.ts`** — 5 test
- Interval / Cron / OnChange / Manual varsayılan değerleri
- Schedule label formatlaması

**`src/test/api.test.ts`** — 4 test
- API katmanının doğru Tauri komutlarını çağırdığı
- Parametre geçişleri
- `vi.mock("@tauri-apps/api/core")` ile Tauri mock'laması

### 7.2 Backend Testleri

`cargo test` ile:

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

**`src-tauri/src/engine/scheduler.rs`** — 3 entegrasyon testi
- Cron ifadesinden süre hesaplama
- Geçersiz expression için fallback davranışı

**`src-tauri/src/lib.rs`** — Binding üretimi testi
```bash
cargo test export_bindings --manifest-path src-tauri/Cargo.toml
```

### 7.3 Test Mock Yapısı

```typescript
// src/test/setup.ts
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

beforeEach(() => {
  vi.clearAllMocks();
});
```

---

## 8. Kurulum ve Çalıştırma

### 8.1 Gereksinimler

| Araç | Versiyon | Notlar |
|------|----------|--------|
| Node.js | 20+ (LTS) | `nvm use 20` ile |
| Rust | stable | `rustup update stable` |
| Tauri CLI | 2.x | `cargo install tauri-cli` |
| macOS | 12+ | Xcode CLT gerekli |
| Windows | 10/11 | WebView2 Runtime |
| Linux | Debian/Arch | `libwebkit2gtk-4.1`, `libappindicator3` |

### 8.2 Ortam Değişkenleri

Ortam değişkeni gerekmez; Tauri konfigürasyonu `tauri.conf.json` içindedir.

Geliştirme sırasında opsiyonel:

```bash
RUST_LOG=info          # Backend log seviyesi (trace/debug/info/warn/error)
SHADOWVAULT_DB_PATH=   # Özel veritabanı yolu (test için kullanışlı)
```

### 8.3 Geliştirme Ortamı

```bash
# Bağımlılıkları yükle
npm install

# Tauri + Vite birlikte başlat
npm run tauri dev
```

### 8.4 Üretim Derlemesi

```bash
# Frontend derle
npm run build

# Platforma özgü installer oluştur
npm run tauri build
# Çıktı: src-tauri/target/release/bundle/
#   macOS: *.dmg, *.app
#   Windows: *.exe (NSIS), *.msi
#   Linux: *.deb, *.AppImage
```

### 8.5 Komutlar Özeti

```bash
npm install          # Bağımlılıkları yükle
npm run dev          # Yalnızca Vite web sunucusu
npm run tauri dev    # Tam masaüstü geliştirme modu
npm run build        # Frontend derle
npm run tauri build  # Platforma özgü paket oluştur
npm run test         # Frontend testleri çalıştır
npm run test:watch   # İzleme modunda test

# TypeScript binding'lerini yenile (Rust değişikliği sonrası)
cargo test export_bindings --manifest-path src-tauri/Cargo.toml
```

### 8.6 Yeni Tauri Komutu Ekleme Akışı

1. `src-tauri/src/commands/<module>.rs` içine `#[tauri::command]` + `#[specta::specta]` ile yaz
2. `src-tauri/src/lib.rs` içindeki `specta_builder()` ve `generate_handler![]` listelerine ekle
3. `cargo test export_bindings` ile binding'leri yenile
4. `src/api/tauri.ts` içine typed wrapper ekle
5. `src/store/index.ts` veya ilgili sayfada kullan

### 8.7 Deep Link Testi (Lisans Aktivasyonu)

```bash
# macOS — LemonSqueezy aktivasyon linkini simüle et
open "shadowvault://activate?key=SV-TEST-1234"
```

---

## 9. Frontend

### 9.1 Sayfa Yapısı

| Sayfa | Dosya | Açıklama |
|-------|-------|----------|
| **Dashboard** | `pages/Dashboard.tsx` | İstatistik kartları, aktif işler, kopyalama ilerleme |
| **Sources** | `pages/Sources.tsx` | Kaynak ve hedef yönetimi, sıralama/filtreleme |
| **Logs** | `pages/Logs.tsx` | Sayfalı yedekleme günlükleri, durum filtreleri |
| **Settings** | `pages/Settings.tsx` | Tema, dil, başlatma, log tutma, config import/export |
| **License** | `pages/LicensePage.tsx` | Lisans aktivasyon ve donanım ID görüntüleme |
| **Vault** | `pages/VaultPage.tsx` | Şifreli kasa explorer'ı, dosya yönetimi |

### 9.2 Bileşen Mimarisi

```
src/components/
├── layout/
│   ├── Layout.tsx          — Ana wrapper (sidebar + topbar + içerik)
│   ├── TopBar.tsx          — Üst bar (marka, duraklatma butonu)
│   └── Sidebar.tsx         — Navigasyon (daraltılabilir)
├── ui/
│   ├── Button.tsx, Toggle.tsx, Badge.tsx, Modal.tsx
│   ├── AboutModal.tsx      — Uygulama bilgisi
│   ├── OnboardingModal.tsx — 3 adımlı ilk kurulum sihirbazı
│   └── ErrorFallback.tsx   — Hata sınırı fallback
├── sources/
│   ├── SourceCard.tsx, SourceList.tsx
│   ├── AddSourceModal.tsx, EditSourceModal.tsx
├── destinations/
│   ├── DestinationList.tsx
│   ├── AddDestinationModal.tsx, EditDestinationModal.tsx
│   └── PreviewModal.tsx    — Kopyalanacak dosya önizleme
├── schedule/
│   └── SchedulePicker.tsx  — Interval / Cron / OnChange / Manual seçici
├── logs/
│   └── LogPanel.tsx        — Log satırları ve filtreleme
└── vault/
    ├── VaultExplorer.tsx, VaultSidebar.tsx
    ├── VaultEntryGrid.tsx, VaultEntryList.tsx
    └── VaultContextMenu.tsx
```

### 9.3 State Yönetimi

`createStore` ile yönetilen global store:

```typescript
interface AppStore {
  sources: Source[];
  logs: LogEntry[];
  logTotal: number;
  settings: AppSettings | null;
  isSchedulerPaused: boolean;
  runningJobs: Set<string>;
  isLoading: boolean;
  activePage: "dashboard" | "sources" | "logs" | "settings" | "license" | "vault";
  watcherWarning: string | null;
  licenseStatus: "checking" | "valid" | "invalid";
  sidebarCollapsed: boolean;
  copyProgress: Record<string, CopyProgress>;
}
```

### 9.4 Uluslararasılaştırma (i18n)

`src/i18n/index.ts` içinde Türkçe (varsayılan) ve İngilizce tam çeviri desteği:

```typescript
const t = useTranslation(); // Aktif dili otomatik seçer
t("sources.addSource")      // "Kaynak Ekle" | "Add Source"
t("logs.status.success")    // "Başarılı" | "Success"
```

Dil ayarı `settings` tablosundaki `language` anahtarıyla kalıcı hale getirilir.

### 9.5 Klavye Kısayolları ve Native Menü

| Kısayol | Eylem |
|---------|-------|
| `Cmd/Ctrl + N` | Yeni kaynak oluştur |
| `Cmd/Ctrl + R` | Tüm kaynakları çalıştır |
| `Cmd + \` | Kenar çubuğunu aç/kapat |

Native menü `menu.rs`'de oluşturulur ve dil değişikliğinde `rebuild_app_menu` komutuyla yenilenir.

---

## 10. Production Notes

### 10.1 Kod İmzalama

Dağıtım için platform kod imzalama sertifikaları gereklidir:

- **macOS:** Apple Developer ID (`tauri.conf.json` → `bundle.macOS.signingIdentity`)
- **Windows:** Code Signing Certificate (Tauri `TAURI_SIGNING_*` env)

### 10.2 Otomatik Güncelleme

Güncelleme endpoint'i:

```
https://github.com/realsarius/shadowvault/releases/latest/download/latest.json
```

`latest.json` formatı Tauri Updater standartlarına uygun olmalıdır. `check_update()` komutu bu URL'yi sorgular; `install_update()` paketi indirir ve yeniden başlatır.

### 10.3 Lisans API

Lisans doğrulama ve aktivasyon için backend API:

- **Base URL:** `https://license.berkansozer.com`
- **Aktivasyon:** `POST /licenses/activate` → `{ key, hardware_id }`
- **Doğrulama:** `POST /licenses/validate` → `{ key, hardware_id }`
- **Deaktivasyon:** `POST /licenses/deactivate` → `{ key, hardware_id }`

Ağ erişimi olmadığında `validate_license()` `{ status: "valid", offline: true }` döner.

### 10.4 Vault Temp Dosya Yönetimi

`open_file_cmd` ile açılan dosyalar:
1. Sistem geçici dizinine şifresiz kopyalanır
2. OS varsayılan uygulamasıyla açılır
3. Pencere kapanmadan önce otomatik yeniden şifrelenir
4. Geçici kopya güvenli olarak silinir

`sync_open_vault_files()` bu akışı pencere `CloseRequested` event'inde otomatik çalıştırır.

### 10.5 Operasyonel Kontroller

Uygulama sağlık doğrulaması için:

```bash
# Uygulama çalışıyor mu?
pgrep -f "shadowvault" && echo "Çalışıyor" || echo "Durdu"

# Veritabanı bütünlüğü (development)
sqlite3 ~/Library/Application\ Support/com.shadowvault.app/shadowvault.db "PRAGMA integrity_check;"

# Log tablosundaki son 10 kayıt
sqlite3 ~/Library/Application\ Support/com.shadowvault.app/shadowvault.db \
  "SELECT started_at, status, error_message FROM copy_logs ORDER BY started_at DESC LIMIT 10;"
```

---

## 11. Lisans ve Kullanım Notu

Bu repo açık kaynak olarak lisanslanmamıştır.

- Kaynak kod ve ilişkili materyaller **All Rights Reserved** kapsamında korunur.
- Yazılı izin olmadan kodun kopyalanması, yeniden kullanılması, dağıtılması, türev iş üretilmesi veya üretim ortamında kullanılması yasaktır.
- Kullanılan üçüncü parti paketler kendi lisans koşullarına tabidir (`package.json` ve `Cargo.toml` bağımlılıklarının kaynak repolarına bakınız).

Detaylı metin için kök dizindeki [`LICENSE`](LICENSE) dosyasına bakabilirsiniz.
