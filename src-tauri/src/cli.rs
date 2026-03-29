//! ShadowVault CLI — headless mod

use clap::{Args, Parser, Subcommand, ValueEnum};
use std::sync::Arc;

use crate::db;
use crate::db::queries;
use crate::engine::block::snapshot::BackupLevel;
use crate::engine::copier::CopyJob;
use crate::vault::crypto::derive_key;
use crate::vault::fs::{move_entry, EntryKind, VaultEntry, VaultMeta};

// ── CLI tanımı ─────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "shadowvault", about = "ShadowVault CLI", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Kaynaklarla ilgili işlemler
    Sources {
        #[command(subcommand)]
        cmd: SourcesSubcmd,
    },
    /// Hedeflerle ilgili işlemler
    Destinations {
        #[command(subcommand)]
        cmd: DestinationsSubcmd,
    },
    /// Yedek loglarını göster
    Logs(LogsCmd),
    /// Tüm kaynak/hedeflerin son durumunu göster
    Status(StatusCmd),
    /// Bir hedefi hemen yedekle
    RunNow(RunNowCmd),
    /// Şifreli kasa işlemleri
    Vault {
        #[command(subcommand)]
        cmd: VaultSubcmd,
    },
}

#[derive(Subcommand)]
pub enum SourcesSubcmd {
    /// Kaynak listesini göster
    List {
        #[arg(long, help = "JSON formatında çıktı")]
        json: bool,
    },
}

#[derive(Subcommand)]
pub enum DestinationsSubcmd {
    /// Hedef listesini göster
    List {
        #[arg(long, help = "Belirli bir kaynağa ait hedefleri filtrele")]
        source_id: Option<String>,
        #[arg(long, help = "JSON formatında çıktı")]
        json: bool,
    },
}

#[derive(Args)]
pub struct LogsCmd {
    #[arg(long, default_value_t = 20, help = "Gösterilecek log sayısı")]
    pub limit: i64,
    #[arg(long, help = "Duruma göre filtrele (Success, Failed, Skipped...)")]
    pub status: Option<String>,
    #[arg(long, help = "Kaynak ID'ye göre filtrele")]
    pub source_id: Option<String>,
    #[arg(long, help = "Hedef ID'ye göre filtrele")]
    pub destination_id: Option<String>,
    #[arg(long, help = "JSON formatında çıktı")]
    pub json: bool,
}

#[derive(Args)]
pub struct StatusCmd {
    #[arg(long, help = "JSON formatında çıktı")]
    pub json: bool,
}

#[derive(Args)]
pub struct RunNowCmd {
    #[arg(long, help = "Yedeklenecek hedefin ID'si")]
    pub destination_id: String,
    #[arg(long, value_enum, help = "Yedek seviyesi (varsayılan: otomatik)")]
    pub level: Option<BackupLevelArg>,
    #[arg(long, help = "JSON formatında çıktı")]
    pub json: bool,
}

#[derive(Clone, ValueEnum)]
pub enum BackupLevelArg {
    Level0,
    Level1Cumulative,
    Level1Differential,
}

