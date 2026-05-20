use serde_yaml::Value as YamlValue;
use std::path::Path;

pub fn format_path(path: &Path, vault: &Path) -> String {
    path.strip_prefix(vault)
        .unwrap_or(path)
        .display()
        .to_string()
}

pub fn parse_fields(spec: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for piece in spec.split(',') {
        let trimmed = piece.trim();
        if trimmed.is_empty() {
            continue;
        }
        if out.iter().any(|x| x == trimmed) {
            continue;
        }
        out.push(trimmed.to_string());
    }
    out
}

pub fn format_tsv(path: &Path, vault: &Path, fm: &YamlValue, fields: &[&str]) -> String {
    let mut cells: Vec<String> = Vec::with_capacity(fields.len() + 1);
    cells.push(format_path(path, vault));
    for field in fields {
        let cell = match lookup_field_ci(fm, field) {
            Some(v) => render_cell_for_tsv(v),
            None => String::new(),
        };
        cells.push(cell);
    }
    cells.join("\t")
}

pub(crate) fn lookup_field_ci<'a>(fm: &'a YamlValue, name: &str) -> Option<&'a YamlValue> {
    let mapping = fm.as_mapping()?;
    let name_lower = name.to_lowercase();
    for (key, value) in mapping {
        let Some(key_str) = key.as_str() else {
            continue;
        };
        if key_str.to_lowercase() == name_lower {
            return Some(value);
        }
    }
    None
}

pub(crate) fn yaml_to_json(v: &YamlValue) -> serde_json::Value {
    match v {
        YamlValue::Null => serde_json::Value::Null,
        YamlValue::Bool(b) => serde_json::Value::Bool(*b),
        YamlValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                serde_json::Value::Number(i.into())
            } else if let Some(u) = n.as_u64() {
                serde_json::Value::Number(u.into())
            } else if let Some(f) = n.as_f64() {
                serde_json::Number::from_f64(f)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            } else {
                serde_json::Value::Null
            }
        }
        YamlValue::String(s) => serde_json::Value::String(s.clone()),
        YamlValue::Sequence(seq) => {
            serde_json::Value::Array(seq.iter().map(yaml_to_json).collect())
        }
        YamlValue::Mapping(map) => {
            let mut obj = serde_json::Map::new();
            for (k, val) in map {
                let key = match k {
                    YamlValue::String(s) => s.clone(),
                    YamlValue::Bool(b) => b.to_string(),
                    YamlValue::Number(n) => n.to_string(),
                    _ => continue,
                };
                obj.insert(key, yaml_to_json(val));
            }
            serde_json::Value::Object(obj)
        }
        YamlValue::Tagged(t) => yaml_to_json(&t.value),
    }
}

fn normalize_cell(s: &str) -> String {
    s.chars()
        .map(|c| if c == '\t' || c == '\n' { ' ' } else { c })
        .collect()
}

fn scalar_for_cell(v: &YamlValue) -> String {
    match v {
        YamlValue::Null => String::new(),
        YamlValue::Bool(b) => b.to_string(),
        YamlValue::Number(n) => n.to_string(),
        YamlValue::String(s) => normalize_cell(s),
        _ => String::new(),
    }
}

