use serde_yaml::Value as YamlValue;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub fn format_path(path: &Path, vault: &Path) -> String {
    path.strip_prefix(vault)
        .unwrap_or(path)
        .display()
        .to_string()
}

pub fn parse_fields(spec: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut seen_lower: Vec<String> = Vec::new();
    for piece in spec.split(',') {
        let trimmed = piece.trim();
        if trimmed.is_empty() {
            continue;
        }
        let key = trimmed.to_lowercase();
        if seen_lower.iter().any(|x| x == &key) {
            continue;
        }
        seen_lower.push(key);
        out.push(trimmed.to_string());
    }
    out
}

pub fn format_tsv(path: &Path, vault: &Path, fm: &YamlValue, fields: &[&str]) -> String {
    let mut cells: Vec<String> = Vec::with_capacity(fields.len() + 1);
    cells.push(normalize_cell(&format_path(path, vault)));
    for field in fields {
        let cell = match lookup_field_ci(fm, field) {
            Some(v) => render_cell_for_tsv(v),
            None => String::new(),
        };
        cells.push(cell);
    }
    cells.join("\t")
}

pub fn format_json_query(
    matches: &[(PathBuf, YamlValue)],
    vault: &Path,
    fields: Option<&[&str]>,
) -> String {
    let items: Vec<serde_json::Value> = matches
        .iter()
        .map(|(path, fm)| {
            let frontmatter_json = match fields {
                Some(list) => narrow_frontmatter(fm, list),
                None => yaml_to_json(fm),
            };
            let mut obj = serde_json::Map::new();
            obj.insert(
                "file".to_string(),
                serde_json::Value::String(format_path(path, vault)),
            );
            obj.insert("frontmatter".to_string(), frontmatter_json);
            serde_json::Value::Object(obj)
        })
        .collect();
    serde_json::to_string(&serde_json::Value::Array(items))
        .expect("serde_json::to_string of Value cannot fail")
}

pub fn format_json_values(counts: &HashMap<String, usize>, show_count: bool) -> String {
    let mut items: Vec<(String, usize)> = counts
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();

    if show_count {
        items.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    } else {
        items.sort_by(|a, b| a.0.cmp(&b.0));
    }

    let arr: Vec<serde_json::Value> = items
        .into_iter()
        .map(|(value, count)| {
            let mut obj = serde_json::Map::new();
            obj.insert("value".to_string(), serde_json::Value::String(value));
            if show_count {
                obj.insert(
                    "count".to_string(),
                    serde_json::Value::Number((count as u64).into()),
                );
            }
            serde_json::Value::Object(obj)
        })
        .collect();
    serde_json::to_string(&serde_json::Value::Array(arr))
        .expect("serde_json::to_string of Value cannot fail")
}

fn narrow_frontmatter(fm: &YamlValue, fields: &[&str]) -> serde_json::Value {
    let mut obj = serde_json::Map::new();
    for field in fields {
        let Some((key, value)) = lookup_field_ci_entry(fm, field) else {
            continue;
        };
        obj.insert(key.to_string(), yaml_to_json(value));
    }
    serde_json::Value::Object(obj)
}

fn lookup_field_ci_entry<'a>(fm: &'a YamlValue, name: &str) -> Option<(&'a str, &'a YamlValue)> {
    let mapping = fm.as_mapping()?;
    let name_lower = name.to_lowercase();
    for (key, value) in mapping {
        let Some(key_str) = key.as_str() else {
            continue;
        };
        if key_str.to_lowercase() == name_lower {
            return Some((key_str, value));
        }
    }
    None
}

