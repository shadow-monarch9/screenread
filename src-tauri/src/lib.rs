mod compiler {
    pub mod exporter;
}
mod grabber {
    pub mod clipboard;
}
mod hooks {
    pub mod shortcut;
}
mod inspector {
    pub mod classifier;
}
mod parser {
    pub mod markdown;
    pub mod table;
}

use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition};

/// Hotkey entry point. Off the hotkey thread: grabbing sleeps while polling the clipboard.
pub fn on_shortcut(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        if let Err(e) = show_popup(&app) {
            eprintln!("{e}");
        }
    });
}

fn show_popup(app: &AppHandle) -> Result<(), String> {
    let w = app.get_webview_window("overlay").ok_or("no overlay window")?;
    if w.is_visible().unwrap_or(false) {
        // Focus must return to the source app before we send Ctrl+C.
        let _ = w.hide();
        std::thread::sleep(Duration::from_millis(80));
    }
    let cur = app.cursor_position().map_err(|e| e.to_string())?;
    let text = grabber::clipboard::grab(app)?;

    let size = w.outer_size().map_err(|e| e.to_string())?;
    let (mut x, mut y) = (cur.x + 12.0, cur.y + 12.0);
    if let Ok(Some(m)) = app.monitor_from_point(cur.x, cur.y) {
        let (p, s) = (m.position(), m.size());
        x = x.min(p.x as f64 + s.width as f64 - size.width as f64);
        y = y.min(p.y as f64 + s.height as f64 - size.height as f64);
    }
    let _ = w.set_position(PhysicalPosition::new(x, y));
    let _ = w.show();
    let _ = w.set_focus();
    w.emit("grabbed", serde_json::json!({ "kind": inspector::classifier::classify(&text), "text": text }))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn hide_overlay(app: AppHandle) {
    if let Some(w) = app.get_webview_window("overlay") {
        let _ = w.hide();
    }
}

/// Unambiguous grids (tab-delimited, JSON) become a Markdown table; everything else is Markdown already.
fn as_markdown(text: String) -> String {
    if text.contains('\t') || text.trim_start().starts_with('[') {
        if let Some(rows) = parser::table::extract(&text) {
            return parser::table::to_markdown(&rows);
        }
    }
    text
}

#[tauri::command]
fn render_html(text: String) -> String {
    parser::markdown::to_html(&as_markdown(text))
}

#[tauri::command]
fn export_docx(app: AppHandle, text: String, template: String) -> Result<String, String> {
    let bytes = compiler::exporter::docx(&parser::markdown::page(&as_markdown(text), &template))?;
    compiler::exporter::save(&app, "docx", &bytes)
}

/// `format` is "xlsx" or "csv". Local parse when the text is a real grid; otherwise (prose),
/// or when `ai` is set, route to AI instead of dumping lines into column A.
#[tauri::command]
async fn export_table(app: AppHandle, text: String, schema: String, format: String, ai: bool) -> Result<String, String> {
    let rows = match if ai { None } else { parser::table::extract(&text) } {
        Some(r) => r,
        None => extract_entities_with_ai(text, schema).await?,
    };
    let (ext, bytes) = if format == "csv" { ("csv", compiler::exporter::csv(&rows).into_bytes()) } else { ("xlsx", compiler::exporter::xlsx(&rows)?) };
    compiler::exporter::save(&app, ext, &bytes)
}

/// Opt-in only: reached from the "Extract Entities to Excel (AI)" button on prose.
async fn extract_entities_with_ai(text: String, schema: String) -> Result<Vec<Vec<String>>, String> {
    let cols = match schema.as_str() {
        "recruiter" => "Name, Role, Skills, Exp",
        "sales" => "Name, Title, Company, Email",
        "financial" => "Metric, Value, Period",
        _ => return Err(format!("unknown schema {schema}")),
    };
    let key = std::env::var("GEMINI_API_KEY").map_err(|_| "GEMINI_API_KEY is not set")?;
    let prompt = format!(
        "Extract the entities from the text below as a JSON array of arrays. The first row is the header [{cols}]; then one row per entity, same column order, empty string if unknown. Add nothing that is not in the text.

{text}"
    );
    let json = tauri::async_runtime::spawn_blocking(move || {
        let res: serde_json::Value = ureq::post("https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent")
            .set("x-goog-api-key", &key)
            .send_json(serde_json::json!({
                "contents": [{"parts": [{"text": prompt}]}],
                "generationConfig": {"responseMimeType": "application/json"}
            }))
            .map_err(|e| e.to_string())?
            .into_json()
            .map_err(|e| e.to_string())?;
        res["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| "AI returned no text".to_string())
    })
    .await
    .map_err(|e| e.to_string())??;
    parser::table::extract(&json).ok_or_else(|| "AI found no entities".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| hooks::shortcut::register(app.handle()))
        .invoke_handler(tauri::generate_handler![hide_overlay, render_html, export_docx, export_table])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