#[derive(Subcommand)]
pub enum VaultSubcmd {
    /// Tüm kasaları listele
    List {
        #[arg(long)]
        json: bool,
    },
    /// Kasa içeriğini göster
    Ls {
        #[arg(long, help = "Kasa ID'si")]
        id: String,
        #[arg(long, help = "Kasa şifresi (yoksa VAULT_PASSWORD env kullanılır)")]
        password: Option<String>,
        #[arg(long, help = "Klasör ID'si (boşsa kök dizin)")]
        parent: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Bir girişi klasöre taşı
    Mv {
        #[arg(long, help = "Kasa ID'si")]
        id: String,
        #[arg(long, help = "Kasa şifresi")]
        password: Option<String>,
        #[arg(long, help = "Taşınacak giriş ID'si")]
        entry: String,
        #[arg(long, help = "Hedef klasör ID'si (boşsa kök dizine taşı)")]
        to: Option<String>,
    },
    /// Yeni klasör oluştur
    Mkdir {
        #[arg(long, help = "Kasa ID'si")]
        id: String,
        #[arg(long, help = "Kasa şifresi")]
        password: Option<String>,
        #[arg(long, help = "Klasör adı")]
        name: String,
        #[arg(long, help = "Üst klasör ID'si (boşsa kök dizin)")]
        parent: Option<String>,
    },
}

// ── DB yolu çözümleme ───────────────────────────────────────────────────────

/// CLI için DB yolunu belirler.
/// Öncelik: SHADOWVAULT_DB_PATH env → OS varsayılan yolu → hata.
pub fn resolve_db_path() -> Result<String, String> {
    if let Ok(p) = std::env::var("SHADOWVAULT_DB_PATH") {
        return Ok(p);
    }

    let candidates = os_db_candidates();
    for path in &candidates {
        if std::path::Path::new(path).exists() {
            return Ok(path.clone());
        }
    }

    Err(format!(
        "Veritabanı bulunamadı.\n\
         Kontrol edilen yollar:\n{}\n\n\
         Çözüm: SHADOWVAULT_DB_PATH=/yol/shadowvault.db shadowvault ...",
        candidates.join("\n  - ")
    ))
}

#[cfg(target_os = "macos")]
fn os_db_candidates() -> Vec<String> {
    let home = std::env::var("HOME").unwrap_or_default();
    vec![format!(
        "{}/Library/Application Support/com.shadowvault.app/shadowvault.db",
        home
    )]
}

#[cfg(target_os = "linux")]
fn os_db_candidates() -> Vec<String> {
    let home = std::env::var("HOME").unwrap_or_default();
    let xdg = std::env::var("XDG_DATA_HOME").unwrap_or_else(|_| format!("{}/.local/share", home));
    vec![format!("{}/com.shadowvault.app/shadowvault.db", xdg)]
}

#[cfg(target_os = "windows")]
fn os_db_candidates() -> Vec<String> {
    let appdata = std::env::var("APPDATA").unwrap_or_default();
    vec![format!(
        "{}\\com.shadowvault.app\\data\\shadowvault.db",
        appdata
    )]
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn os_db_candidates() -> Vec<String> {
    vec![]
}

// ── Ana giriş noktası ───────────────────────────────────────────────────────

pub fn run(cli: Cli) -> i32 {
    let rt = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Tokio runtime başlatılamadı: {}", e);
            return 1;
        }
    };
    rt.block_on(async { dispatch(cli).await })
}

async fn dispatch(cli: Cli) -> i32 {
    let db_path = match resolve_db_path() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Hata: {}", e);
            return 1;
        }
    };

    let pool = match db::init_db(&db_path).await {
        Ok(p) => Arc::new(p),
        Err(e) => {
            eprintln!("Veritabanı açılamadı ({}): {}", db_path, e);
            return 1;
        }
    };

    match cli.command {
        Commands::Sources { cmd } => match cmd {
            SourcesSubcmd::List { json } => cmd_sources(&pool, json).await,
        },
        Commands::Destinations { cmd } => match cmd {
            DestinationsSubcmd::List { source_id, json } => {
                cmd_destinations(&pool, source_id, json).await
            }
        },
        Commands::Logs(cmd) => cmd_logs(&pool, cmd).await,
        Commands::Status(cmd) => cmd_status(&pool, cmd).await,
        Commands::RunNow(cmd) => cmd_run_now(&pool, cmd).await,
        Commands::Vault { cmd } => cmd_vault(&pool, cmd).await,
    }
}

// ── sources list ────────────────────────────────────────────────────────────

async fn cmd_sources(pool: &sqlx::SqlitePool, json: bool) -> i32 {
    let sources = match queries::get_all_sources(pool).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Kaynaklar alınamadı: {}", e);
            return 1;
        }
    };

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&sources).unwrap_or_default()
        );
        return 0;
    }

    if sources.is_empty() {
        println!("Kaynak bulunamadı.");
        return 0;
    }

    println!("{:<38} {:<20} {:<8} {}", "ID", "Ad", "Aktif", "Yol");
    println!("{}", "-".repeat(90));
    for s in &sources {
        println!(
            "{:<38} {:<20} {:<8} {}",
            s.id,
            truncate(&s.name, 19),
            if s.enabled { "evet" } else { "hayır" },
            s.path
        );
    }
    println!("\nToplam: {} kaynak", sources.len());
    0
}

// ── destinations list ───────────────────────────────────────────────────────