pub(crate) fn lookup_field_ci<'a>(fm: &'a YamlValue, name: &str) -> Option<&'a YamlValue> {
    lookup_field_ci_entry(fm, name).map(|(_, v)| v)
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
        .map(|c| if c == '\t' || c == '\n' || c == '\r' { ' ' } else { c })
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
    fn parse_fields_dedupes_case_insensitively_preserving_first_spelling() {
        assert_eq!(
            parse_fields("status,Status,STATUS"),
            vec!["status".to_string()]
        );
        assert_eq!(
            parse_fields("Status,status"),
            vec!["Status".to_string()]
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
    fn format_tsv_carriage_return_normalised_to_space() {
        let fm: YamlValue = from_str("note: \"line1\\r\\nline2\"").unwrap();
        let line = format_tsv(&note_path("a.md"), &vault(), &fm, &["note"]);
        assert_eq!(line, "a.md\tline1  line2");
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
    fn format_tsv_path_with_tab_or_newline_normalised_to_space() {
        let fm: YamlValue = from_str("status: active").unwrap();
        let weird = PathBuf::from("/vault/has\ttab/and\nnewline.md");
        let line = format_tsv(&weird, &vault(), &fm, &["status"]);
        assert_eq!(line, "has tab/and newline.md\tactive");
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

    #[test]
    fn format_json_query_empty_matches() {
        let matches: Vec<(PathBuf, YamlValue)> = Vec::new();
        assert_eq!(format_json_query(&matches, &vault(), None), "[]");
    }

    #[test]
    fn format_json_query_single_match_full_frontmatter() {
        let fm: YamlValue = from_str("status: active\nrating: 8").unwrap();
        let matches = vec![(note_path("a.md"), fm)];
        let out = format_json_query(&matches, &vault(), None);
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed[0]["file"], "a.md");
        assert_eq!(parsed[0]["frontmatter"]["status"], "active");
        assert_eq!(parsed[0]["frontmatter"]["rating"], 8);
    }

    #[test]
    fn format_json_query_with_fields_narrows() {
        let fm: YamlValue = from_str("status: active\nrating: 8\nextra: skip").unwrap();
        let matches = vec![(note_path("a.md"), fm)];
        let out = format_json_query(&matches, &vault(), Some(&["status", "rating"]));
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        let fm_obj = parsed[0]["frontmatter"].as_object().unwrap();
        assert_eq!(fm_obj.len(), 2);
        assert!(fm_obj.contains_key("status"));
        assert!(fm_obj.contains_key("rating"));
        assert!(!fm_obj.contains_key("extra"));
    }

    #[test]
    fn format_json_query_with_fields_case_insensitive() {
        let fm: YamlValue = from_str("status: active").unwrap();
        let matches = vec![(note_path("a.md"), fm)];
        let out = format_json_query(&matches, &vault(), Some(&["Status"]));
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed[0]["frontmatter"]["status"], "active");
    }

    #[test]
    fn format_json_query_yaml_array_stays_array() {
        let fm: YamlValue = from_str("tags: [a, b, c]").unwrap();
        let matches = vec![(note_path("a.md"), fm)];
        let out = format_json_query(&matches, &vault(), None);
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed[0]["frontmatter"]["tags"], serde_json::json!(["a", "b", "c"]));
    }

    #[test]
    fn format_json_query_nested_map_stays_nested() {
        let fm: YamlValue = from_str("cover:\n  url: x\n  width: 500").unwrap();
        let matches = vec![(note_path("a.md"), fm)];
        let out = format_json_query(&matches, &vault(), None);
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed[0]["frontmatter"]["cover"]["url"], "x");
        assert_eq!(parsed[0]["frontmatter"]["cover"]["width"], 500);
    }

    #[test]
    fn format_json_query_yaml_date_renders_as_iso_string() {
        let fm: YamlValue = from_str("date: 2024-01-15").unwrap();
        let matches = vec![(note_path("a.md"), fm)];
        let out = format_json_query(&matches, &vault(), None);
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed[0]["frontmatter"]["date"], "2024-01-15");
    }

    #[test]
    fn format_json_query_vault_relative_path() {
        let fm: YamlValue = from_str("k: v").unwrap();
        let matches = vec![(PathBuf::from("/vault/sub/a.md"), fm)];
        let out = format_json_query(&matches, &vault(), None);
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed[0]["file"], "sub/a.md");
    }

    #[test]
    fn format_json_query_absolute_path_when_outside_vault() {
        let fm: YamlValue = from_str("k: v").unwrap();
        let matches = vec![(PathBuf::from("/other/a.md"), fm)];
        let out = format_json_query(&matches, &vault(), None);
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed[0]["file"], "/other/a.md");
    }

    #[test]
    fn format_json_query_is_compact_single_line() {
        let fm: YamlValue = from_str("status: active").unwrap();
        let matches = vec![(note_path("a.md"), fm)];
        let out = format_json_query(&matches, &vault(), None);
        assert!(!out.contains('\n'));
        assert!(!out.contains("  "));
    }

    #[test]
    fn format_json_values_empty_counts() {
        let counts: HashMap<String, usize> = HashMap::new();
        assert_eq!(format_json_values(&counts, false), "[]");
        assert_eq!(format_json_values(&counts, true), "[]");
    }

    #[test]
    fn format_json_values_without_count() {
        let mut counts: HashMap<String, usize> = HashMap::new();
        counts.insert("active".to_string(), 2);
        counts.insert("done".to_string(), 1);
        let out = format_json_values(&counts, false);
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed, serde_json::json!([{"value": "active"}, {"value": "done"}]));
    }

    #[test]
    fn format_json_values_with_count() {
        let mut counts: HashMap<String, usize> = HashMap::new();
        counts.insert("active".to_string(), 2);
        counts.insert("done".to_string(), 1);
        let out = format_json_values(&counts, true);
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(
            parsed,
            serde_json::json!([
                {"value": "active", "count": 2},
                {"value": "done", "count": 1}
            ])
        );
    }

    #[test]
    fn format_json_values_sorting_alphabetic_without_count() {
        let mut counts: HashMap<String, usize> = HashMap::new();
        counts.insert("zebra".to_string(), 1);
        counts.insert("apple".to_string(), 5);
        counts.insert("mango".to_string(), 3);
        let out = format_json_values(&counts, false);
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed[0]["value"], "apple");
        assert_eq!(parsed[1]["value"], "mango");
        assert_eq!(parsed[2]["value"], "zebra");
    }

    #[test]
    fn format_json_query_with_empty_fields_slice_yields_empty_frontmatter() {
        let fm: YamlValue = from_str("status: active\nrating: 8").unwrap();
        let matches = vec![(note_path("a.md"), fm)];
        let out = format_json_query(&matches, &vault(), Some(&[]));
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(parsed[0]["frontmatter"].as_object().unwrap().is_empty());
    }

    #[test]
    fn format_tsv_with_empty_fields_slice_is_path_only() {
        let fm: YamlValue = from_str("status: active").unwrap();
        let line = format_tsv(&note_path("a.md"), &vault(), &fm, &[]);
        assert_eq!(line, "a.md");
    }

    #[test]
    fn format_json_values_sorting_count_desc_then_alphabetic() {
        let mut counts: HashMap<String, usize> = HashMap::new();
        counts.insert("zebra".to_string(), 1);
        counts.insert("apple".to_string(), 5);
        counts.insert("banana".to_string(), 5);
        counts.insert("mango".to_string(), 3);
        let out = format_json_values(&counts, true);
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed[0]["value"], "apple");
        assert_eq!(parsed[1]["value"], "banana");
        assert_eq!(parsed[2]["value"], "mango");
        assert_eq!(parsed[3]["value"], "zebra");
    }
}
