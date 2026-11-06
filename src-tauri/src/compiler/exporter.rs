use std::io::{Cursor, Write};
use tauri::{AppHandle, Manager};
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

const REL_NS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
const DOC_REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const CT_NS: &str = "http://schemas.openxmlformats.org/package/2006/content-types";
const RELS_CT: &str = "application/vnd.openxmlformats-package.relationships+xml";

fn xml_esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn zip_files(files: &[(&str, String)]) -> Result<Vec<u8>, String> {
    let mut z = ZipWriter::new(Cursor::new(Vec::new()));
    let opt = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    for (name, body) in files {
        z.start_file(*name, opt).map_err(|e| e.to_string())?;
        z.write_all(body.as_bytes()).map_err(|e| e.to_string())?;
    }
    Ok(z.finish().map_err(|e| e.to_string())?.into_inner())
}

fn root_rels(target: &str) -> String {
    format!("<?xml version=\"1.0\"?><Relationships xmlns=\"{REL_NS}\"><Relationship Id=\"r1\" Type=\"{DOC_REL}/officeDocument\" Target=\"{target}\"/></Relationships>")
}

pub fn csv(rows: &[Vec<String>]) -> String {
    let cell = |c: &String| {
        if c.contains([',', '"', '\n']) { format!("\"{}\"", c.replace('"', "\"\"")) } else { c.clone() }
    };
    rows.iter().map(|r| r.iter().map(cell).collect::<Vec<_>>().join(",")).collect::<Vec<_>>().join("\r\n")
}

fn col_name(mut i: usize) -> String {
    let mut s = String::new();
    loop {
        s.insert(0, (b'A' + (i % 26) as u8) as char);
        if i < 26 {
            return s;
        }
        i = i / 26 - 1;
    }
}

pub fn xlsx(rows: &[Vec<String>]) -> Result<Vec<u8>, String> {
    let mut data = String::new();
    for (r, row) in rows.iter().enumerate() {
        data += &format!("<row r=\"{}\">", r + 1);
        for (c, v) in row.iter().enumerate() {
            data += &format!("<c r=\"{}{}\" t=\"inlineStr\"><is><t>{}</t></is></c>", col_name(c), r + 1, xml_esc(v));
        }
        data += "</row>";
    }
    let main = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
    zip_files(&[
        ("[Content_Types].xml", format!("<?xml version=\"1.0\"?><Types xmlns=\"{CT_NS}\"><Default Extension=\"rels\" ContentType=\"{RELS_CT}\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/><Override PartName=\"/xl/worksheets/sheet1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/></Types>")),
        ("_rels/.rels", root_rels("xl/workbook.xml")),
        ("xl/workbook.xml", format!("<?xml version=\"1.0\"?><workbook xmlns=\"{main}\" xmlns:r=\"{DOC_REL}\"><sheets><sheet name=\"Sheet1\" sheetId=\"1\" r:id=\"r1\"/></sheets></workbook>")),
        ("xl/_rels/workbook.xml.rels", format!("<?xml version=\"1.0\"?><Relationships xmlns=\"{REL_NS}\"><Relationship Id=\"r1\" Type=\"{DOC_REL}/worksheet\" Target=\"worksheets/sheet1.xml\"/></Relationships>")),
        ("xl/worksheets/sheet1.xml", format!("<?xml version=\"1.0\"?><worksheet xmlns=\"{main}\"><sheetData>{data}</sheetData></worksheet>")),
    ])
}

/// DOCX that embeds the styled HTML as an altChunk; Word converts it on open.
// shortcut: Word-only fidelity (LibreOffice/Docs may ignore altChunk), upgrade to native w:p/w:tbl runs.
pub fn docx(html: &str) -> Result<Vec<u8>, String> {
    zip_files(&[
        ("[Content_Types].xml", format!("<?xml version=\"1.0\"?><Types xmlns=\"{CT_NS}\"><Default Extension=\"rels\" ContentType=\"{RELS_CT}\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Default Extension=\"html\" ContentType=\"text/html\"/><Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/></Types>")),
        ("_rels/.rels", root_rels("word/document.xml")),
        ("word/document.xml", format!("<?xml version=\"1.0\"?><w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\" xmlns:r=\"{DOC_REL}\"><w:body><w:altChunk r:id=\"rId1\"/></w:body></w:document>")),
        ("word/_rels/document.xml.rels", format!("<?xml version=\"1.0\"?><Relationships xmlns=\"{REL_NS}\"><Relationship Id=\"rId1\" Type=\"{DOC_REL}/aFChunk\" Target=\"afchunk.html\"/></Relationships>")),
        ("word/afchunk.html", html.to_string()),
    ])
}

/// Write into the user's Downloads folder; returns the full path.
pub fn save(app: &AppHandle, ext: &str, bytes: &[u8]) -> Result<String, String> {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let path = app.path().download_dir().map_err(|e| e.to_string())?.join(format!("ContextGrab-{secs}.{ext}"));
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn csv_quotes_and_columns() {
        assert_eq!(csv(&[vec!["a,b".into(), "q\"".into()]]), "\"a,b\",\"q\"\"\"");
        assert_eq!([0, 25, 26, 701, 702].map(col_name), ["A", "Z", "AA", "ZZ", "AAA"]);
    }
    #[test]
    fn zips_are_valid() {
        assert_eq!(&xlsx(&[vec!["x".into()]]).unwrap()[..2], b"PK");
        assert_eq!(&docx("<p>x</p>").unwrap()[..2], b"PK");
    }
}
