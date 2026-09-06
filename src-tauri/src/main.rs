#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod md_to_docx;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Manager, State, WindowEvent};

/// Konfigurasi koneksi AI yang disimpan (model, api key, endpoint)
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
struct AiConfig {
    provider: String, // "groq", "openai", "anthropic", "custom"
    model: String,
    api_key: String,
    endpoint: String,
}

struct AppState {
    config: Mutex<Option<AiConfig>>,
    connected: Mutex<bool>,
}

fn config_path() -> PathBuf {
    let proj_dirs = ProjectDirs::from("com", "thecnoclaw", "TheCnoClaw")
        .expect("Tidak bisa menentukan folder config OS");
    let dir = proj_dirs.config_dir();
    fs::create_dir_all(dir).ok();
    dir.join("config.json")
}

fn load_config_from_disk() -> Option<AiConfig> {
    let path = config_path();
    if !path.exists() {
        return None;
    }
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn save_config_to_disk(cfg: &AiConfig) -> Result<(), String> {
    let path = config_path();
    let raw = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    fs::write(path, raw).map_err(|e| e.to_string())
}

/// Command: simpan konfigurasi API (dipanggil dari alur `$thecnoclaw/api`)
#[tauri::command]
fn save_ai_config(
    state: State<AppState>,
    provider: String,
    model: String,
    api_key: String,
    endpoint: String,
) -> Result<String, String> {
    let cfg = AiConfig {
        provider,
        model,
        api_key,
        endpoint,
    };
    save_config_to_disk(&cfg)?;
    *state.config.lock().unwrap() = Some(cfg);
    Ok("sukses".to_string())
}

/// Command: ambil konfigurasi tersimpan (untuk load saat app start / cek status)
#[tauri::command]
fn get_ai_config(state: State<AppState>) -> Option<AiConfig> {
    let mut guard = state.config.lock().unwrap();
    if guard.is_none() {
        *guard = load_config_from_disk();
    }
    guard.clone()
}

/// Command: connect / disconnect toggle (status logis, tidak menghapus config)
#[tauri::command]
fn set_connected(state: State<AppState>, connected: bool) -> bool {
    *state.connected.lock().unwrap() = connected;
    connected
}

#[tauri::command]
fn get_connected(state: State<AppState>) -> bool {
    *state.connected.lock().unwrap()
}

/// Command: hapus config (logout / ganti API key)
#[tauri::command]
fn clear_ai_config(state: State<AppState>) -> Result<(), String> {
    let path = config_path();
    if path.exists() {
        fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    *state.config.lock().unwrap() = None;
    *state.connected.lock().unwrap() = false;
    Ok(())
}

#[derive(Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

/// Command: kirim pesan ke AI provider (default: Groq, OpenAI-compatible endpoint)
#[tauri::command]
async fn send_chat(
    state: State<'_, AppState>,
    messages: Vec<ChatMessage>,
) -> Result<String, String> {
    let cfg = {
        let guard = state.config.lock().unwrap();
        guard.clone()
    };

    let cfg = cfg.ok_or_else(|| "Belum ada konfigurasi API. Jalankan 'api' dulu.".to_string())?;

    if cfg.api_key.trim().is_empty() {
        return Err("API key kosong. Set ulang lewat 'api'.".to_string());
    }

    let endpoint = if cfg.endpoint.trim().is_empty() {
        // default endpoint Groq (OpenAI-compatible)
        "https://api.groq.com/openai/v1/chat/completions".to_string()
    } else {
        cfg.endpoint.clone()
    };

    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "model": cfg.model,
        "messages": messages,
    });

    let resp = client
        .post(&endpoint)
        .bearer_auth(&cfg.api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Gagal menghubungi API: {}", e))?;

    let status = resp.status();
    let raw = resp
        .text()
        .await
        .map_err(|e| format!("Gagal membaca respons: {}", e))?;

    if !status.is_success() {
        return Err(format!("API error ({}): {}", status, raw));
    }

    let parsed: serde_json::Value =
        serde_json::from_str(&raw).map_err(|e| format!("Gagal parse JSON: {} | raw: {}", e, raw))?;

    let content = parsed["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("(tidak ada respons)")
        .to_string();

    Ok(content)
}

/// Command: simpan teks mentah ke file (untuk .txt / .md)
#[tauri::command]
fn save_text_file(path: String, content: String) -> Result<(), String> {
    fs::write(&path, content).map_err(|e| e.to_string())
}

/// Command: konversi markdown (dari jawaban AI) menjadi file .docx asli
/// dengan heading, bold, italic, list, dan tabel sungguhan.
#[tauri::command]
fn save_docx_file(path: String, markdown_content: String) -> Result<(), String> {
    let bytes = md_to_docx::markdown_to_docx_bytes(&markdown_content)?;
    fs::write(&path, bytes).map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState {
            config: Mutex::new(load_config_from_disk()),
            connected: Mutex::new(false),
        })
        .invoke_handler(tauri::generate_handler![
            save_ai_config,
            get_ai_config,
            set_connected,
            get_connected,
            clear_ai_config,
            send_chat,
            save_text_file,
            save_docx_file,
        ])
        .setup(|app| {
            // Menu untuk tray icon (klik kanan)
            let show_item = MenuItem::with_id(app, "show", "Buka TheCnoClaw", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Keluar Sepenuhnya", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("TheCnoClaw — klik untuk buka")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        button_state: tauri::tray::MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let is_visible = window.is_visible().unwrap_or(false);
                            if is_visible {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        // Menutup jendela (tombol X) hanya menyembunyikan, tidak mematikan app,
        // supaya popup/console tetap bisa dipanggil lagi dari tray.
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                window.hide().ok();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("error saat menjalankan aplikasi Tauri");
}
