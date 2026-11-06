use crate::parser::table;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::{thread::sleep, time::Duration};
use tauri::AppHandle;
use tauri_plugin_clipboard_manager::ClipboardExt;

/// Send Ctrl+C to the focused app, return the copied text, then restore the old clipboard.
// shortcut: restores text only; an image/file clipboard is lost, upgrade by snapshotting raw formats.
pub fn grab(app: &AppHandle) -> Result<String, String> {
    let cb = app.clipboard();
    let prev = cb.read_text().ok();
    let _ = cb.clear(); // so "unchanged" can't be mistaken for "copied"

    let e = |e: enigo::InputError| e.to_string();
    let mut kb = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
    // The hotkey's own Alt+Shift are still held; release them or the app sees Ctrl+Alt+Shift+C.
    kb.key(Key::Alt, Direction::Release).map_err(e)?;
    kb.key(Key::Shift, Direction::Release).map_err(e)?;
    kb.key(Key::Control, Direction::Press).map_err(e)?;
    kb.key(Key::Unicode('c'), Direction::Click).map_err(e)?;
    kb.key(Key::Control, Direction::Release).map_err(e)?;

    // Retry until the clipboard has text (browsers can take a while on big selections).
    let mut got = String::new();
    for _ in 0..50 {
        sleep(Duration::from_millis(10));
        if let Ok(t) = cb.read_text() && !t.is_empty() {
            got = t;
            break;
        }
    }
    // The first non-empty read can be a partial write; let the source app finish, then re-read.
    // (Plain thread sleep: we run on a std thread, so no tokio needed.)
    sleep(Duration::from_millis(150));
    if let Ok(t) = cb.read_text() && !t.is_empty() {
        got = t;
    }
    // Web selections carry an HTML flavor; a real <table> beats guessing from plain text.
    // shortcut: a selection mixing a table and prose keeps only the table, upgrade by splicing.
    if let Some(rows) = arboard::Clipboard::new().ok().and_then(|mut c| c.get().html().ok()).and_then(|h| table::from_html(&h)) {
        got = rows.iter().map(|r| r.join("\t")).collect::<Vec<_>>().join("\n");
    }
    if let Some(p) = prev {
        let _ = cb.write_text(p);
    }
    if got.is_empty() { Err("nothing selected".into()) } else { Ok(got) }
}
