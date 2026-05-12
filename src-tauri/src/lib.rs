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
fn fetch_leagues(server: String, session_cookie: String) -> Result<Vec<String>, String> {
    let api_base = match server.as_str() {
        "intl" => "https://www.pathofexile.com",
        "cn" => "https://poe.game.qq.com",
        _ => return Err("Invalid server".to_string()),
    };

    let url = match server.as_str() {
        "cn" => format!("{}/api/trade2/data/leagues?realm=poe2", api_base),
        _ => format!("{}/api/trade2/data/leagues", api_base),
    };

    let mut req = ureq::get(&url)
        .set("Accept", "application/json")
        .set("Content-Type", "application/json");

    if !session_cookie.is_empty() {
        req = req.set("Cookie", &session_cookie);
    }

    let resp = req.call().map_err(|e| format!("获取联赛列表失败: {}", e))?;
    let body: serde_json::Value = resp.into_json().map_err(|e| e.to_string())?;

    let leagues: Vec<String> = body["result"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|l| l["id"].as_str().map(String::from))
        .collect();

    Ok(leagues)
}

#[tauri::command]
async fn search_trade(item_name: String, item_type: String, mod_text: String, league: String, server: String, session_cookie: String) -> Result<TradeSearchResult, String> {
    // Match EE2 exactly:
    // POST https://{host}/api/trade2/search/{league}
    // Headers: only Accept + Content-Type
    // Cookies: session cookie (from user's browser)
    let (api_base, frontend_base) = match server.as_str() {
        "intl" => (
            "https://www.pathofexile.com",
            "https://www.pathofexile.com/trade2/search/poe2",
        ),
        "cn" => (
            "https://poe.game.qq.com",
            "https://poe.game.qq.com/trade2/search/poe2",
        ),
        _ => return Err("Invalid server".to_string()),
    };

    let api_url = match server.as_str() {
        "cn" => format!("{}/api/trade2/search/poe2/{}", api_base, league),
        _ => format!("{}/api/trade2/search/{}", api_base, league),
    };

    let status_opt = if server == "cn" { "securable" } else { "online" };
    let mut query = serde_json::json!({
        "query": {
            "status": { "option": status_opt },
            "stats": [{ "type": "and", "filters": [], "disabled": false }]
        },
        "sort": { "price": "asc" }
    });
    if !item_name.is_empty() {
        query["query"]["name"] = serde_json::json!(item_name);
    }
    if !item_type.is_empty() {
        query["query"]["type"] = serde_json::json!(item_type);
    }

    // Add mod filters using stat ID database
    if !mod_text.is_empty() {
        if let Some(ref stat_db) = *STAT_DB.lock().unwrap() {
            let db_size: usize = stat_db.values().map(|v| v.len()).sum();
            eprintln!("[WarlordTools] stat DB size={}, mod_text len={}", db_size, mod_text.len());
            let mod_filters = match_mods_to_stats(&mod_text, stat_db, server.as_str());
            eprintln!("[WarlordTools] matched {} mod filters from {} mod lines", mod_filters.len(), mod_text.lines().count());
            if !mod_filters.is_empty() {
                if let Some(stats) = query["query"]["stats"].as_array_mut() {
                    stats.push(serde_json::json!({
                        "type": "and",
                        "filters": mod_filters,
                        "disabled": false
                    }));
                }
            }
        } else {
            eprintln!("[WarlordTools] stat DB not loaded, skipping mod filters");
        }
    }

    // Only send the two headers that EE2 sends (no Referer/Origin/User-Agent hack)
    let mut req = ureq::post(&api_url)
        .set("Accept", "application/json")
        .set("Content-Type", "application/json");

    if !session_cookie.is_empty() {
        let clean_cookie: String = session_cookie
            .chars()
            .filter(|c| !c.is_control() || *c == '\t')
            .collect();
        if !clean_cookie.is_empty() {
            req = req.set("Cookie", &clean_cookie);
        }
    }

    let resp = req
        .send_json(&query)
        .map_err(|e| {
            let err_str = e.to_string();
            if err_str.contains("status 401") || err_str.contains("Unauthorized") {
                format!(
                    "401 未登录。请按以下步骤获取 Cookie：\n\
                    1. 浏览器打开 {} 并登录\n\
                    2. F12 → Application → Cookies\n\
                    3. 复制所有 Cookie，粘贴到右侧「Session Cookie」输入框",
                    api_base
                )
            } else if err_str.contains("status 400") {
                format!("400 请求错误。联赛名可能不正确: 「{}」。请检查赛季选择", league)
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
        url: format!("{}/{}/{}", frontend_base, league, search_id),
        total,
    })
}

#[tauri::command]
async fn open_login_window(app: tauri::AppHandle, server: String) -> Result<(), String> {
    let label = format!("login-{}", server);
    let url = match server.as_str() {
        "cn" => "https://poe.game.qq.com/trade2/",
        "intl" => "https://www.pathofexile.com/trade2/",
        _ => return Err("Invalid server".to_string()),
    };

    // Close existing login window if any
    if let Some(existing) = app.get_webview_window(&label) {
        let _ = existing.destroy();
    }

    tauri::WebviewWindowBuilder::new(&app, &label, tauri::WebviewUrl::External(url.parse().map_err(|e: url::ParseError| e.to_string())?))
        .title(match server.as_str() {
            "cn" => "登录国服市集",
            _ => "Login to PoE Trade",
        })
        .inner_size(1024.0, 768.0)
        .center()
        .build()
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
async fn check_login_status(app: tauri::AppHandle, server: String) -> Result<bool, String> {
    let label = format!("login-{}", server);
    let window = app.get_webview_window(&label)
        .ok_or("登录窗口未打开")?;
    let current_url = window.url().map_err(|e| e.to_string())?;
    // Logged in = user is on the trade site (not a login/oauth page)
    let url_str = current_url.to_string();
    let is_logged_in = url_str.contains("trade2") && !url_str.contains("login");
    Ok(is_logged_in)
}

#[tauri::command]
async fn close_login_window(app: tauri::AppHandle, server: String) -> Result<(), String> {
    let label = format!("login-{}", server);
    if let Some(window) = app.get_webview_window(&label) {
        window.destroy().map_err(|e| e.to_string())?;
    }
    Ok(())
}

// Open trade site and paste item text directly (no API call, trade site does the search)
#[tauri::command]
async fn open_trade_with_item(
    app: tauri::AppHandle,
    server: String,
    league: String,
    item_text: String,
) -> Result<String, String> {
    let label = format!("trade-paste-{}", server);
    // Close existing if any
    if let Some(w) = app.get_webview_window(&label) { let _ = w.destroy(); }

    let base_url = match server.as_str() {
        "cn" => format!("https://poe.game.qq.com/trade2/search/poe2/{}", league),
        "intl" => format!("https://www.pathofexile.com/trade2/search/poe2/{}", league),
        _ => return Err("Invalid server".to_string()),
    };

    // Escape item text for JS
    let escaped = item_text.replace('\\', "\\\\").replace('\'', "\\'").replace('\n', "\\n").replace('\r', "");

    let init_script = format!(r#"
        (function() {{
            var itemText = '{escaped}';
            console.log('[WarlordTools] init_script started, item text length:', itemText.length);
            var pollCount = 0;
            var iv = setInterval(function() {{
                pollCount++;
                // Find the largest textarea on the page (paste area)
                var textareas = document.querySelectorAll('textarea');
                var inputs = document.querySelectorAll('input[type="text"], input:not([type])');
                var allFields = [].concat(Array.from(textareas), Array.from(inputs));
                // Filter visible fields
                var visible = allFields.filter(function(el) {{
                    return el.offsetParent !== null && !el.disabled;
                }});
                console.log('[WarlordTools] Poll #' + pollCount + ': ' + visible.length + ' visible fields, ' + textareas.length + ' textareas');

                if (visible.length > 0 && pollCount >= 3) {{
                    // Pick the largest textarea or the first visible input
                    var target = textareas.length > 0 ? textareas[0] : visible[0];
                    console.log('[WarlordTools] Filling field:', target.tagName, target.placeholder || target.className);
                    // Use React value setter pattern
                    var nativeInputValueSetter = Object.getOwnPropertyDescriptor(window.HTMLTextAreaElement.prototype, 'value').set;
                    nativeInputValueSetter.call(target, itemText);
                    target.dispatchEvent(new Event('input', {{ bubbles: true }}));
                    target.dispatchEvent(new Event('change', {{ bubbles: true }}));
                    console.log('[WarlordTools] Item text set on field');
                    clearInterval(iv);

                    // Look for search/submit button after a short delay
                    setTimeout(function() {{
                        var buttons = document.querySelectorAll('button');
                        for (var i = 0; i < buttons.length; i++) {{
                            var b = buttons[i];
                            var text = (b.textContent || '').toLowerCase();
                            if (text.includes('search') || text.includes('搜索') || text.includes('submit') || text.includes('提交')) {{
                                console.log('[WarlordTools] Clicking button:', text);
                                b.click();
                                return;
                            }}
                        }}
                        console.log('[WarlordTools] No search button found, user may need to click manually');
                    }}, 800);
                }}
                if (pollCount > 40) {{
                    console.log('[WarlordTools] Timeout after 20s');
                    clearInterval(iv);
                }}
            }}, 500);
        }})();
    "#, escaped = escaped);

    tauri::WebviewWindowBuilder::new(&app, &label, tauri::WebviewUrl::External(base_url.parse().map_err(|e: url::ParseError| e.to_string())?))
        .title("查价结果")
        .inner_size(1200.0, 800.0)
        .center()
        .initialization_script(&init_script)
        .build()
        .map_err(|e| e.to_string())?;

    Ok(base_url)
}

// Extract numeric values from PoE item mod text for stat filters
// e.g., "+149 最大魔力" -> ("最大魔力", 149), "法术暴击率提高 61%" -> ("法术暴击率", 61)
fn parse_mod_filters(mod_text: &str) -> Vec<(String, f64)> {
    let mut filters = Vec::new();
    for line in mod_text.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }
        // Skip non-mod lines
        if line.starts_with("物品类别") || line.starts_with("稀") || line.starts_with("Item Class") || line.starts_with("Rarity")
            || line.starts_with("--------") || line.starts_with("物品等级") || line.starts_with("Item Level")
            || line.starts_with("需求") || line.starts_with("插槽") || line.starts_with("Sockets")
            || line.starts_with("品质") || line.starts_with("Quality") || line.starts_with("Note") || line.starts_with("备注")
        {
            continue;
        }
        // Remove PoE2 suffixes: (rune), (implicit), (crafted), (desecrated), (augmented)
        let cleaned = line.replace("(rune)", "").replace("(implicit)", "").replace("(crafted)", "")
            .replace("(desecrated)", "").replace("(augmented)", "").replace("(enchant)", "")
            .trim().to_string();
        if cleaned.is_empty() { continue; }

        // Extract the "stat name" part: remove numbers and common prefixes
        let stat_name = cleaned
            .replace(|c: char| c.is_numeric() || c == '+' || c == '-' || c == '%' || c == '.' || c == '（' || c == '）' || c == '(' || c == ')', "")
            .replace("获得相当于伤害  的额外", "").replace("获得相当于伤害  的", "")
            .replace("  的额外", "").replace("  的", "").replace("  ", " ").trim().to_string();

        // Extract ALL numbers from the line
        let nums: Vec<f64> = cleaned
            .split(|c: char| !c.is_numeric() && c != '.' && c != '-' && c != '+')
            .filter_map(|s| {
                let s = s.trim().trim_start_matches('+');
                s.parse::<f64>().ok()
            })
            .filter(|&n| n > 0.0)
            .collect();

        if let Some(&val) = nums.first() {
            let name = if stat_name.len() > 2 { stat_name } else { cleaned.clone() };
            // Skip very short names and flavor text
            if name.len() >= 3 && val >= 1.0 && val <= 10000.0 {
                filters.push((name, val));
            }
        }
    }
    filters
}

// Webview-based leagues fetch (for CN where direct API returns 401)
#[tauri::command]
async fn fetch_leagues_webview(app: tauri::AppHandle, server: String) -> Result<Vec<String>, String> {
    let label = format!("login-{}", server);
    if app.get_webview_window(&label).is_none() {
        return Err("请先登录市集".to_string());
    }
    let window = app.get_webview_window(&label).unwrap();

    let api_url = match server.as_str() {
        "cn" => "https://poe.game.qq.com/api/trade2/data/leagues?realm=poe2",
        "intl" => "https://www.pathofexile.com/api/trade2/data/leagues",
        _ => return Err("Invalid server".to_string()),
    };

    let js = format!(r#"
        (async function() {{
            try {{
                var resp = await fetch('{api_url}');
                var data = await resp.json();
                var ids = (data.result || []).map(function(l) {{ return l.id; }});
                var payload = btoa(unescape(encodeURIComponent(JSON.stringify(ids))));
                history.replaceState(null, '', location.pathname + '?__wr_ok__=' + encodeURIComponent(payload));
            }} catch(e) {{
                var msg = btoa(String(e.message || e));
                history.replaceState(null, '', location.pathname + '?__wr_err__=' + encodeURIComponent(msg));
            }}
        }})();
    "#, api_url = api_url);

    window.eval(&js).map_err(|e| e.to_string())?;

    let start = std::time::Instant::now();
    loop {
        if start.elapsed() > std::time::Duration::from_secs(10) {
            return Err("加载联赛超时".to_string());
        }
        std::thread::sleep(std::time::Duration::from_millis(300));
        let current_url = match window.url() { Ok(u) => u.to_string(), Err(_) => continue };

        if let Some(pos) = current_url.find("__wr_ok__=") {
            let encoded = &current_url[pos + 10..];
            let encoded = encoded.split('&').next().unwrap_or("").split('#').next().unwrap_or("").split('\0').next().unwrap_or("");
            let b64 = url_decode(encoded);
            let raw = base64_decode(&b64).unwrap_or_default();
            let leagues: Vec<String> = serde_json::from_str(&raw).map_err(|e| format!("解析失败: {}", e))?;
            let _ = window.eval("history.back()");
            return Ok(leagues);
        }
        if current_url.contains("__wr_err__=") { break; }
    }
    Err("加载联赛失败".to_string())
}

// Webview-based search: JS fetch() from webview sends ALL cookies (including HttpOnly)
// Uses URL hash as data channel (no HTTP bridge → no mixed content issues)
#[tauri::command]
async fn search_trade_webview(
    app: tauri::AppHandle,
    server: String,
    league: String,
    item_name: String,
    item_type: String,
    mod_text: String,
) -> Result<TradeSearchResult, String> {
    let label = format!("login-{}", server);
    // Re-open login window if closed (WebView2 cookies persist across sessions)
    if app.get_webview_window(&label).is_none() {
        let url = match server.as_str() {
            "cn" => "https://poe.game.qq.com/trade2/",
            "intl" => "https://www.pathofexile.com/trade2/",
            _ => return Err("Invalid server".to_string()),
        };
        tauri::WebviewWindowBuilder::new(&app, &label, tauri::WebviewUrl::External(url.parse().map_err(|e: url::ParseError| e.to_string())?))
            .title("查价中...")
            .inner_size(1024.0, 768.0)
            .center()
            .visible(false)  // hidden — cookies still work
            .build()
            .map_err(|e| e.to_string())?;
        // Wait for page to load (cookies sent, DOM ready for fetch)
        std::thread::sleep(std::time::Duration::from_millis(1500));
    }
    let window = app.get_webview_window(&label)
        .ok_or("登录窗口创建失败")?;

    let api_url = match server.as_str() {
        "cn" => format!("https://poe.game.qq.com/api/trade2/search/poe2/{}", league),
        "intl" => format!("https://www.pathofexile.com/api/trade2/search/{}", league),
        _ => return Err("Invalid server".to_string()),
    };

    let frontend_base = match server.as_str() {
        "cn" => format!("https://poe.game.qq.com/trade2/search/poe2/{}", league),
        "intl" => format!("https://www.pathofexile.com/trade2/search/poe2/{}", league),
        _ => return Err("Invalid server".to_string()),
    };

    let status_opt = if server == "cn" { "securable" } else { "online" };
    let mut query = serde_json::json!({
        "query": {
            "status": { "option": status_opt },
            "stats": [{ "type": "and", "filters": [], "disabled": false }]
        },
        "sort": { "price": "asc" }
    });
    if !item_name.is_empty() {
        query["query"]["name"] = serde_json::json!(item_name);
    }
    if !item_type.is_empty() {
        query["query"]["type"] = serde_json::json!(item_type);
    }

    // Add mod filters using stat ID database
    if !mod_text.is_empty() {
        if let Some(ref stat_db) = *STAT_DB.lock().unwrap() {
            let db_size: usize = stat_db.values().map(|v| v.len()).sum();
            eprintln!("[WarlordTools] stat DB size={}, mod_text len={}", db_size, mod_text.len());
            let mod_filters = match_mods_to_stats(&mod_text, stat_db, server.as_str());
            eprintln!("[WarlordTools] matched {} mod filters from {} mod lines", mod_filters.len(), mod_text.lines().count());
            if !mod_filters.is_empty() {
                if let Some(stats) = query["query"]["stats"].as_array_mut() {
                    stats.push(serde_json::json!({
                        "type": "and",
                        "filters": mod_filters,
                        "disabled": false
                    }));
                }
            }
        } else {
            eprintln!("[WarlordTools] stat DB not loaded, skipping mod filters");
        }
    }

    let query_json = serde_json::to_string(&query).map_err(|e| e.to_string())?;
    let query_escaped = query_json.replace('\\', "\\\\").replace('\'', "\\'");
    let api_url_escaped = api_url.replace('\'', "\\'");

    eprintln!("[WarlordTools] ====== 查价请求 ======");
    eprintln!("[WarlordTools] POST {}", api_url);
    eprintln!("[WarlordTools] Body: {}", query_json);
    eprintln!("[WarlordTools] =====================");

    // JS: fetch (same-origin = all cookies), store result as a query param
    let js = format!(r#"
        console.group('%c[WarlordTools 查价请求]', 'color:#409eff;font-weight:bold');
        console.log('%cPOST %c{api_url}', 'color:#67c23a', 'color:#e6a23c');
        console.log('%cHeaders:%c Accept: application/json%c Content-Type: application/json', 'color:#909399', 'color:#ccc', '');
        var queryObj = JSON.parse('{query_json}');
        console.log('Body:', JSON.stringify(queryObj, null, 2));
        console.groupEnd();
        (async function() {{
            try {{
                console.group('%c[WarlordTools 响应]', 'color:#409eff;font-weight:bold');
                var resp = await fetch('{api_url}', {{
                    method: 'POST',
                    headers: {{ 'Accept': 'application/json', 'Content-Type': 'application/json' }},
                    body: '{query_json}'
                }});
                console.log('Status:', resp.status, resp.statusText);
                console.log('Response Headers:', JSON.stringify(Object.fromEntries([...resp.headers])));
                var data = await resp.json();
                console.log('Body:', JSON.stringify(data, null, 2));
                console.groupEnd();
                if (!data.id) {{
                    var why = data.error;
                    if (typeof why === 'object' && why !== null) {{
                        why = why.message || JSON.stringify(why);
                    }}
                    throw new Error(why || '搜索返回异常 (无id)');
                }}
                var payload = btoa(JSON.stringify({{id: data.id, total: data.total}}));
                var newUrl = location.pathname + '?__wr_ok__=' + encodeURIComponent(payload);
                history.replaceState(null, '', newUrl);
            }} catch(e) {{
                // Robust error → string conversion
                var errStr = '';
                if (typeof e === 'string') {{
                    errStr = e;
                }} else if (e && typeof e.message === 'string') {{
                    errStr = e.message;
                }} else if (e && typeof e.statusText === 'string') {{
                    errStr = 'HTTP ' + (e.status || '?') + ' ' + e.statusText;
                }} else if (e && e.error) {{
                    errStr = (typeof e.error === 'string') ? e.error : JSON.stringify(e.error);
                }} else {{
                    try {{ errStr = JSON.stringify(e); }} catch(_) {{ errStr = String(e); }}
                }}
                console.group('%c[WarlordTools 错误]', 'color:#f56c6c;font-weight:bold');
                console.error('Message:', errStr);
                console.error('Full Error:', e);
                console.groupEnd();
                var msg = btoa(unescape(encodeURIComponent(errStr)));
                history.replaceState(null, '', location.pathname + '?__wr_err__=' + encodeURIComponent(msg));
            }}
        }})();
    "#, api_url = api_url_escaped, query_json = query_escaped);

    window.eval(&js).map_err(|e| e.to_string())?;

    // Poll window.url() for the result (max 15s)
    let start = std::time::Instant::now();
    loop {
        if start.elapsed() > std::time::Duration::from_secs(15) {
            return Err("搜索超时，请检查网络或重试".to_string());
        }
        std::thread::sleep(std::time::Duration::from_millis(300));
        let current_url = match window.url() {
            Ok(u) => u.to_string(),
            Err(_) => continue,
        };
        eprintln!("[WarlordTools] Polling URL: {}", current_url);

        if let Some(pos) = current_url.find("__wr_ok__=") {
            let encoded = &current_url[pos + 10..];  // "__wr_ok__=" 是 10 个字符
            let encoded = encoded.split('&').next().unwrap_or("").split('#').next().unwrap_or("").split('\0').next().unwrap_or("");
            let b64 = url_decode(encoded);
            eprintln!("[WarlordTools] Decoded base64: '{}'", b64);
            let raw = base64_decode(&b64).unwrap_or_default();
            eprintln!("[WarlordTools] Parsed JSON: '{}'", raw);
            let result_json: serde_json::Value = serde_json::from_str(&raw)
                .map_err(|e| format!("解析失败: {}\n完整URL: {}\nBase64: {}\nJSON: {}", e, current_url, b64, raw))?;
            let search_id = result_json["id"].as_str().unwrap_or("");
            let total = result_json["total"].as_u64();
            let _ = window.eval("history.back()");
            return Ok(TradeSearchResult {
                url: format!("{}/{}", frontend_base, search_id),
                total,
            });
        }
        if let Some(pos) = current_url.find("__wr_err__=") {
            let encoded = &current_url[pos + 11..];  // "__wr_err__=" 是 11 个字符
            let encoded = encoded.split('&').next().unwrap_or("").split('#').next().unwrap_or("").split('\0').next().unwrap_or("");
            let b64 = url_decode(encoded);
            let raw = base64_decode(&b64).unwrap_or_else(|_| format!("(base64解码失败) b64='{}' encoded='{}'", b64, encoded));
            eprintln!("[WarlordTools] ERROR raw: '{}'", raw);
            let _ = window.eval("history.back()");
            return Err(format!("搜索失败: {}", raw));
        }
    }
}

fn base64_decode(s: &str) -> Result<String, String> {
    // Simple base64 decode using only std
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let s = s.trim();
    let mut bytes = Vec::with_capacity(s.len() * 3 / 4);
    let mut buf = 0u32;
    let mut bits = 0;
    for c in s.chars() {
        if c == '=' { break; }
        let idx = CHARS.iter().position(|&x| x == c as u8).ok_or("invalid base64")?;
        buf = (buf << 6) | idx as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            bytes.push((buf >> bits) as u8);
        }
    }
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

fn url_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                    continue;
                }
            }
            result.push('%');
            result.push_str(&hex);
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    result
}

