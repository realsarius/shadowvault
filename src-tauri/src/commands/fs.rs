use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
pub async fn pick_directory(app: AppHandle) -> Result<Option<String>, String> {
    let path = app
        .dialog()
        .file()
        .blocking_pick_folder();

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
pub async fn pick_file(app: AppHandle) -> Result<Option<String>, String> {
    let path = app
        .dialog()
        .file()
        .blocking_pick_file();

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
