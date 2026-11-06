use tauri::AppHandle;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, ShortcutState};

const MODS: Modifiers = Modifiers::ALT.union(Modifiers::SHIFT);

/// Alt+Shift+S grabs the current selection and opens the popup at the cursor.
pub fn register(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    app.plugin(
        tauri_plugin_global_shortcut::Builder::new()
            .with_handler(|app, sc, ev| {
                if ev.state() == ShortcutState::Pressed && sc.matches(MODS, Code::KeyS) {
                    crate::on_shortcut(app);
                }
            })
            .build(),
    )?;
    app.global_shortcut().register("alt+shift+s")?;
    Ok(())
}
