use serde_yaml::Value;
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

    serde_yaml::from_str(yaml_str).ok()
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
}
