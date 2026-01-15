/// Evaluate a numeric literal string to its canonical JS string representation.
///
/// Handles decimal, hex (0x), octal (0o, legacy 0), binary (0b), scientific notation,
/// and bigint (n suffix).
/// Returns the canonical string that JS would use as an object key.
pub(crate) fn evaluate_numeric_literal(source: &str) -> String {
    let source = source.trim();

    // handle bigint suffix
    let (source, is_bigint) = if source.ends_with('n') || source.ends_with('N') {
        (&source[..source.len() - 1], true)
    } else {
        (source, false)
    };

    // empty after stripping
    if source.is_empty() {
        return "0".to_string();
    }

    // parse based on prefix
    let value: f64 = if source.len() > 2 {
        match &source[..2] {
            // hexadecimal
            "0x" | "0X" => i64::from_str_radix(&source[2..], 16)
                .map(|v| v as f64)
                .unwrap_or(f64::NAN),
            // ES6 octal
            "0o" | "0O" => i64::from_str_radix(&source[2..], 8)
                .map(|v| v as f64)
                .unwrap_or(f64::NAN),
            // binary
            "0b" | "0B" => i64::from_str_radix(&source[2..], 2)
                .map(|v| v as f64)
                .unwrap_or(f64::NAN),
            _ => parse_decimal_or_legacy_octal(source),
        }
    } else {
        parse_decimal_or_legacy_octal(source)
    };

    // convert to canonical string representation
    if is_bigint {
        // bigint: just the integer digits
        if value.is_finite() {
            format!("{}", value as i64)
        } else {
            "0".to_string()
        }
    } else {
        number_to_string(value)
    }
}

/// Parse a decimal number or legacy octal (starts with 0).
fn parse_decimal_or_legacy_octal(source: &str) -> f64 {
    // legacy octal: starts with 0 and contains only 0-7
    if source.starts_with('0')
        && source.len() > 1
        && source[1..].chars().all(|c| c.is_ascii_digit())
        && source.chars().skip(1).all(|c| c >= '0' && c <= '7')
    {
        return i64::from_str_radix(&source[1..], 8)
            .map(|v| v as f64)
            .unwrap_or_else(|_| source.parse().unwrap_or(f64::NAN));
    }

    // regular decimal (including scientific notation)
    source.parse().unwrap_or(f64::NAN)
}

/// Convert an f64 to its canonical JS string representation.
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
        // integer-valued, not too large: no decimal point
        format!("{}", value as i64)
    } else {
        // use JS-like representation
        let s = format!("{value}");
        // JS uses lowercase 'e' for scientific notation
        s.replace('E', "e")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluate_decimal() {
        assert_eq!(evaluate_numeric_literal("123"), "123");
        assert_eq!(evaluate_numeric_literal("0"), "0");
        assert_eq!(evaluate_numeric_literal("42"), "42");
    }

    #[test]
    fn test_evaluate_float() {
        assert_eq!(evaluate_numeric_literal("1.5"), "1.5");
        assert_eq!(evaluate_numeric_literal("3.14159"), "3.14159");
    }

    #[test]
    fn test_evaluate_scientific() {
        assert_eq!(evaluate_numeric_literal("1e2"), "100");
        assert_eq!(evaluate_numeric_literal("1e10"), "10000000000");
        assert_eq!(evaluate_numeric_literal("2e308"), "Infinity");
    }

    #[test]
    fn test_evaluate_hex() {
        assert_eq!(evaluate_numeric_literal("0x1a"), "26");
        assert_eq!(evaluate_numeric_literal("0XFF"), "255");
        assert_eq!(evaluate_numeric_literal("0x10"), "16");
    }

    #[test]
    fn test_evaluate_octal() {
        assert_eq!(evaluate_numeric_literal("0o17"), "15");
        assert_eq!(evaluate_numeric_literal("0O21"), "17");
    }

    #[test]
    fn test_evaluate_legacy_octal() {
        assert_eq!(evaluate_numeric_literal("017"), "15");
        assert_eq!(evaluate_numeric_literal("021"), "17");
    }

    #[test]
    fn test_evaluate_binary() {
        assert_eq!(evaluate_numeric_literal("0b101"), "5");
        assert_eq!(evaluate_numeric_literal("0B1111"), "15");
    }

    #[test]
    fn test_evaluate_bigint() {
        assert_eq!(evaluate_numeric_literal("123n"), "123");
        assert_eq!(evaluate_numeric_literal("0x10n"), "16");
    }
}