#[cfg(target_os = "windows")]
static mut POE_HWND: isize = 0;

#[cfg(target_os = "windows")]
unsafe extern "system" fn poe_enum_callback(hwnd: isize, _lparam: isize) -> i32 {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    extern "system" {
        fn IsWindowVisible(hWnd: isize) -> i32;
        fn GetWindowTextW(hWnd: isize, lpString: *mut u16, nMaxCount: i32) -> i32;
    }

    if IsWindowVisible(hwnd) == 0 {
        return 1;
    }
    let mut buf = [0u16; 256];
    let len = GetWindowTextW(hwnd, buf.as_mut_ptr(), 255);
    if len > 0 {
        let title = OsString::from_wide(&buf[..len as usize]).to_string_lossy().to_string();
        if title.contains("Path of Exile") || title.contains("流放之路") {
            POE_HWND = hwnd;
            return 0;
        }
    }
    1
}

fn simulate_copy() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        extern "system" {
            fn EnumWindows(callback: unsafe extern "system" fn(isize, isize) -> i32, lParam: isize) -> i32;
            fn SetForegroundWindow(hWnd: isize) -> i32;
            fn ShowWindow(hWnd: isize, nCmdShow: i32) -> i32;
            fn keybd_event(bVk: u8, bScan: u8, dwFlags: u32, dwExtraInfo: usize);
        }

        const VK_CONTROL: u8 = 0x11;
        const VK_MENU: u8 = 0x12;       // Alt key
        const KEYEVENTF_KEYUP: u32 = 0x0002;
        const SW_RESTORE: i32 = 9;

        unsafe {
            POE_HWND = 0;
            EnumWindows(poe_enum_callback, 0);

            if POE_HWND != 0 {
                ShowWindow(POE_HWND, SW_RESTORE);
                SetForegroundWindow(POE_HWND);
                std::thread::sleep(std::time::Duration::from_millis(50));

                // EE2 sends Ctrl+Alt+C (Alt=show advanced mods in PoE2)
                // Hold modifiers, tap C, release — matching EE2's pressKeysToCopyItemText
                keybd_event(VK_CONTROL, 0, 0, 0);
                keybd_event(VK_MENU, 0, 0, 0);
                std::thread::sleep(std::time::Duration::from_millis(10));
                // Tap C
                keybd_event('C' as u8, 0, 0, 0);
                std::thread::sleep(std::time::Duration::from_millis(5));
                keybd_event('C' as u8, 0, KEYEVENTF_KEYUP, 0);
                // Release modifiers (reverse order)
                keybd_event(VK_MENU, 0, KEYEVENTF_KEYUP, 0);
                keybd_event(VK_CONTROL, 0, KEYEVENTF_KEYUP, 0);
                eprintln!("[WarlordTools] keybd_event Ctrl+Alt+C sent to game");
            } else {
                eprintln!("[WarlordTools] Game window not found");
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        use enigo::{Enigo, Key, Keyboard, Settings, Direction::{Press, Release, Click}};
        let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
        let _ = enigo.key(Key::Control, Press);
        let _ = enigo.key(Key::Alt, Press);
        let _ = enigo.key(Key::Unicode('c'), Click);
        let _ = enigo.key(Key::Alt, Release);
        let _ = enigo.key(Key::Control, Release);
    }

    Ok(())
}

