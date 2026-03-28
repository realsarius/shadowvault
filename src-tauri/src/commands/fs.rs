use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
#[specta::specta]
pub async fn pick_directory(app: AppHandle) -> Result<Option<String>, String> {
    let path = app.dialog().file().blocking_pick_folder();

    match path {
        Some(file_path) => {
            let path_str = file_path
                .as_path()
                .map(|p| p.to_string_lossy().to_string())
                .or_else(|| Some(file_path.to_string()))
                .unwrap_or_default();
            Ok(Some(path_str))
        }
        None => Ok(None),
    }
}

#[tauri::command]
#[specta::specta]
pub async fn pick_file(app: AppHandle) -> Result<Option<String>, String> {
    let path = app.dialog().file().blocking_pick_file();

    match path {
        Some(file_path) => {
            let path_str = file_path
                .as_path()
                .map(|p| p.to_string_lossy().to_string())
                .or_else(|| Some(file_path.to_string()))
                .unwrap_or_default();
            Ok(Some(path_str))
        }
        None => Ok(None),
    }
}

#[derive(Serialize, specta::Type)]
pub struct DiskInfo {
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub path: String,
}

#[tauri::command]
#[specta::specta]
pub async fn check_path_type(path: String) -> Result<String, String> {
    let p = std::path::Path::new(&path);
    if p.is_dir() {
        Ok("Directory".to_string())
    } else if p.is_file() {
        Ok("File".to_string())
    } else {
        Err(format!("Path does not exist: {}", path))
    }
}

#[tauri::command]
#[specta::specta]
pub async fn open_path(path: String) -> Result<(), String> {
    open::that(&path).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn get_disk_info(path: String) -> Result<DiskInfo, String> {
    use sysinfo::Disks;

    let disks = Disks::new_with_refreshed_list();
    let path_buf = std::path::PathBuf::from(&path);

    let mut best: Option<(std::path::PathBuf, u64, u64)> = None;
    for disk in &disks {
        let mount = disk.mount_point();
        if path_buf.starts_with(mount) {
            let mount_depth = mount.components().count();
            let is_better = best
                .as_ref()
                .map(|(m, _, _)| m.components().count() < mount_depth)
                .unwrap_or(true);
            if is_better {
                best = Some((
                    mount.to_path_buf(),
                    disk.total_space(),
                    disk.available_space(),
                ));
            }
        }
    }

    match best {
        Some((_, total_bytes, available_bytes)) => Ok(DiskInfo {
            total_bytes,
            available_bytes,
            path,
        }),
        None => Err(format!("Disk bilgisi alınamadı: {}", path)),
    }
}