async fn cmd_destinations(pool: &sqlx::SqlitePool, source_id: Option<String>, json: bool) -> i32 {
    let sources = match queries::get_all_sources(pool).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Kaynaklar alınamadı: {}", e);
            return 1;
        }
    };

    let mut all_dests = Vec::new();
    for src in &sources {
        if let Some(ref filter_id) = source_id {
            if &src.id != filter_id {
                continue;
            }
        }
        for dest in &src.destinations {
            all_dests.push((src.name.clone(), src.id.clone(), dest.clone()));
        }
    }

    if json {
        let json_dests: Vec<_> = all_dests.iter().map(|(_, _, d)| d).collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&json_dests).unwrap_or_default()
        );
        return 0;
    }

    if all_dests.is_empty() {
        println!("Hedef bulunamadı.");
        return 0;
    }

    println!(
        "{:<38} {:<18} {:<10} {:<12} {}",
        "ID", "Kaynak", "Aktif", "Son Durum", "Yol"
    );
    println!("{}", "-".repeat(100));
    for (src_name, _, dest) in &all_dests {
        println!(
            "{:<38} {:<18} {:<10} {:<12} {}",
            dest.id,
            truncate(src_name, 17),
            if dest.enabled { "evet" } else { "hayır" },
            dest.last_status
                .as_ref()
                .map(|s| format!("{:?}", s))
                .unwrap_or_else(|| "-".into()),
            dest.path
        );
    }
    println!("\nToplam: {} hedef", all_dests.len());
    0
}

// ── logs ────────────────────────────────────────────────────────────────────

async fn cmd_logs(pool: &sqlx::SqlitePool, cmd: LogsCmd) -> i32 {
    let logs = match queries::get_logs(
        pool,
        cmd.source_id.as_deref(),
        cmd.destination_id.as_deref(),
        cmd.status.as_deref(),
        None,
        None,
        None,
        Some(cmd.limit),
        None,
    )
    .await
    {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Loglar alınamadı: {}", e);
            return 1;
        }
    };

    if cmd.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&logs).unwrap_or_default()
        );
        return 0;
    }

    if logs.is_empty() {
        println!("Log bulunamadı.");
        return 0;
    }

    println!(
        "{:<6} {:<17} {:<7} {:<8} {:<10} {:<30} {}",
        "ID", "Başlangıç", "Süre", "Dosyalar", "Boyut", "Hedef", "Durum/Hata"
    );
    println!("{}", "-".repeat(110));
    for log in &logs {
        let started = log.started_at.format("%Y-%m-%d %H:%M").to_string();
        let duration = match log.ended_at {
            Some(ended) => {
                let secs = (ended - log.started_at).num_seconds();
                if secs < 60 {
                    format!("{}s", secs)
                } else {
                    format!("{}dk{}s", secs / 60, secs % 60)
                }
            }
            None => "-".into(),
        };
        let size = log
            .bytes_copied
            .map(human_bytes)
            .unwrap_or_else(|| "-".into());
        let files = log
            .files_copied
            .map(|f| f.to_string())
            .unwrap_or_else(|| "-".into());
        let dest = truncate(&log.destination_path, 29);
        let status_or_err = match &log.error_message {
            Some(e) => truncate(e, 40),
            None => log.status.clone(),
        };
        println!(
            "{:<6} {:<17} {:<7} {:<8} {:<10} {:<30} {}",
            log.id, started, duration, files, size, dest, status_or_err
        );
    }
    println!("\nToplam: {} log gösterildi", logs.len());
    0
}

// ── status ──────────────────────────────────────────────────────────────────

async fn cmd_status(pool: &sqlx::SqlitePool, cmd: StatusCmd) -> i32 {
    let sources = match queries::get_all_sources(pool).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Kaynaklar alınamadı: {}", e);
            return 1;
        }
    };

    if cmd.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&sources).unwrap_or_default()
        );
        return 0;
    }

    if sources.is_empty() {
        println!("Kaynak bulunamadı.");
        return 0;
    }

    for src in &sources {
        println!(
            "\n[{}] {} ({})",
            if src.enabled { "+" } else { "-" },
            src.name,
            src.path
        );
        for dest in &src.destinations {
            let status = dest
                .last_status
                .as_ref()
                .map(|s| format!("{:?}", s))
                .unwrap_or_else(|| "henüz çalışmadı".into());
            let last_run = dest
                .last_run
                .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "-".into());
            println!(
                "  {} {} → {} [{}] son: {}",
                if dest.enabled { "▶" } else { "■" },
                dest.id,
                dest.path,
                status,
                last_run
            );
        }
    }
    0
}

// ── run-now ─────────────────────────────────────────────────────────────────