// ---- Stat ID Database ----
use std::sync::Mutex;
use std::collections::HashMap;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct StatEntry {
    id: String,
    text: String,
}

static STAT_DB: Mutex<Option<HashMap<String, Vec<StatEntry>>>> = Mutex::new(None);

#[tauri::command]
async fn get_stat_db(app: tauri::AppHandle, server: String) -> Result<usize, String> {
    // Try to load from disk first
    let cache_path = get_stat_cache_path(&server);
    if let Ok(cached) = std::fs::read_to_string(&cache_path) {
        if let Ok(map) = serde_json::from_str::<HashMap<String, Vec<StatEntry>>>(&cached) {
            let count: usize = map.values().map(|v| v.len()).sum();
            *STAT_DB.lock().unwrap() = Some(map);
            eprintln!("[WarlordTools] stat DB loaded from cache: {} entries", count);
            return Ok(count);
        }
    }
    // Fetch from API via webview
    let count = fetch_stat_data_webview(app, server).await?;
    // Save to disk cache
    if let Some(ref db) = *STAT_DB.lock().unwrap() {
        if let Ok(json) = serde_json::to_string(db) {
            let _ = std::fs::create_dir_all(std::path::Path::new(&cache_path).parent().unwrap());
            let _ = std::fs::write(&cache_path, json);
        }
    }
    Ok(count)
}

