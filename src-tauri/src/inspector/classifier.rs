use crate::parser::table;

/// "table" if the text parses as a grid (pipes, tabs, CSV, JSON array), else "prose".
pub fn classify(text: &str) -> &'static str {
    if table::extract(text).is_some() { "table" } else { "prose" }
}

#[cfg(test)]
mod tests {
    use super::classify;
    #[test]
    fn kinds() {
        assert_eq!(classify("| a | b |\n|-|-|\n| 1 | 2 |"), "table");
        assert_eq!(classify("a\tb\n1\t2"), "table");
        assert_eq!(classify("n,v\nx,1"), "table");
        assert_eq!(classify(r#"[{"a":1},{"a":2}]"#), "table");
        assert_eq!(classify("Name    Age\nBob     42"), "table");
        assert_eq!(classify("Name: Bob\nRole: Dev"), "table");
        assert_eq!(classify("# Title\n\nSome **prose**, with a comma.\n\n- item"), "prose");
    }
}