fn render_cell_for_tsv(v: &YamlValue) -> String {
    match v {
        YamlValue::Sequence(seq) => {
            let parts: Vec<String> = seq
                .iter()
                .map(|item| match item {
                    YamlValue::Mapping(_) | YamlValue::Sequence(_) => {
                        serde_json::to_string(&yaml_to_json(item)).unwrap_or_default()
                    }
                    _ => scalar_for_cell(item),
                })
                .collect();
            normalize_cell(&parts.join(", "))
        }
        YamlValue::Mapping(_) => {
            let json = yaml_to_json(v);
            normalize_cell(&serde_json::to_string(&json).unwrap_or_default())
        }
        _ => scalar_for_cell(v),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml::from_str;
    use std::path::PathBuf;

    #[test]
    fn parse_fields_single_value() {
        assert_eq!(parse_fields("status"), vec!["status".to_string()]);
    }

    #[test]
    fn parse_fields_multiple_trimmed() {
        assert_eq!(
            parse_fields(" a , b ,c"),
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn parse_fields_drops_empties() {
        assert_eq!(
            parse_fields("a,,b"),
            vec!["a".to_string(), "b".to_string()]
        );
    }

    #[test]
    fn parse_fields_dedupes_preserving_first_occurrence() {
        assert_eq!(
            parse_fields("a,b,a,c,b"),
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn parse_fields_empty_input() {
        assert!(parse_fields("").is_empty());
        assert!(parse_fields("   ").is_empty());
        assert!(parse_fields(",,").is_empty());
    }

    fn vault() -> PathBuf {
        PathBuf::from("/vault")
    }

    fn note_path(name: &str) -> PathBuf {
        PathBuf::from(format!("/vault/{}", name))
    }

    #[test]
    fn format_tsv_all_scalars_present() {
        let fm: YamlValue = from_str("status: active\nrating: 8\ndone: true").unwrap();
        let line = format_tsv(
            &note_path("a.md"),
            &vault(),
            &fm,
            &["status", "rating", "done"],
        );
        assert_eq!(line, "a.md\tactive\t8\ttrue");
    }

    #[test]
    fn format_tsv_missing_field_empty_column() {
        let fm: YamlValue = from_str("status: active").unwrap();
        let line = format_tsv(&note_path("a.md"), &vault(), &fm, &["status", "missing"]);
        assert_eq!(line, "a.md\tactive\t");
    }

    #[test]
    fn format_tsv_case_insensitive_lookup() {
        let fm: YamlValue = from_str("status: active").unwrap();
        let line = format_tsv(&note_path("a.md"), &vault(), &fm, &["Status"]);
        assert_eq!(line, "a.md\tactive");
    }

    #[test]
    fn format_tsv_sequence_joined_with_comma_space() {
        let fm: YamlValue = from_str("tags: [a, b, c]").unwrap();
        let line = format_tsv(&note_path("a.md"), &vault(), &fm, &["tags"]);
        assert_eq!(line, "a.md\ta, b, c");
    }

    #[test]
    fn format_tsv_mapping_rendered_as_compact_json() {
        let fm: YamlValue = from_str("cover:\n  url: x\n  width: 500").unwrap();
        let line = format_tsv(&note_path("a.md"), &vault(), &fm, &["cover"]);
        assert!(line.contains("\"url\":\"x\""));
        assert!(line.contains("\"width\":500"));
        assert!(line.starts_with("a.md\t{"));
        assert!(line.ends_with("}"));
    }

    #[test]
    fn format_tsv_tab_and_newline_normalised_to_space() {
        let fm: YamlValue = from_str("note: \"line1\\tmid\\nline2\"").unwrap();
        let line = format_tsv(&note_path("a.md"), &vault(), &fm, &["note"]);
        assert_eq!(line, "a.md\tline1 mid line2");
    }

    #[test]
    fn format_tsv_mixed_types() {
        let fm: YamlValue =
            from_str("title: Hello\nrating: 9\nactive: true\ndate: 2024-01-15").unwrap();
        let line = format_tsv(
            &note_path("a.md"),
            &vault(),
            &fm,
            &["title", "rating", "active", "date"],
        );
        assert_eq!(line, "a.md\tHello\t9\ttrue\t2024-01-15");
    }

    #[test]
    fn format_tsv_wiki_link_kept_raw() {
        let fm: YamlValue = from_str("author: \"[[Steve Jobs]]\"").unwrap();
        let line = format_tsv(&note_path("a.md"), &vault(), &fm, &["author"]);
        assert_eq!(line, "a.md\t[[Steve Jobs]]");
    }

    #[test]
    fn format_path_vault_relative() {
        assert_eq!(
            format_path(&PathBuf::from("/vault/sub/a.md"), &PathBuf::from("/vault")),
            "sub/a.md"
        );
    }

    #[test]
    fn format_path_absolute_fallback_when_outside_vault() {
        assert_eq!(
            format_path(&PathBuf::from("/other/a.md"), &PathBuf::from("/vault")),
            "/other/a.md"
        );
    }
}