fn get_stat_cache_path(server: &str) -> String {
    let dir = dirs_next().unwrap_or_else(|| std::path::PathBuf::from("."));
    format!("{}/WarlordToolsConfig/stats_{}.json", dir.display(), server)
}

fn dirs_next() -> Option<std::path::PathBuf> {
    // Get LocalAppData equivalent
    std::env::var("LOCALAPPDATA").ok().map(std::path::PathBuf::from)
}

#[tauri::command]
async fn fetch_stat_data_webview(app: tauri::AppHandle, server: String) -> Result<usize, String> {
    let api_url = match server.as_str() {
        "cn" => "https://poe.game.qq.com/api/trade/data/stats",
        "intl" => "https://www.pathofexile.com/api/trade/data/stats",
        _ => return Err("Invalid server".to_string()),
    };

    let label = format!("login-{}", server);
    // Re-use login window if open, otherwise create hidden
    if app.get_webview_window(&label).is_none() {
        let url = match server.as_str() {
            "cn" => "https://poe.game.qq.com/trade2/",
            _ => "https://www.pathofexile.com/trade2/",
        };
        tauri::WebviewWindowBuilder::new(&app, &label, tauri::WebviewUrl::External(url.parse().map_err(|e: url::ParseError| e.to_string())?))
            .title(".")
            .inner_size(1.0, 1.0)
            .visible(false)
            .build().map_err(|e| e.to_string())?;
        std::thread::sleep(std::time::Duration::from_millis(2000));
    }
    let window = app.get_webview_window(&label).ok_or("no window")?;
    eprintln!("[WarlordTools] fetch_stat_data: using window, fetching...");

    let js = format!(r#"
        (async function() {{
            try {{
                var resp = await fetch('{url}');
                var data = await resp.json();
                var payload = btoa(unescape(encodeURIComponent(JSON.stringify(data))));
                history.replaceState(null, '', location.pathname + '?__wr_ok__=' + encodeURIComponent(payload));
            }} catch(e) {{
                history.replaceState(null, '', location.pathname + '?__wr_err__=' + encodeURIComponent(String(e.message || e)));
            }}
        }})();
    "#, url = api_url);

    window.eval(&js).map_err(|e| e.to_string())?;

    let start = std::time::Instant::now();
    loop {
        if start.elapsed() > std::time::Duration::from_secs(15) { return Err("超时".to_string()); }
        std::thread::sleep(std::time::Duration::from_millis(300));
        let cur = window.url().map(|u| u.to_string()).unwrap_or_default();
        if let Some(pos) = cur.find("__wr_ok__=") {
            let enc = &cur[pos + 10..].split('&').next().unwrap_or("").split('#').next().unwrap_or("");
            let b64 = url_decode(enc);
            let json_str = base64_decode(&b64).unwrap_or_default();
            let root: serde_json::Value = serde_json::from_str(&json_str).map_err(|e| format!("JSON: {}", e))?;

            let mut map: HashMap<String, Vec<StatEntry>> = HashMap::new();
            if let Some(categories) = root["result"].as_array() {
                for cat in categories {
                    let label = cat["label"].as_str().unwrap_or("").to_string();
                    let entries: Vec<StatEntry> = cat["entries"].as_array().unwrap_or(&vec![])
                        .iter().filter_map(|e| {
                            let id = e["id"].as_str()?;
                            let text = e["text"].as_str()?;
                            if id.is_empty() || text.is_empty() { return None; }
                            Some(StatEntry { id: id.to_string(), text: text.to_string() })
                        }).collect();
                    if !entries.is_empty() {
                        map.entry(label).or_default().extend(entries);
                    }
                }
            }
            let count: usize = map.values().map(|v| v.len()).sum();
            *STAT_DB.lock().unwrap() = Some(map);
            let _ = window.eval("history.back()");
            return Ok(count);
        }
        if cur.contains("__wr_err__=") { break; }
    }
    Err("加载失败".to_string())
}

fn match_mods_to_stats(mod_text: &str, db: &HashMap<String, Vec<StatEntry>>, _server: &str) -> Vec<serde_json::Value> {
    let mut filters = Vec::new();
    let mut total_entries = 0usize;
    let mut matched_lines = 0usize;
    for line in mod_text.lines() {
        let line = line.trim();
        if line.is_empty() || line.len() < 3 { continue; }
        // Skip non-mod lines
        if line.starts_with("物品类别") || line.starts_with("稀") || line.starts_with("Item Class")
            || line.starts_with("--------") || line.starts_with("物品等级") || line.starts_with("Item Level")
            || line.starts_with("需求") || line.starts_with("插槽") || line.starts_with("Sockets")
            || line.starts_with("品质") || line.starts_with("Quality") || line.starts_with("获得技能")
            || line.starts_with("[品质]")
        { continue; }

        // Remove PoE2 suffixes AND bracket tags
        let cleaned = line
            .replace("(rune)", "").replace("(implicit)", "").replace("(crafted)", "")
            .replace("(desecrated)", "").replace("(augmented)", "").replace("(enchant)", "")
            .trim().to_string();
        // Also strip [Tag] format from line
        let cleaned = regex::Regex::new(r"\[.*?\]").unwrap().replace_all(&cleaned, "").to_string().trim().to_string();
        if cleaned.is_empty() { continue; }

        // Extract number(s) from the mod line
        let nums: Vec<f64> = cleaned
            .split(|c: char| !c.is_numeric() && c != '.' && c != '-' && c != '+')
            .filter_map(|s| { let s = s.trim().trim_start_matches('+'); s.parse::<f64>().ok() })
            .filter(|&n| n > 0.0 && n < 100000.0)
            .collect();
        if nums.is_empty() { continue; }

        // Try to match mod text against stat entries
        let mut matched = false;
        for (_, entries) in db.iter() {
            for entry in entries {
                total_entries += 1;
                // Convert stat text "#% increased Movement Speed" → regex
                // Replace # with number capture group
                let escaped = regex::escape(&entry.text);
                let pattern = escaped.replace("#", r"(\d+(?:\.\d+)?)");
                if let Ok(re) = regex::Regex::new(&format!("(?i)^{}$", pattern)) {
                    if re.is_match(&cleaned) {
                        if let Some(caps) = re.captures(&cleaned) {
                            if let Some(m) = caps.get(1) {
                                if let Ok(val) = m.as_str().parse::<f64>() {
                                    filters.push(serde_json::json!({
                                        "id": entry.id,
                                        "value": { "min": val },
                                        "disabled": false
                                    }));
                                    matched = true;
                                    matched_lines += 1;
                                    eprintln!("[WarlordTools] MATCH: '{}' -> {} (min={})", cleaned, entry.id, val);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            if matched { break; }
        }
        if !matched {
            eprintln!("[WarlordTools] NO MATCH: '{}' (nums: {:?})", cleaned, nums);
        }
    }
    eprintln!("[WarlordTools] match_mods_to_stats: {} filters from {} lines, scanned {} entries", filters.len(), matched_lines, total_entries);
    filters
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
                let last_trigger = std::sync::Arc::new(Mutex::new(None::<std::time::Instant>));

                app.handle().plugin(
                    tauri_plugin_global_shortcut::Builder::new()
                        .with_handler({
                            let last_trigger = last_trigger.clone();
                            move |_app, shortcut, event| {
                            if event.state != ShortcutState::Pressed {
                                return;
                            }
                            // Prevent double-fire within 2 seconds
                            {
                                let mut last = last_trigger.lock().unwrap();
                                if let Some(t) = *last {
                                    if t.elapsed() < std::time::Duration::from_secs(2) {
                                        eprintln!("[WarlordTools] 忽略重复触发");
                                        return;
                                    }
                                }
                                *last = Some(std::time::Instant::now());
                            }
                            // Ctrl+D = quick price check
                            let is_ctrl_d = shortcut.matches(Modifiers::CONTROL, Code::KeyD);
                            // Ctrl+Alt+D = sticky price check
                            let is_ctrl_alt_d = shortcut.matches(Modifiers::CONTROL | Modifiers::ALT, Code::KeyD);

                            if is_ctrl_d || is_ctrl_alt_d {
                                eprintln!("[WarlordTools] 快捷键触发: {}", if is_ctrl_alt_d { "Ctrl+Alt+D" } else { "Ctrl+D" });
                                // User manually presses Ctrl+C in-game, we just read the clipboard
                                match arboard::Clipboard::new() {
                                    Ok(mut clipboard) => {
                                        match clipboard.get_text() {
                                            Ok(text) => {
                                                if text.is_empty() {
                                                    eprintln!("[WarlordTools] 剪贴板为空");
                                                } else {
                                                    eprintln!("[WarlordTools] 剪贴板 ({} chars): {}", text.len(), &text[..text.len().min(100)]);
                                                    let _ = handle.emit("price-check-triggered", serde_json::json!({
                                                        "text": text,
                                                        "sticky": is_ctrl_alt_d
                                                    }));
                                                }
                                            }
                                            Err(e) => eprintln!("[WarlordTools] 剪贴板读取失败: {}", e),
                                        }
                                    }
                                    Err(e) => eprintln!("[WarlordTools] 剪贴板初始化失败: {}", e),
                                }
                            }
                            } // end closure
                        }) // end with_handler block
                        .with_shortcut("Ctrl+D")?
                        .with_shortcut("Ctrl+Alt+D")?
                        .build(),
                )?;
                eprintln!("[WarlordTools] 全局快捷键 Ctrl+D 和 Ctrl+Alt+D 已注册成功");
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
            fetch_leagues,
            search_trade,
            open_login_window,
            close_login_window,
            check_login_status,
            fetch_leagues_webview,
            search_trade_webview,
            open_trade_with_item,
            get_stat_db,
            fetch_stat_data_webview
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