async fn cmd_run_now(pool: &sqlx::SqlitePool, cmd: RunNowCmd) -> i32 {
    let dest = match queries::get_destination_by_id(pool, &cmd.destination_id).await {
        Ok(Some(d)) => d,
        Ok(None) => {
            eprintln!("Hata: Destination '{}' bulunamadı.", cmd.destination_id);
            return 2;
        }
        Err(e) => {
            eprintln!("Veritabanı hatası: {}", e);
            return 1;
        }
    };

    let source = match queries::get_source_by_id(pool, &dest.source_id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            eprintln!("Hata: Source '{}' bulunamadı.", dest.source_id);
            return 2;
        }
        Err(e) => {
            eprintln!("Veritabanı hatası: {}", e);
            return 1;
        }
    };

    let backup_level = cmd.level.map(|l| match l {
        BackupLevelArg::Level0 => BackupLevel::Level0,
        BackupLevelArg::Level1Cumulative => BackupLevel::Level1Cumulative,
        BackupLevelArg::Level1Differential => BackupLevel::Level1Differential,
    });

    if !cmd.json {
        eprintln!("Yedekleme başlatılıyor: {} → {}", source.path, dest.path);
    }

    let job = CopyJob {
        source,
        destination: dest,
        trigger: "CLI".to_string(),
        app: None,
        backup_level,
    };

    match job.execute(Arc::new(pool.clone())).await {
        Ok(log_entry) => {
            if cmd.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&log_entry).unwrap_or_default()
                );
            } else {
                eprintln!("Tamamlandı.");
                println!("Durum     : {}", log_entry.status);
                println!(
                    "Dosyalar  : {}",
                    log_entry
                        .files_copied
                        .map(|f| f.to_string())
                        .unwrap_or_else(|| "-".into())
                );
                println!(
                    "Boyut     : {}",
                    log_entry
                        .bytes_copied
                        .map(human_bytes)
                        .unwrap_or_else(|| "-".into())
                );
                if let Some(err) = &log_entry.error_message {
                    eprintln!("Hata      : {}", err);
                }
            }
            0
        }
        Err(e) => {
            if cmd.json {
                eprintln!("{}", serde_json::json!({ "error": e.to_string() }));
            } else {
                eprintln!("Yedekleme başarısız: {}", e);
            }
            1
        }
    }
}

// ── vault ───────────────────────────────────────────────────────────────────

async fn cmd_vault(pool: &sqlx::SqlitePool, cmd: VaultSubcmd) -> i32 {
    match cmd {
        VaultSubcmd::List { json } => vault_list(pool, json).await,
        VaultSubcmd::Ls {
            id,
            password,
            parent,
            json,
        } => vault_ls(pool, &id, password, parent.as_deref(), json).await,
        VaultSubcmd::Mv {
            id,
            password,
            entry,
            to,
        } => vault_mv(pool, &id, password, &entry, to.as_deref()).await,
        VaultSubcmd::Mkdir {
            id,
            password,
            name,
            parent,
        } => vault_mkdir(pool, &id, password, &name, parent.as_deref()).await,
    }
}

/// DB'den vault_path al, şifreyle master key türet.
async fn vault_open(
    pool: &sqlx::SqlitePool,
    vault_id: &str,
    password: Option<String>,
) -> Result<(std::path::PathBuf, VaultMeta), String> {
    let vault_path: String = sqlx::query_scalar("SELECT vault_path FROM vaults WHERE id = ?")
        .bind(vault_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Kasa '{}' bulunamadı.", vault_id))?;

    let vault_path = std::path::PathBuf::from(&vault_path);

    let salt_hex = std::fs::read_to_string(vault_path.join(".shadow_salt"))
        .map_err(|_| "Kasa salt dosyası bulunamadı.".to_string())?;
    let salt_bytes =
        hex::decode(salt_hex.trim()).map_err(|_| "Geçersiz salt formatı.".to_string())?;

    let pwd = password
        .or_else(|| std::env::var("VAULT_PASSWORD").ok())
        .ok_or_else(|| {
            "Şifre gerekli: --password <şifre> veya VAULT_PASSWORD env değişkeni kullanın."
                .to_string()
        })?;

    let master_key =
        derive_key(&pwd, &salt_bytes).map_err(|e| format!("Anahtar türetme hatası: {}", e))?;

    let meta = VaultMeta::load(&vault_path, &master_key)
        .map_err(|_| "Yanlış şifre veya bozuk kasa.".to_string())?;

    Ok((vault_path, meta))
}

async fn vault_list(pool: &sqlx::SqlitePool, json: bool) -> i32 {
    let rows: Vec<(String, String, String)> =
        sqlx::query_as("SELECT id, name, vault_path FROM vaults ORDER BY created_at DESC")
            .fetch_all(pool)
            .await
            .unwrap_or_default();

    if json {
        let out: Vec<_> = rows
            .iter()
            .map(|(id, name, path)| serde_json::json!({ "id": id, "name": name, "path": path }))
            .collect();
        println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
        return 0;
    }

    if rows.is_empty() {
        println!("Kasa bulunamadı.");
        return 0;
    }

    println!("{:<38} {:<20} {}", "ID", "Ad", "Yol");
    println!("{}", "-".repeat(90));
    for (id, name, path) in &rows {
        println!("{:<38} {:<20} {}", id, truncate(name, 19), path);
    }
    0
}

async fn vault_ls(
    pool: &sqlx::SqlitePool,
    vault_id: &str,
    password: Option<String>,
    parent: Option<&str>,
    json: bool,
) -> i32 {
    let (_, meta) = match vault_open(pool, vault_id, password).await {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Hata: {}", e);
            return 1;
        }
    };

    let entries: Vec<_> = meta
        .entries
        .iter()
        .filter(|e| e.parent_id.as_deref() == parent)
        .collect();

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&entries).unwrap_or_default()
        );
        return 0;
    }

    if entries.is_empty() {
        println!("Bu dizin boş.");
        return 0;
    }

    println!("{:<38} {:<12} {}", "ID", "Tür", "Ad");
    println!("{}", "-".repeat(70));
    for e in &entries {
        println!(
            "{:<38} {:<12} {}",
            e.id,
            if matches!(e.kind, EntryKind::Directory) {
                "📁 Klasör"
            } else {
                "📄 Dosya"
            },
            e.name
        );
    }
    println!("\nToplam: {} giriş", entries.len());
    0
}

