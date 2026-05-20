use crate::output::lookup_field_ci;
use serde_json::Value;
use std::collections::HashMap;

pub fn collect_values<'a, I>(frontmatters: I, property: &str) -> HashMap<String, usize>
where
    I: IntoIterator<Item = &'a Value>,
{
    let mut counts: HashMap<String, usize> = HashMap::new();

    for fm in frontmatters {
        let Some(value) = lookup_field_ci(fm, property) else {
            continue;
        };

        match value {
            Value::Array(arr) => {
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

fn value_to_string(v: &Value) -> Option<String> {
    match v {
        Value::String(s) if !s.is_empty() => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_collect_values() {
        let data = [
            json!({"status": "active"}),
            json!({"status": "done"}),
            json!({"status": "active"}),
        ];
        let counts = collect_values(data.iter(), "status");
        assert_eq!(counts.get("active"), Some(&2));
        assert_eq!(counts.get("done"), Some(&1));
    }

    #[test]
    fn test_collect_array_values() {
        let data = [json!({"tags": ["a", "b", "a"]})];

        let counts = collect_values(data.iter(), "tags");
        assert_eq!(counts.get("a"), Some(&2));
        assert_eq!(counts.get("b"), Some(&1));
    }

    #[test]
    fn test_collect_values_case_insensitive_property() {
        let data = [
            json!({"Status": "active"}),
            json!({"status": "done"}),
            json!({"STATUS": "active"}),
        ];
        let counts = collect_values(data.iter(), "status");
        assert_eq!(counts.get("active"), Some(&2));
        assert_eq!(counts.get("done"), Some(&1));
    }
}
