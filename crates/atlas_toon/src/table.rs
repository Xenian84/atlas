//! TOON table builder — deterministic column ordering, CSV rows.

/// Build a TOON table block.
/// Header: `{label}[{n}]{{{cols}}}:`
/// Rows:   one CSV line per item.
pub struct ToonTable {
    label:   String,
    columns: Vec<String>,
    rows:    Vec<Vec<String>>,
}

impl ToonTable {
    pub fn new(label: impl Into<String>, columns: Vec<&str>) -> Self {
        Self {
            label:   label.into(),
            columns: columns.into_iter().map(String::from).collect(),
            rows:    vec![],
        }
    }

    pub fn add_row(&mut self, values: Vec<String>) {
        self.rows.push(values);
    }

    /// Render the complete table block (with trailing newline).
    pub fn render(&self, indent: usize) -> String {
        let pad = " ".repeat(indent);
        if self.rows.is_empty() {
            return format!("{}{}[0]{{{}}}:\n", pad, self.label, self.columns.join(","));
        }
        let mut out = String::new();
        out.push_str(&format!(
            "{}{}[{}]{{{}}}:\n",
            pad,
            self.label,
            self.rows.len(),
            self.columns.join(",")
        ));
        for row in &self.rows {
            let escaped: Vec<String> = row.iter().map(|v| escape_csv(v)).collect();
            out.push_str(&format!("{} {}\n", pad, escaped.join(",")));
        }
        out
    }
}

/// Escape a single CSV field: wrap in quotes if it contains comma, newline, or quote.
fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains(' ') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else if s.is_empty() {
        "-".to_string()
    } else {
        s.to_string()
    }
}

/// Render a simple scalar list: `label[N]: v1 v2 v3`
pub fn render_list(label: &str, items: &[String], indent: usize) -> String {
    let pad = " ".repeat(indent);
    if items.is_empty() {
        return format!("{}{}[0]:\n", pad, label);
    }
    format!("{}{}[{}]: {}\n", pad, label, items.len(), items.join(" "))
}