async fn vault_mv(
    pool: &sqlx::SqlitePool,
    vault_id: &str,
    password: Option<String>,
    entry_id: &str,
    to: Option<&str>,
) -> i32 {
    let (vault_path, mut meta) = match vault_open(pool, vault_id, password.clone()).await {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Hata: {}", e);
            return 1;
        }
    };

    if let Err(e) = move_entry(&mut meta, entry_id, to) {
        eprintln!("Taşıma hatası: {}", e);
        return 1;
    }

    // Master key'i tekrar türet (save için gerekli)
    let salt_hex = std::fs::read_to_string(vault_path.join(".shadow_salt")).unwrap_or_default();
    let salt_bytes = hex::decode(salt_hex.trim()).unwrap_or_default();
    let pwd = password
        .or_else(|| std::env::var("VAULT_PASSWORD").ok())
        .unwrap_or_default();
    let master_key = match derive_key(&pwd, &salt_bytes) {
        Ok(k) => k,
        Err(e) => {
            eprintln!("Anahtar hatası: {}", e);
            return 1;
        }
    };

    if let Err(e) = meta.save(&vault_path, &master_key) {
        eprintln!("Kaydetme hatası: {}", e);
        return 1;
    }

    let dest = to.unwrap_or("(kök dizin)");
    println!("Taşındı: {} → {}", entry_id, dest);
    0
}

async fn vault_mkdir(
    pool: &sqlx::SqlitePool,
    vault_id: &str,
    password: Option<String>,
    name: &str,
    parent: Option<&str>,
) -> i32 {
    use uuid::Uuid;

    let (vault_path, mut meta) = match vault_open(pool, vault_id, password.clone()).await {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Hata: {}", e);
            return 1;
        }
    };

    let new_id = Uuid::new_v4().simple().to_string();
    meta.entries.push(VaultEntry {
        id: new_id.clone(),
        name: name.to_string(),
        kind: EntryKind::Directory,
        parent_id: parent.map(str::to_string),
        size: None,
        modified: Some(chrono::Utc::now().to_rfc3339()),
        nonce: None,
    });

    let salt_hex = std::fs::read_to_string(vault_path.join(".shadow_salt")).unwrap_or_default();
    let salt_bytes = hex::decode(salt_hex.trim()).unwrap_or_default();
    let pwd = password
        .or_else(|| std::env::var("VAULT_PASSWORD").ok())
        .unwrap_or_default();
    let master_key = match derive_key(&pwd, &salt_bytes) {
        Ok(k) => k,
        Err(e) => {
            eprintln!("Anahtar hatası: {}", e);
            return 1;
        }
    };

    if let Err(e) = meta.save(&vault_path, &master_key) {
        eprintln!("Kaydetme hatası: {}", e);
        return 1;
    }

    println!("Klasör oluşturuldu: {} ({})", name, new_id);
    0
}

// ── yardımcı fonksiyonlar ───────────────────────────────────────────────────

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}

fn human_bytes(bytes: i64) -> String {
    const KB: i64 = 1024;
    const MB: i64 = KB * 1024;
    const GB: i64 = MB * 1024;
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
