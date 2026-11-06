import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./styles/overlay.css";
import "./styles/resume.css";
import "./styles/memo.css";

// IPC types: `Grabbed` is emitted by Rust; the args mirror the Rust command parameters.
type Kind = "table" | "prose";
type Template = "resume" | "memo";
interface Grabbed { kind: Kind; text: string }
interface ExportDocxArgs { text: string; template: Template }
interface ExportTableArgs { text: string; schema: string; format: "xlsx" | "csv"; ai: boolean }

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
const status = $("status"), printRoot = $("print-root");
const tpl = $<HTMLSelectElement>("tpl"), schema = $<HTMLSelectElement>("schema");

let text = "";
let kind: Kind = "prose";
const say = (m: string) => (status.textContent = m);
const hide = () => invoke("hide_overlay");

listen<Grabbed>("grabbed", (e) => {
  text = e.payload.text;
  kind = e.payload.kind;
  document.querySelector('[data-act="xlsx"]')!.classList.toggle("rec", kind === "table");
  document.querySelector('[data-act="pdf"]')!.classList.toggle("rec", kind === "prose");
  $("preview").textContent = `${e.payload.kind === "table" ? "📊" : "📄"} ${text.replace(/\s+/g, " ").slice(0, 60)}`;
  say("");
});

$("close").addEventListener("click", hide);
document.addEventListener("keydown", (e) => e.key === "Escape" && hide());
window.addEventListener("afterprint", hide);

document.addEventListener("click", async (e) => {
  const act = (e.target as HTMLElement).dataset.act;
  if (!act) return;
  try {
    if (act === "pdf") {
      // WebView2 print dialog -> "Microsoft Print to PDF" / Save as PDF.
      printRoot.className = tpl.value;
      printRoot.innerHTML = await invoke<string>("render_html", { text });
      await new Promise((r) => setTimeout(r, 50)); // let the DOM paint before the print snapshot
      window.print();
    } else if (act === "docx") {
      const args: ExportDocxArgs = { text, template: tpl.value as Template };
      say(`Saved ${await invoke<string>("export_docx", { ...args })}`);
    } else {
      // xlsx/csv: Rust splits locally when the text is a real grid, else (prose) asks the AI.
      const ai = act === "ai" || kind === "prose";
      if (ai) say("Asking AI…");
      const args: ExportTableArgs = { text, schema: schema.value, format: act === "csv" ? "csv" : "xlsx", ai: act === "ai" };
      say(`Saved ${await invoke<string>("export_table", { ...args })}`);
    }
  } catch (err) {
    say(`Error: ${err}`);
  }
});
