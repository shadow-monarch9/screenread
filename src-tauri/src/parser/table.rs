use serde_json::Value;

type Rows = Vec<Vec<String>>;

/// Detect a grid: a JSON array, or the longest run of >= 2 consecutive lines that are
/// tab-delimited, a Markdown pipe table, `Key: Value`, same-width CSV, or 2+-space aligned.
/// Bullets/numbered lists never count.
pub fn extract(text: &str) -> Option<Rows> {
    if let Some(r) = from_json(text) {
        return (r.len() >= 2).then_some(r);
    }
    // Trim spaces only: trailing tabs are empty cells, not padding.
    let lines: Vec<&str> = text.lines().map(|l| l.trim_matches(' ')).filter(|l| !l.is_empty()).collect();
    let modes: [fn(&str) -> Option<Vec<String>>; 5] = [tab_cells, pipe_cells, kv_cells, csv_cells, space_cells];
    modes.iter().find_map(|f| {
        let r: Rows = longest_run(&lines, f).into_iter().filter(|r| !is_separator(r)).collect();
        (r.len() >= 2).then_some(r)
    })
}

/// Longest run of consecutive lines that parse as cells with the same column count.
fn longest_run(lines: &[&str], cells: impl Fn(&str) -> Option<Vec<String>>) -> Rows {
    let (mut best, mut cur): (Rows, Rows) = (vec![], vec![]);
    for l in lines {
        match cells(l) {
            Some(c) if cur.last().is_none_or(|p| p.len() == c.len()) => cur.push(c),
            c => cur = c.into_iter().collect(), // run broken: restart (with this line if it parsed)
        }
        if cur.len() > best.len() {
            best = cur.clone();
        }
    }
    best
}

fn is_list_item(l: &str) -> bool {
    ["- ", "* ", "+ "].iter().any(|p| l.starts_with(p))
        || l.split_once(". ").is_some_and(|(n, _)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
}

fn is_separator(r: &[String]) -> bool {
    r.iter().all(|c| !c.is_empty() && c.chars().all(|ch| matches!(ch, '-' | ':')))
}

fn tab_cells(l: &str) -> Option<Vec<String>> {
    (!is_list_item(l) && l.contains('\t')).then(|| l.split('\t').map(|c| c.trim().to_string()).collect())
}

fn pipe_cells(l: &str) -> Option<Vec<String>> {
    l.starts_with('|').then(|| l.trim_matches('|').split('|').map(|c| c.trim().to_string()).collect())
}

fn csv_cells(l: &str) -> Option<Vec<String>> {
    // shortcut: cells > 40 chars or sentence-ending lines are treated as prose, upgrade if long-cell CSV matters.
    if is_list_item(l) || l.ends_with(['.', '!', '?']) {
        return None;
    }
    let c = split_csv(l);
    (c.len() >= 2 && c.iter().all(|x| x.len() <= 40)).then_some(c)
}

fn split_csv(line: &str) -> Vec<String> {
    let (mut cells, mut cur, mut quoted) = (vec![], String::new(), false);
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match (c, quoted) {
            ('"', true) if chars.peek() == Some(&'"') => { cur.push('"'); chars.next(); }
            ('"', _) => quoted = !quoted,
            (',', false) => cells.push(std::mem::take(&mut cur).trim().to_string()),
            _ => cur.push(c),
        }
    }
    cells.push(cur.trim().to_string());
    cells
}

/// `[{...},{...}]` (header = first object's keys) or `[[...],[...]]`.
fn from_json(text: &str) -> Option<Rows> {
    let v: Vec<Value> = serde_json::from_str(text.trim()).ok()?;
    if !v.iter().all(|r| r.is_object() || r.is_array()) {
        return None;
    }
    let s = |v: &Value| match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        o => o.to_string(),
    };
    Some(match v.first()?.as_object() {
        Some(o) => {
            let keys: Vec<&String> = o.keys().collect();
            std::iter::once(keys.iter().map(|k| k.to_string()).collect())
                .chain(v.iter().map(|r| keys.iter().map(|k| r.get(k.as_str()).map_or(String::new(), s)).collect()))
                .collect()
        }
        None => v.iter().map(|r| r.as_array().map_or(vec![], |a| a.iter().map(s).collect())).collect(),
    })
}

fn kv_cells(l: &str) -> Option<Vec<String>> {
    if is_list_item(l) {
        return None;
    }
    let (k, v) = l.split_once(':')?;
    let (k, v) = (k.trim_matches(['*', ' ']), v.trim_matches(['*', ' ']));
    // Short key only, so a long "Dear Sam: sentence" stays prose; "//" skips URLs.
    (!k.is_empty() && !v.is_empty() && k.len() <= 30 && k.split_whitespace().count() <= 4 && !v.starts_with("//"))
        .then(|| vec![k.to_string(), v.to_string()])
}

fn space_cells(l: &str) -> Option<Vec<String>> {
    if is_list_item(l) || l.ends_with(['.', '!', '?']) {
        return None;
    }
    let c: Vec<String> = l.split("  ").map(str::trim).filter(|x| !x.is_empty()).map(String::from).collect();
    (c.len() >= 2 && c.iter().all(|x| x.len() <= 40)).then_some(c)
}

