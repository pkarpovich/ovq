use serde_json::Value;
use std::fs;
use std::path::Path;

pub fn parse_frontmatter(path: &Path) -> Option<Value> {
    let content = fs::read_to_string(path).ok()?;
    extract_and_parse(&content)
}

fn extract_and_parse(content: &str) -> Option<Value> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return None;
    }

    let after_first = &trimmed[3..];
    let end_idx = after_first.find("\n---")?;
    let yaml_str = &after_first[..end_idx];

    serde_saphyr::from_str::<Value>(yaml_str).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_frontmatter() {
        let content = r#"---
title: Test
tags: [a, b]
---
Body content"#;
        let fm = extract_and_parse(content).unwrap();
        assert_eq!(fm["title"], "Test");
    }

    #[test]
    fn test_no_frontmatter() {
        let content = "Just body content";
        assert!(extract_and_parse(content).is_none());
    }

    #[test]
    fn yaml_date_lands_as_iso_string() {
        let content = "---\ndate: 2024-12-16\n---\nBody";
        let fm = extract_and_parse(content).unwrap();
        assert_eq!(fm["date"], "2024-12-16");
        assert!(fm["date"].is_string());
    }

    #[test]
    fn yaml_array_of_strings_stays_array() {
        let content = "---\ntags: [project, todo, idea]\n---\nBody";
        let fm = extract_and_parse(content).unwrap();
        let arr = fm["tags"].as_array().expect("tags should be a JSON array");
        let values: Vec<&str> = arr.iter().map(|v| v.as_str().unwrap()).collect();
        assert_eq!(values, vec!["project", "todo", "idea"]);
    }
}
