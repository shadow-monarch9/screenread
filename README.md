<div align="center">

# ContextGrab

**Smart Context Exporter & Formatter**

Turn screen selections into structured PDF, DOCX, XLSX, and CSV files in &lt;150ms.

Select text anywhere → press `Alt + Shift + S` → pick a format. Local-first, no account, no cloud unless you click the AI button.

</div>

---

## Quick start

1. Download the latest installer (`.exe` / `.msi` for Windows, `.dmg` for macOS) from the [Releases page](../../releases/latest).
2. Run it. The builds are unsigned, so Windows SmartScreen / macOS Gatekeeper will warn on first launch.
3. ContextGrab starts in the background with no window. Highlight text in any app and press **`Alt + Shift + S`**.

> Windows is the tested platform. macOS builds are produced by CI but have not been verified, and macOS needs Accessibility permission for ContextGrab to send the copy keystroke.

## How it works

| Step | What happens |
| :-- | :-- |
| `Alt + Shift + S` | Sends a native `Ctrl+C` to the focused app, reads the text, then restores your previous clipboard text. |
| Inspect | A local heuristic decides **table** vs **prose** (and uses the HTML `<table>` from web selections when present). |
| Popup | A small popup opens at your cursor. The recommended exporter is highlighted. |
| Export | Files are written to your **Downloads** folder (PDF goes through the print dialog). |

## Features

- **PDF** – Markdown rendered with a print stylesheet (Resume or Memo style) through the system print dialog, choose *Save as PDF*.
- **DOCX** – Styled document built locally.
- **XLSX / CSV** – Split locally, no AI needed, from Markdown pipe tables, tab-separated text, CSV, multi-space aligned columns, `Key: Value` lines, JSON arrays, and HTML tables copied from web pages.
- **Smart table detection** – Needs 2 or more consecutive, same-width lines. Bullet and numbered lists are never treated as tables.
- **Optional AI entity extraction (✨)** – Turns messy prose into a table using Gemini with a schema you pick (Recruiter, Sales Lead, Financial). Runs **only** when you click the AI button, or click XLSX/CSV on text that is not a table. Your selected text is then sent to Google's API.

### Enabling AI (optional)

Set a Gemini API key as an environment variable, then restart ContextGrab:

```powershell
setx GEMINI_API_KEY "your-key"
```

## Known limits

- Clipboard restore is **text only**: an image or file you had copied is lost after a grab.
- DOCX opens correctly in Microsoft Word; LibreOffice and Google Docs may not render it.
- A selection mixing a web table and prose keeps only the table; merged cells (`colspan`/`rowspan`) are not handled.
- Primary-monitor popup positioning is not verified on every multi-monitor layout.

## Feedback & bug reports

Early builds, so your feedback matters most. Please include your **persona**, **operating system**, and what you selected.

- 🐞 [Report a bug](../../issues/new?template=bug_report.md) – formatting issues, clipboard timing, UI glitches
- 💡 [Request a feature](../../issues/new?template=feature_request.md) – new export formats, custom print styles, workflow integrations

## Development

Requires [Node 22+](https://nodejs.org), [pnpm](https://pnpm.io), and [Rust](https://rustup.rs) (stable, 2024 edition).

```powershell
pnpm install
pnpm tauri dev      # run with hot reload
pnpm tauri build    # installers in src-tauri/target/release/bundle/
cargo test --manifest-path src-tauri/Cargo.toml
```

### Releasing

```bash
git tag v0.1.0
git push origin v0.1.0
```

The `release` workflow builds Windows and macOS installers and attaches them to a **draft** GitHub Release for you to review and publish.
