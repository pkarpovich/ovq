use super::ast::{CompareOp, Date, Expr, Value};
use serde_yaml::Value as YamlValue;

pub fn evaluate(expr: &Expr, frontmatter: &YamlValue) -> bool {
    match expr {
        Expr::Compare { field, op, value } => eval_compare(frontmatter, field, *op, value),
        Expr::Contains { field, value } => eval_contains(frontmatter, field, value),
        Expr::And(left, right) => evaluate(left, frontmatter) && evaluate(right, frontmatter),
        Expr::Or(left, right) => evaluate(left, frontmatter) || evaluate(right, frontmatter),
    }
}

fn get_field_case_insensitive<'a>(fm: &'a YamlValue, field: &str) -> Option<&'a YamlValue> {
    let mapping = fm.as_mapping()?;
    let field_lower = field.to_lowercase();
    for (key, value) in mapping {
        if let Some(key_str) = key.as_str() {
            if key_str.to_lowercase() == field_lower {
                return Some(value);
            }
        }
    }
    None
}

fn strip_obsidian_link(s: &str) -> &str {
    s.strip_prefix("[[")
        .and_then(|s| s.strip_suffix("]]"))
        .unwrap_or(s)
}

fn normalize_for_compare(s: &str) -> String {
    strip_obsidian_link(s).to_lowercase()
}

fn eval_compare(fm: &YamlValue, field: &str, op: CompareOp, value: &Value) -> bool {
    try_eval_compare(fm, field, op, value).unwrap_or(false)
}

fn try_eval_compare(fm: &YamlValue, field: &str, op: CompareOp, value: &Value) -> Option<bool> {
    let fm_value = get_field_case_insensitive(fm, field)?;

    match value {
        Value::String(s) => {
            let fm_str = yaml_to_string(fm_value)?;
            compare_str(&fm_str, s, op)
        }
        Value::Number(n) => {
            let fm_num = yaml_to_number(fm_value)?;
            compare_float(fm_num, *n, op)
        }
        Value::Bool(b) => {
            let fm_bool = fm_value.as_bool()?;
            match op {
                CompareOp::Eq => Some(fm_bool == *b),
                CompareOp::Ne => Some(fm_bool != *b),
                _ => None,
            }
        }
        Value::Date(d) => {
            let fm_date = yaml_to_date(fm_value)?;
            compare_ord(&fm_date, d, op)
        }
    }
}

fn eval_contains(fm: &YamlValue, field: &str, value: &Value) -> bool {
    let Some(fm_value) = get_field_case_insensitive(fm, field) else {
        return false;
    };

    let Value::String(needle) = value else {
        return false;
    };

    let needle_normalized = normalize_for_compare(needle);

    if let Some(arr) = fm_value.as_sequence() {
        return arr.iter().any(|item| {
            yaml_to_string(item)
                .map(|s| normalize_for_compare(&s) == needle_normalized)
                .unwrap_or(false)
        });
    }

    if let Some(s) = yaml_to_string(fm_value) {
        return normalize_for_compare(&s).contains(&needle_normalized);
    }

    false
}

fn yaml_to_string(v: &YamlValue) -> Option<String> {
    match v {
        YamlValue::String(s) => Some(s.clone()),
        YamlValue::Number(n) => Some(n.to_string()),
        YamlValue::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn yaml_to_number(v: &YamlValue) -> Option<f64> {
    v.as_f64().or_else(|| v.as_i64().map(|i| i as f64))
}

fn yaml_to_date(v: &YamlValue) -> Option<Date> {
    let s = v.as_str()?;
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let year: i32 = parts[0].parse().ok()?;
    let month: u8 = parts[1].parse().ok()?;
    let day: u8 = parts[2].parse().ok()?;
    Some(Date::new(year, month, day))
}

fn compare_str(a: &str, b: &str, op: CompareOp) -> Option<bool> {
    let a_norm = normalize_for_compare(a);
    let b_norm = normalize_for_compare(b);
    compare_ord(&a_norm, &b_norm, op)
}

fn compare_ord<T: Ord>(a: &T, b: &T, op: CompareOp) -> Option<bool> {
    Some(match op {
        CompareOp::Eq => a == b,
        CompareOp::Ne => a != b,
        CompareOp::Gt => a > b,
        CompareOp::Lt => a < b,
        CompareOp::Ge => a >= b,
        CompareOp::Le => a <= b,
    })
}

fn compare_float(a: f64, b: f64, op: CompareOp) -> Option<bool> {
    Some(match op {
        CompareOp::Eq => (a - b).abs() < f64::EPSILON,
        CompareOp::Ne => (a - b).abs() >= f64::EPSILON,
        CompareOp::Gt => a > b,
        CompareOp::Lt => a < b,
        CompareOp::Ge => a >= b,
        CompareOp::Le => a <= b,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml::from_str;

    #[test]
    fn test_string_eq() {
        let fm: YamlValue = from_str("status: active").unwrap();
        let expr = Expr::Compare {
            field: "status".to_string(),
            op: CompareOp::Eq,
            value: Value::String("active".to_string()),
        };
        assert!(evaluate(&expr, &fm));
    }

    #[test]
    fn test_case_insensitive_field() {
        let fm: YamlValue = from_str("Status: active").unwrap();
        let expr = Expr::Compare {
            field: "status".to_string(),
            op: CompareOp::Eq,
            value: Value::String("active".to_string()),
        };
        assert!(evaluate(&expr, &fm));
    }

    #[test]
    fn test_case_insensitive_value() {
        let fm: YamlValue = from_str("status: ACTIVE").unwrap();
        let expr = Expr::Compare {
            field: "status".to_string(),
            op: CompareOp::Eq,
            value: Value::String("active".to_string()),
        };
        assert!(evaluate(&expr, &fm));
    }

    #[test]
    fn test_obsidian_link_stripping() {
        let fm: YamlValue = from_str("project: \"[[Graph0mane]]\"").unwrap();
        let expr = Expr::Compare {
            field: "project".to_string(),
            op: CompareOp::Eq,
            value: Value::String("Graph0mane".to_string()),
        };
        assert!(evaluate(&expr, &fm));
    }

    #[test]
    fn test_contains_array() {
        let fm: YamlValue = from_str("tags: [a, b, c]").unwrap();
        let expr = Expr::Contains {
            field: "tags".to_string(),
            value: Value::String("b".to_string()),
        };
        assert!(evaluate(&expr, &fm));
    }

    #[test]
    fn test_contains_case_insensitive() {
        let fm: YamlValue = from_str("tags: [Project, TODO]").unwrap();
        let expr = Expr::Contains {
            field: "tags".to_string(),
            value: Value::String("project".to_string()),
        };
        assert!(evaluate(&expr, &fm));
    }
}
