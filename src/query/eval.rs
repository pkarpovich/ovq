use super::ast::{CompareOp, Date, Expr, Value};
use serde_json::Value as JsonValue;

pub fn evaluate(expr: &Expr, frontmatter: &JsonValue) -> bool {
    match expr {
        Expr::Compare { field, op, value } => eval_compare(frontmatter, field, *op, value),
        Expr::Contains { field, value } => eval_contains(frontmatter, field, value),
        Expr::Truthy { field, negated } => eval_truthy(frontmatter, field) != *negated,
        Expr::And(left, right) => evaluate(left, frontmatter) && evaluate(right, frontmatter),
        Expr::Or(left, right) => evaluate(left, frontmatter) || evaluate(right, frontmatter),
    }
}

fn get_field_case_insensitive<'a>(fm: &'a JsonValue, field: &str) -> Option<&'a JsonValue> {
    let object = fm.as_object()?;
    let field_lower = field.to_lowercase();
    for (key, value) in object {
        if key.to_lowercase() == field_lower {
            return Some(value);
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

fn eval_truthy(fm: &JsonValue, field: &str) -> bool {
    let Some(value) = get_field_case_insensitive(fm, field) else {
        return false;
    };

    match value {
        JsonValue::Null => false,
        JsonValue::Bool(b) => *b,
        JsonValue::String(s) => !s.is_empty(),
        JsonValue::Number(_) => true,
        JsonValue::Array(seq) => !seq.is_empty(),
        JsonValue::Object(map) => !map.is_empty(),
    }
}

fn eval_compare(fm: &JsonValue, field: &str, op: CompareOp, value: &Value) -> bool {
    try_eval_compare(fm, field, op, value).unwrap_or(false)
}

fn try_eval_compare(fm: &JsonValue, field: &str, op: CompareOp, value: &Value) -> Option<bool> {
    if let Value::Null = value {
        let field_is_null = get_field_case_insensitive(fm, field)
            .map(|v| v.is_null())
            .unwrap_or(true);
        return match op {
            CompareOp::Eq => Some(field_is_null),
            CompareOp::Ne => Some(!field_is_null),
            _ => None,
        };
    }

    let fm_value = get_field_case_insensitive(fm, field)?;

    match value {
        Value::String(s) => {
            let fm_str = json_to_string(fm_value)?;
            compare_str(&fm_str, s, op)
        }
        Value::Number(n) => {
            let fm_num = json_to_number(fm_value)?;
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
            let fm_date = json_to_date(fm_value)?;
            compare_ord(&fm_date, d, op)
        }
        Value::Null => None,
    }
}

fn eval_contains(fm: &JsonValue, field: &str, value: &Value) -> bool {
    let Some(fm_value) = get_field_case_insensitive(fm, field) else {
        return false;
    };

    let Value::String(needle) = value else {
        return false;
    };

    let needle_normalized = normalize_for_compare(needle);

    if let Some(arr) = fm_value.as_array() {
        return arr.iter().any(|item| {
            json_to_string(item)
                .map(|s| normalize_for_compare(&s) == needle_normalized)
                .unwrap_or(false)
        });
    }

    if let Some(s) = json_to_string(fm_value) {
        return normalize_for_compare(&s).contains(&needle_normalized);
    }

    false
}

fn json_to_string(v: &JsonValue) -> Option<String> {
    match v {
        JsonValue::String(s) => Some(s.clone()),
        JsonValue::Number(n) => Some(n.to_string()),
        JsonValue::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn json_to_number(v: &JsonValue) -> Option<f64> {
    v.as_f64()
}

fn json_to_date(v: &JsonValue) -> Option<Date> {
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
    use serde_json::json;

    #[test]
    fn test_string_eq() {
        let fm = json!({"status": "active"});
        let expr = Expr::Compare {
            field: "status".to_string(),
            op: CompareOp::Eq,
            value: Value::String("active".to_string()),
        };
        assert!(evaluate(&expr, &fm));
    }

    #[test]
    fn test_case_insensitive_field() {
        let fm = json!({"Status": "active"});
        let expr = Expr::Compare {
            field: "status".to_string(),
            op: CompareOp::Eq,
            value: Value::String("active".to_string()),
        };
        assert!(evaluate(&expr, &fm));
    }

    #[test]
    fn test_case_insensitive_value() {
        let fm = json!({"status": "ACTIVE"});
        let expr = Expr::Compare {
            field: "status".to_string(),
            op: CompareOp::Eq,
            value: Value::String("active".to_string()),
        };
        assert!(evaluate(&expr, &fm));
    }

    #[test]
    fn test_obsidian_link_stripping() {
        let fm = json!({"project": "[[Graph0mane]]"});
        let expr = Expr::Compare {
            field: "project".to_string(),
            op: CompareOp::Eq,
            value: Value::String("Graph0mane".to_string()),
        };
        assert!(evaluate(&expr, &fm));
    }

    #[test]
    fn test_contains_array() {
        let fm = json!({"tags": ["a", "b", "c"]});
        let expr = Expr::Contains {
            field: "tags".to_string(),
            value: Value::String("b".to_string()),
        };
        assert!(evaluate(&expr, &fm));
    }

    #[test]
    fn test_contains_case_insensitive() {
        let fm = json!({"tags": ["Project", "TODO"]});
        let expr = Expr::Contains {
            field: "tags".to_string(),
            value: Value::String("project".to_string()),
        };
        assert!(evaluate(&expr, &fm));
    }

    #[test]
    fn test_truthy_exists() {
        let fm = json!({"date": "2024-01-01"});
        let expr = Expr::Truthy {
            field: "date".to_string(),
            negated: false,
        };
        assert!(evaluate(&expr, &fm));
    }

    #[test]
    fn test_truthy_missing() {
        let fm = json!({"status": "active"});
        let expr = Expr::Truthy {
            field: "date".to_string(),
            negated: false,
        };
        assert!(!evaluate(&expr, &fm));
    }

    #[test]
    fn test_truthy_negated() {
        let fm = json!({"status": "active"});
        let expr = Expr::Truthy {
            field: "date".to_string(),
            negated: true,
        };
        assert!(evaluate(&expr, &fm));
    }

    #[test]
    fn test_truthy_empty_string() {
        let fm = json!({"date": ""});
        let expr = Expr::Truthy {
            field: "date".to_string(),
            negated: false,
        };
        assert!(!evaluate(&expr, &fm));
    }

    #[test]
    fn test_null_eq_missing() {
        let fm = json!({"status": "active"});
        let expr = Expr::Compare {
            field: "date".to_string(),
            op: CompareOp::Eq,
            value: Value::Null,
        };
        assert!(evaluate(&expr, &fm));
    }

    #[test]
    fn test_null_ne_exists() {
        let fm = json!({"date": "2024-01-01"});
        let expr = Expr::Compare {
            field: "date".to_string(),
            op: CompareOp::Ne,
            value: Value::Null,
        };
        assert!(evaluate(&expr, &fm));
    }

    #[test]
    fn test_null_eq_exists() {
        let fm = json!({"date": "2024-01-01"});
        let expr = Expr::Compare {
            field: "date".to_string(),
            op: CompareOp::Eq,
            value: Value::Null,
        };
        assert!(!evaluate(&expr, &fm));
    }
}
