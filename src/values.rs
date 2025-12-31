use serde_yaml::Value as YamlValue;
use std::collections::HashMap;

pub fn collect_values(frontmatters: &[(String, YamlValue)], property: &str) -> HashMap<String, usize> {
    let mut counts: HashMap<String, usize> = HashMap::new();

    for (_, fm) in frontmatters {
        let Some(value) = fm.get(property) else {
            continue;
        };

        match value {
            YamlValue::Sequence(arr) => {
                for item in arr {
                    if let Some(s) = value_to_string(item) {
                        *counts.entry(s).or_default() += 1;
                    }
                }
            }
            _ => {
                if let Some(s) = value_to_string(value) {
                    *counts.entry(s).or_default() += 1;
                }
            }
        }
    }

    counts
}

pub fn format_values(counts: HashMap<String, usize>, show_count: bool) -> Vec<String> {
    let mut items: Vec<(String, usize)> = counts.into_iter().collect();

    if show_count {
        items.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        items
            .into_iter()
            .map(|(val, count)| format!("{}: {}", val, count))
            .collect()
    } else {
        items.sort_by(|a, b| a.0.cmp(&b.0));
        items.into_iter().map(|(val, _)| val).collect()
    }
}

fn value_to_string(v: &YamlValue) -> Option<String> {
    match v {
        YamlValue::String(s) if !s.is_empty() => Some(s.clone()),
        YamlValue::Number(n) => Some(n.to_string()),
        YamlValue::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml::from_str;

    #[test]
    fn test_collect_values() {
        let fm1: YamlValue = from_str("status: active").unwrap();
        let fm2: YamlValue = from_str("status: done").unwrap();
        let fm3: YamlValue = from_str("status: active").unwrap();

        let data = vec![
            ("a.md".to_string(), fm1),
            ("b.md".to_string(), fm2),
            ("c.md".to_string(), fm3),
        ];

        let counts = collect_values(&data, "status");
        assert_eq!(counts.get("active"), Some(&2));
        assert_eq!(counts.get("done"), Some(&1));
    }

    #[test]
    fn test_collect_array_values() {
        let fm: YamlValue = from_str("tags: [a, b, a]").unwrap();
        let data = vec![("x.md".to_string(), fm)];

        let counts = collect_values(&data, "tags");
        assert_eq!(counts.get("a"), Some(&2));
        assert_eq!(counts.get("b"), Some(&1));
    }
}
