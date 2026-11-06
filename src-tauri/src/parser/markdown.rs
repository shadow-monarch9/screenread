use pulldown_cmark::{Options, Parser, html};

pub fn to_html(md: &str) -> String {
    let mut out = String::new();
    let opts = Options::ENABLE_TABLES | Options::ENABLE_TASKLISTS | Options::ENABLE_STRIKETHROUGH;
    html::push_html(&mut out, Parser::new_ext(md, opts));
    out
}

/// Standalone document with the template CSS inlined (used for DOCX).
pub fn page(md: &str, template: &str) -> String {
    let (css, class) = if template == "memo" {
        (include_str!("../../../src/styles/memo.css"), "memo")
    } else {
        (include_str!("../../../src/styles/resume.css"), "resume")
    };
    format!(
        "<html><head><meta charset=\"utf-8\"><style>{css}</style></head><body class=\"{class}\">{}</body></html>",
        to_html(md)
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn renders_tables_and_tasks() {
        let h = super::to_html("| A | B |\n|---|---|\n| 1 | 2 |\n\n- [x] done");
        assert!(h.contains("<table>") && h.contains("<td>1</td>") && h.contains("checkbox"));
    }
    #[test]
    fn renders_basics() {
        let h = super::to_html("# T\n\n**b** `c`\n\n- x");
        assert!(h.contains("<h1>T</h1>") && h.contains("<strong>b</strong>") && h.contains("<code>c</code>") && h.contains("<li>x</li>"));
    }
}
