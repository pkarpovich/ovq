use serde_json::Value;
use serde_saphyr::options::MergeKeyPolicy;
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

    let options = serde_saphyr::options! {
        strict_booleans: true,
        merge_keys: MergeKeyPolicy::AsOrdinary,
    };
    serde_saphyr::from_str_with_options::<Value>(yaml_str, options).ok()
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

    #[test]
    fn yaml_11_boolean_literals_stay_strings() {
        let content = "---\npublished: yes\ndraft: no\nactive: on\nhidden: off\n---\nBody";
        let fm = extract_and_parse(content).unwrap();
        assert_eq!(fm["published"], "yes");
        assert_eq!(fm["draft"], "no");
        assert_eq!(fm["active"], "on");
        assert_eq!(fm["hidden"], "off");
    }

    #[test]
    fn yaml_true_false_remain_booleans() {
        let content = "---\npublished: true\ndraft: false\n---\nBody";
        let fm = extract_and_parse(content).unwrap();
        assert_eq!(fm["published"], true);
        assert_eq!(fm["draft"], false);
    }

    #[test]
    fn yaml_merge_key_is_not_expanded() {
        let content = "---\ndefaults: &defs\n  a: 1\n  b: 2\ntarget:\n  <<: *defs\n  c: 3\n---\nBody";
        let fm = extract_and_parse(content).unwrap();
        let target = fm["target"].as_object().expect("target is a map");
        assert!(!target.contains_key("a"));
        assert!(!target.contains_key("b"));
        assert_eq!(target["c"], 3);
    }
}
