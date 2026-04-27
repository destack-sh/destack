/// Evaluate a numeric literal string to its canonical property-key string.
pub(crate) fn evaluate_numeric_literal(source: &str) -> String {
    let source = source.trim();

    // strip bigint suffix
    let (source, is_bigint) = if source.ends_with('n') || source.ends_with('N') {
        (&source[..source.len() - 1], true)
    } else {
        (source, false)
    };
    if source.is_empty() {
        return "0".to_string();
    }

    // parse numeric syntax accepted by the parser
    let value = if source.len() > 2 {
        match &source[..2] {
            "0x" | "0X" => i64::from_str_radix(&source[2..], 16)
                .map(|value| value as f64)
                .unwrap_or(f64::NAN),
            "0o" | "0O" => i64::from_str_radix(&source[2..], 8)
                .map(|value| value as f64)
                .unwrap_or(f64::NAN),
            "0b" | "0B" => i64::from_str_radix(&source[2..], 2)
                .map(|value| value as f64)
                .unwrap_or(f64::NAN),
            _ => parse_decimal_or_octal(source),
        }
    } else {
        parse_decimal_or_octal(source)
    };

    // convert into the key spelling used by JavaScript
    if is_bigint {
        if value.is_finite() {
            format!("{}", value as i64)
        } else {
            "0".to_string()
        }
    } else {
        number_to_string(value)
    }
}

/// Parse decimal syntax or legacy octal syntax.
fn parse_decimal_or_octal(source: &str) -> f64 {
    let is_octal = source.starts_with('0')
        && source.len() > 1
        && source[1..].chars().all(|c| c.is_ascii_digit())
        && source.chars().skip(1).all(|c| ('0'..='7').contains(&c));
    if is_octal {
        return i64::from_str_radix(&source[1..], 8)
            .map(|value| value as f64)
            .unwrap_or_else(|_| source.parse().unwrap_or(f64::NAN));
    }

    source.parse().unwrap_or(f64::NAN)
}

/// Convert a number into JavaScript-like display syntax.
fn number_to_string(value: f64) -> String {
    if value.is_nan() {
        "NaN".to_string()
    } else if value.is_infinite() {
        if value.is_sign_positive() {
            "Infinity".to_string()
        } else {
            "-Infinity".to_string()
        }
    } else if value == 0.0 {
        "0".to_string()
    } else if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        format!("{value}").replace('E', "e")
    }
}