/// First `<table>` of a clipboard HTML fragment as rows (tags stripped, entities decoded).
/// None unless it has >= 2 rows and >= 2 columns. colspan/rowspan are ignored.
pub fn from_html(html: &str) -> Option<Rows> {
    let lower = html.to_ascii_lowercase(); // same byte offsets as `html`
    let start = lower.find("<table")?;
    let end = lower[start..].find("</table>").map_or(lower.len(), |e| start + e);
    let (mut rows, mut pos): (Rows, usize) = (vec![], start);
    while let Some(i) = lower[pos..end].find("<tr") {
        let rs = pos + i;
        let re = lower[rs..end].find("</tr>").map_or(end, |e| rs + e);
        let cells = html_cells(&html[rs..re]);
        if !cells.is_empty() {
            rows.push(cells);
        }
        pos = re;
    }
    let w = rows.iter().map(Vec::len).max()?;
    rows.iter_mut().for_each(|r| r.resize(w, String::new()));
    (rows.len() >= 2 && w >= 2).then_some(rows)
}

fn html_cells(tr: &str) -> Vec<String> {
    let clean = |s: &str| {
        let s = s.replace("&nbsp;", " ").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&#39;", "'").replace("&amp;", "&");
        s.split_whitespace().collect::<Vec<_>>().join(" ")
    };
    let (mut cells, mut cur, mut open) = (vec![], String::new(), false);
    let mut rest = tr;
    while let Some(lt) = rest.find('<') {
        if open {
            cur.push_str(&rest[..lt]);
        }
        let (tag, next) = match rest[lt + 1..].find('>') {
            Some(g) => (&rest[lt + 1..lt + 1 + g], lt + 2 + g),
            None => ("", rest.len()),
        };
        let tag = tag.to_ascii_lowercase();
        let closing = tag.starts_with('/');
        let name = tag.trim_start_matches('/').split(|c: char| c.is_whitespace() || c == '/').next().unwrap_or("");
        match (closing, name) {
            (_, "td" | "th") => {
                if open {
                    cells.push(clean(&std::mem::take(&mut cur)));
                }
                open = !closing;
            }
            (_, "br") if open => cur.push(' '),
            _ => {}
        }
        rest = &rest[next..];
    }
    if open {
        cur.push_str(rest);
        cells.push(clean(&cur));
    }
    cells
}

/// Pipe-table Markdown, so tabular text can go through the same PDF pipeline as prose.
pub fn to_markdown(rows: &Rows) -> String {
    let line = |r: &Vec<String>| format!("| {} |", r.iter().map(|c| c.replace('|', "\\|")).collect::<Vec<_>>().join(" | "));
    let sep = format!("|{}|", vec!["---"; rows[0].len()].join("|"));
    format!("{}\n{sep}\n{}", line(&rows[0]), rows[1..].iter().map(line).collect::<Vec<_>>().join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn markdown_table() {
        let r = extract("| A | B |\n|---|:-:|\n| 1 | 2 |").unwrap();
        assert_eq!(r, vec![vec!["A", "B"], vec!["1", "2"]]);
    }
    #[test]
    fn tab_table_and_non_table() {
        assert_eq!(extract("a\tb\n1\t2").unwrap().len(), 2);
        assert!(extract("just prose").is_none());
    }
    #[test]
    fn csv_quotes_and_ragged() {
        assert_eq!(extract("a,b\n\"x,1\",\"q\"\"\"").unwrap()[1], vec!["x,1", "q\""]);
        assert!(extract("a,b\nc").is_none());
    }
    #[test]
    fn json_objects_keep_key_order_and_arrays() {
        assert_eq!(extract(r#"[{"z":1,"a":"x"},{"z":2}]"#).unwrap(), vec![vec!["z", "a"], vec!["1", "x"], vec!["2", ""]]);
        assert_eq!(extract(r#"[["h"],["v"]]"#).unwrap().len(), 2);
    }
    #[test]
    fn prose_lists_and_stray_tabs_are_not_tables() {
        assert!(extract("- a, b\n- c, d\n- e, f").is_none());
        assert!(extract("1. one, two\n2. three, four").is_none());
        assert!(extract("Hello there\tfriend\nplain line").is_none());
        assert!(extract("Dear Sam, thanks.\nI agree, mostly.").is_none());
    }
    #[test]
    fn table_inside_prose_is_found() {
        let r = extract("Here you go:\n\n| A | B |\n|---|---|\n| 1 | 2 |\n\nHope it helps").unwrap();
        assert_eq!(r, vec![vec!["A", "B"], vec!["1", "2"]]);
    }
    #[test]
    fn space_aligned_and_key_value() {
        assert_eq!(extract("Name    Age\nBob     42").unwrap(), vec![vec!["Name", "Age"], vec!["Bob", "42"]]);
        assert_eq!(extract("Name: Bob\n**Role:** Dev").unwrap(), vec![vec!["Name", "Bob"], vec!["Role", "Dev"]]);
        assert!(extract("Note: this one line only").is_none());
        assert!(extract("See https://a.b/c\nand https://d.e/f").is_none());
    }
    #[test]
    fn trailing_empty_tab_cells_keep_columns() {
        assert_eq!(extract("a\tb\t\n1\t2\t").unwrap()[0], vec!["a", "b", ""]);
    }
    #[test]
    fn html_table() {
        let h = "<html><body><!--StartFragment--><TABLE><tr><th>Name</th><th>Note</th></tr><tr><td>A &amp; B</td><td>x<br>y&nbsp;z</td></tr></TABLE><!--EndFragment-->";
        assert_eq!(from_html(h).unwrap(), vec![vec!["Name", "Note"], vec!["A & B", "x y z"]]);
        assert!(from_html("<p>no table</p>").is_none());
    }
    #[test]
    fn to_markdown_roundtrip() {
        let r = extract("a\tb\n1\t2").unwrap();
        assert_eq!(extract(&to_markdown(&r)).unwrap(), r);
    }
}
