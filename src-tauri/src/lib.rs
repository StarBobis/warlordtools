use std::path::Path;
use std::fs;
use tauri::Manager;
use tauri::Emitter;
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

#[tauri::command]
fn get_clipboard_text() -> Result<String, String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.get_text().map_err(|e| e.to_string())
}

#[derive(serde::Serialize, serde::Deserialize)]
struct TradeSearchResult {
    url: String,
    total: Option<u64>,
}

#[tauri::command]
fn search_trade(item_name: String, item_type: String, league: String, server: String, session_cookie: String) -> Result<TradeSearchResult, String> {
    let (base_url, frontend_base, referer) = match server.as_str() {
        "intl" => (
            format!("https://www.pathofexile.com/api/trade2/search/poe2/{}", league),
            format!("https://www.pathofexile.com/trade2/search/poe2/{}", league),
            "https://www.pathofexile.com/",
        ),
        "cn" => (
            format!("https://poe.game.qq.com/api/trade2/search/poe2/{}", league),
            format!("https://poe.game.qq.com/trade2/search/poe2/{}", league),
            "https://poe.game.qq.com/",
        ),
        _ => return Err("Invalid server".to_string()),
    };

    let mut query = serde_json::json!({
        "query": {
            "status": { "option": "online" },
            "type": item_type
        },
        "sort": { "price": "asc" }
    });

    if !item_name.is_empty() {
        query["query"]["name"] = serde_json::json!(item_name);
    }

    let mut req = ureq::post(&base_url)
        .set("Content-Type", "application/json")
        .set("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36")
        .set("Accept", "application/json")
        .set("Referer", &referer)
        .set("Origin", &referer.trim_end_matches('/'));

    if !session_cookie.is_empty() {
        req = req.set("Cookie", &session_cookie);
    }

    let resp = req
        .send_json(&query)
        .map_err(|e| {
            let err_str = e.to_string();
            if err_str.contains("status 401") || err_str.contains("Unauthorized") {
                "需要登录市集。请在浏览器中登录 poe.game.qq.com 后，F12 → Network → 复制完整 Cookie 字符串，粘贴到右侧「Session Cookie」输入框".to_string()
            } else if err_str.contains("status 429") {
                "请求太频繁，请稍等几秒再试".to_string()
            } else if err_str.contains("status 403") {
                "访问被拒绝，可能需要完成人机验证。请在浏览器中打开市集完成验证后再试".to_string()
            } else {
                format!("市集搜索失败: {}", err_str)
            }
        })?;

    let body: serde_json::Value = resp.into_json().map_err(|e| e.to_string())?;

    let search_id = body["id"].as_str().unwrap_or("");
    let total = body["total"].as_u64();

    Ok(TradeSearchResult {
        url: format!("{}/{}", frontend_base, search_id),
        total,
    })
}

fn simulate_copy() -> Result<(), String> {
    use enigo::{Enigo, Key, Keyboard, Settings, Direction::{Press, Release, Click}};
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
    // Release any stuck modifiers first
    let _ = enigo.key(Key::Control, Release);
    let _ = enigo.key(Key::Shift, Release);
    let _ = enigo.key(Key::Alt, Release);
    std::thread::sleep(std::time::Duration::from_millis(20));
    // Send Ctrl+C
    let _ = enigo.key(Key::Control, Press);
    let _ = enigo.key(Key::Unicode('c'), Click);
    let _ = enigo.key(Key::Control, Release);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            #[cfg(desktop)]
            {
                use tauri_plugin_global_shortcut::{Code, Modifiers, ShortcutState};

                let handle = app.handle().clone();

                app.handle().plugin(
                    tauri_plugin_global_shortcut::Builder::new()
                        .with_handler(move |_app, shortcut, event| {
                            if event.state != ShortcutState::Pressed {
                                return;
                            }
                            // Ctrl+D = quick price check (auto-hide on mouse move)
                            let is_ctrl_d = shortcut.matches(Modifiers::CONTROL, Code::KeyD);
                            // Ctrl+Alt+D = sticky price check (stays open)
                            let is_ctrl_alt_d = shortcut.matches(Modifiers::CONTROL | Modifiers::ALT, Code::KeyD);

                            if is_ctrl_d || is_ctrl_alt_d {
                                // Simulate Ctrl+C to copy item under cursor in-game
                                if let Err(e) = simulate_copy() {
                                    eprintln!("Failed to simulate Ctrl+C: {}", e);
                                    return;
                                }
                                // Wait for the game to process and put item text on clipboard
                                std::thread::sleep(std::time::Duration::from_millis(80));

                                // Read clipboard
                                match arboard::Clipboard::new() {
                                    Ok(mut clipboard) => {
                                        match clipboard.get_text() {
                                            Ok(text) => {
                                                // Emit to frontend
                                                let sticky = is_ctrl_alt_d;
                                                let _ = handle.emit("price-check-triggered", serde_json::json!({
                                                    "text": text,
                                                    "sticky": sticky
                                                }));
                                            }
                                            Err(e) => eprintln!("Clipboard read error: {}", e),
                                        }
                                    }
                                    Err(e) => eprintln!("Clipboard init error: {}", e),
                                }
                            }
                        })
                        .with_shortcut("Ctrl+D")?
                        .with_shortcut("Ctrl+Alt+D")?
                        .build(),
                )?;
            }
            Ok(())
        })
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
            rename_filter_file,
            get_clipboard_text,
            search_trade
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
