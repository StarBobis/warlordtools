use std::path::Path;
use std::fs;
use tauri::Manager;
pub mod powershell_opener;
pub use powershell_opener::{open_file, open_folder, copy_file_powershell};

#[tauri::command]
fn open_folder_cmd(path: String) -> Result<(), String> {
    open_folder(&path)
}

#[tauri::command]
fn open_file_cmd(path: String) -> Result<(), String> {
    open_file(&path)
}

#[tauri::command]
fn copy_sound_file(src: String, dest: String) -> Result<(), String> {
    copy_file_powershell(&src, &dest)
}


// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn scan_filter_files(path: String) -> Result<Vec<String>, String> {
    let mut filters = Vec::new();
    let root = Path::new(&path);

    if !root.exists() {
        return Err("Path does not exist".to_string());
    }

    // Recursively scan
    fn visit_dirs(dir: &Path, filters: &mut Vec<String>) -> std::io::Result<()> {
        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    visit_dirs(&path, filters)?;
                } else {
                    if let Some(ext) = path.extension() {
                        if ext == "filter" {
                            if let Some(path_str) = path.to_str() {
                                filters.push(path_str.to_string());
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    match visit_dirs(root, &mut filters) {
        Ok(_) => Ok(filters),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
fn read_file_content(path: String) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| e.to_string())
}

#[tauri::command]
fn write_file_content(path: String, content: String) -> Result<(), String> {
    fs::write(path, content).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_filter_file(path: String) -> Result<(), String> {
    fs::remove_file(path).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_filter_folder(path: String) -> Result<(), String> {
    fs::remove_dir_all(path).map_err(|e| e.to_string())
}

#[tauri::command]
fn create_filter_folder(path: String) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|e| e.to_string())
}

#[tauri::command]
fn path_exists(path: String) -> Result<bool, String> {
    Ok(Path::new(&path).exists())
}

#[tauri::command]
fn rename_filter_file(old_path: String, new_path: String) -> Result<(), String> {
    let new_path_ref = Path::new(&new_path);

    if new_path_ref.exists() {
        return Err("目标文件已存在".to_string());
    }

    fs::rename(&old_path, new_path_ref).map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_overlay_window(app: tauri::AppHandle, label: String, target_url: String) -> Result<(), String> {
    if app.get_webview_window(&label).is_some() {
        return Ok(());
    }

    let script = r#"
      console.log("Blocking NitroAds");
      try {
          window.NitroAds = new Proxy({}, {
            get: () => () => ({ then: (cb) => cb?.() }),
            set: () => true
          });
          Object.freeze(window.NitroAds);
      } catch(e) {}
    "#;

    tauri::WebviewWindowBuilder::new(&app, &label, tauri::WebviewUrl::External(target_url.parse().map_err(|e: url::ParseError| e.to_string())?))
        .title("Overlay")
        .decorations(false)
        .transparent(false)
        .skip_taskbar(true)
        .visible(false)
        .inner_size(800.0, 600.0)
        .initialization_script(script)
        .build()
        .map_err(|e| e.to_string())?;
    
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            scan_filter_files,
            read_file_content,
            write_file_content,
            open_folder_cmd,
            open_file_cmd,
            create_overlay_window,
            copy_sound_file,
            delete_filter_file,
            delete_filter_folder,
            create_filter_folder,
            path_exists,
            rename_filter_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
