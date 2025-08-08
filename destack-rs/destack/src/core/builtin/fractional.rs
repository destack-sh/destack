//! see https://observablehq.com/@dgreensp/implementing-fractional-indexing
//! sync with `destack/core/builtin/fractional.py` and `destack-ts/src/utils/fractional.ts`
// base 95 digits used for fractional indexing integers
const _BASE_95_DIGITS: &str =
    "!#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[]^_`abcdefghijklmnopqrstuvwxyz{|}~";

/// canonical zero integer for fractional indexing
pub const FRACTIONAL_INTEGER_ZERO: &str = "a0";
/// minimum representable integer for fractional indexing
pub const FRACTIONAL_INTEGER_MIN: &str = "A00000000000000000000000000";
/// maximum representable integer for fractional indexing
pub const FRACTIONAL_INTEGER_MAX: &str = "aZZZZZZZZZZZZZZZZZZZZZZZZZ";

#[derive(thiserror::Error, Debug)]
pub enum FractionalError {
    #[error("invalid order key head: {head}")]
    InvalidOrderKeyHead { head: char },
    #[error("trailing zero")]
    TrailingZero,
    #[error("invalid order key: {key}")]
    InvalidOrderKey { key: String },
    #[error("{a} >= {b}")]
    InvalidComparison { a: String, b: String },
}

/// Gets the length of the integer part of the given order key
fn _get_integer_length(head: char) -> Result<usize, FractionalError> {
    if head >= 'a' && head <= 'z' {
        Ok(head as usize - 'a' as usize + 2)
    } else if head >= 'A' && head <= 'Z' {
        Ok('Z' as usize - head as usize + 2)
    } else {
        Err(FractionalError::InvalidOrderKeyHead { head })
    }
}

/// Gets the midpoint between two strings, `a` and `b` in base 95
/// `a` may be empty string, `b` is `None` or non-empty string
/// `a < b` lexicographically if `b` is non-null
/// No trailing zeros allowed
fn _midpoint(a: &str, b: Option<&str>) -> Result<String, FractionalError> {
    // errors
    if let Some(bv) = b {
        if a >= bv {
            return Err(FractionalError::InvalidComparison {
                a: a.to_string(),
                b: bv.to_string(),
            });
        }
    }
    if (!a.is_empty() && a.ends_with('0')) || (b.is_some() && b.unwrap().ends_with('0')) {
        return Err(FractionalError::TrailingZero);
    }

    if let Some(bv) = b {
        // remove the longest common prefix; pad `a` with '0' as we go
        let mut n = 0usize;
        loop {
            let a_char = if n < a.len() {
                a.chars().nth(n).unwrap()
            } else {
                '0'
            };
            let b_char_opt = bv.chars().nth(n);
            if b_char_opt.is_none() || a_char != b_char_opt.unwrap() {
                break;
            }
            n += 1;
        }
        if n > 0 {
            let inner_midpoint = _midpoint(&a[n..], Some(&bv[n..]))?;
            return Ok(bv[..n].to_string() + &inner_midpoint);
        }
    }

    // first digits (or lack of digit) are different
    let digit_a = if !a.is_empty() {
        _BASE_95_DIGITS
            .find(a.chars().next().unwrap())
            .expect("invalid base95 digit in 'a'")
    } else {
        0
    };
    let digit_b = if let Some(bv) = b {
        if !bv.is_empty() {
            _BASE_95_DIGITS
                .find(bv.chars().next().unwrap())
                .expect("invalid base95 digit in 'b'")
        } else {
            _BASE_95_DIGITS.len()
        }
    } else {
        _BASE_95_DIGITS.len()
    };

    if digit_b - digit_a > 1 {
        // midpoint digit (round half up)
        let mid_digit = (digit_a + digit_b + 1) / 2; // equivalent to int(0.5 + 0.5*(a+b)) for integers
        Ok(_BASE_95_DIGITS.chars().nth(mid_digit).unwrap().to_string())
    } else {
        // first digits are consecutive
        if let Some(bv) = b {
            if bv.len() > 1 {
                return Ok(bv.chars().next().unwrap().to_string());
            }
        }
        // `b` is null or has length 1 (a single digit).
        // the first digit of `a` is the previous digit to `b`, or '9' if `b` is null.
        // given, for example, midpoint('49', '5'), return
        // '4' + midpoint('9', null), which will become '4' + '9' + midpoint('', null) => '495'
        let mut result = String::new();
        result.push(_BASE_95_DIGITS.chars().nth(digit_a).unwrap());
        result.push_str(&_midpoint(&a[1..], None)?);
        Ok(result)
    }
}

/// Gets the integer part of the given order key
fn _get_integer_part(key: &str) -> Result<String, FractionalError> {
    let head = key
        .chars()
        .next()
        .ok_or_else(|| FractionalError::InvalidOrderKey {
            key: key.to_string(),
        })?;
    let integer_part_length = _get_integer_length(head)?;
    if integer_part_length > key.len() {
        return Err(FractionalError::InvalidOrderKey {
            key: key.to_string(),
        });
    }
    Ok(key[..integer_part_length].to_string())
}

fn _is_valid_order_key(key: &str) -> bool {
    if key == FRACTIONAL_INTEGER_MIN {
        return false;
    }
    let integer_part = match _get_integer_part(key) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let fractional_part = &key[integer_part.len()..];
    !(fractional_part.len() > 0 && fractional_part.ends_with('0'))
}

/// Validates that the given key is a valid order key
fn _validate_order_key(key: &str) -> Result<(), FractionalError> {
    if !_is_valid_order_key(key) {
        return Err(FractionalError::InvalidOrderKey {
            key: key.to_string(),
        });
    }
    Ok(())
}

/// Increments the given integer `x` in base 95
/// Returns `None` if the result is too large
pub fn increment_integer(x: &str) -> Option<String> {
    // Increments integer part in base 95; returns None if result too large
    let mut chars = x.chars();
    let head = chars.next()?;
    let mut digs: Vec<char> = chars.collect();
    let mut carry = true;
    for i in (0..digs.len()).rev() {
        let d_index = _BASE_95_DIGITS.find(digs[i]).unwrap() + 1;
        if d_index == _BASE_95_DIGITS.len() {
            digs[i] = '0';
        } else {
            digs[i] = _BASE_95_DIGITS.chars().nth(d_index).unwrap();
            carry = false;
            break;
        }
    }
    if carry {
        if head == 'Z' {
            return Some("a0".to_string());
        }
        if head == 'z' {
            return None;
        }
        let h = (head as u8 + 1) as char;
        if h > 'a' {
            digs.push('0');
        } else {
            digs.pop();
        }
        let mut out = String::new();
        out.push(h);
        out.extend(digs);
        Some(out)
    } else {
        let mut out = String::new();
        out.push(head);
        out.extend(digs);
        Some(out)
    }
}

/// Decrements the given integer `x` in base 95
pub fn decrement_integer(x: &str) -> Option<String> {
    // Decrements integer part in base 95; returns None if result too small
    let mut chars = x.chars();
    let head = chars.next()?;
    let mut digs: Vec<char> = chars.collect();
    let mut borrow = true;
    for i in (0..digs.len()).rev() {
        let d_index = _BASE_95_DIGITS.find(digs[i]).unwrap() as isize - 1;
        if d_index == -1 {
            digs[i] = _BASE_95_DIGITS.chars().last().unwrap();
        } else {
            digs[i] = _BASE_95_DIGITS.chars().nth(d_index as usize).unwrap();
            borrow = false;
        }
    }
    if borrow {
        if head == 'a' {
            let mut out = String::new();
            out.push('Z');
            out.push(_BASE_95_DIGITS.chars().last().unwrap());
            return Some(out);
        }
        if head == 'A' {
            return None;
        }
        let h = (head as u8 - 1) as char;
        if h < 'Z' {
            digs.push(_BASE_95_DIGITS.chars().last().unwrap());
        } else {
            digs.pop();
        }
        let mut out = String::new();
        out.push(h);
        out.extend(digs);
        Some(out)
    } else {
        let mut out = String::new();
        out.push(head);
        out.extend(digs);
        Some(out)
    }
}

/// Generates a key between the given keys `a` and `b` (inclusive) with logarithmic fraction growth
pub fn get_order_key(a: Option<&str>, b: Option<&str>) -> Result<String, FractionalError> {
    // validate
    if let Some(av) = a {
        _validate_order_key(av)?;
    }
    if let Some(bv) = b {
        _validate_order_key(bv)?;
    }
    if let (Some(av), Some(bv)) = (a, b) {
        if av >= bv {
            return Err(FractionalError::InvalidComparison {
                a: av.to_string(),
                b: bv.to_string(),
            });
        }
    }

    if a.is_none() {
        if b.is_none() {
            return Ok(FRACTIONAL_INTEGER_ZERO.to_string());
        }
        let b = b.unwrap();
        let ib = _get_integer_part(b)?;
        let fb = &b[ib.len()..];
        if ib == FRACTIONAL_INTEGER_MIN {
            return Ok(ib + &_midpoint("", Some(fb))?);
        }
        if ib.as_str() < b {
            return Ok(ib);
        } else {
            return Ok(decrement_integer(&ib).unwrap());
        }
    }
    if b.is_none() {
        let a = a.unwrap();
        let ia = _get_integer_part(a)?;
        let fa = &a[ia.len()..];
        if let Some(i) = increment_integer(&ia) {
            Ok(i)
        } else {
            Ok(ia + &_midpoint(fa, None)?)
        }
    } else {
        let a = a.unwrap();
        let b = b.unwrap();
        let ia = _get_integer_part(a)?;
        let fa = &a[ia.len()..];
        let ib = _get_integer_part(b)?;
        let fb = &b[ib.len()..];
        if ia == ib {
            Ok(ia + &_midpoint(fa, Some(fb))?)
        } else {
            let i = increment_integer(&ia).unwrap();
            if i.as_str() < b {
                Ok(i)
            } else {
                Ok(ia + &_midpoint(fa, None)?)
            }
        }
    }
}

/// Generates evenly spread `n` keys between the given keys `a` and `b` (inclusive)
pub fn get_order_keys(
    a: Option<&str>,
    b: Option<&str>,
    n: u32,
) -> Result<Vec<String>, FractionalError> {
    if n == 0 {
        return Ok(vec![]);
    }
    if n == 1 {
        return Ok(vec![get_order_key(a, b)?]);
    }
    if b.is_none() {
        let mut c = get_order_key(a, b)?;
        let mut result = vec![c.clone()];
        for _ in 0..(n - 1) {
            c = get_order_key(Some(&c), b)?;
            result.push(c.clone());
        }
        return Ok(result);
    }
    if a.is_none() {
        let mut c = get_order_key(a, b)?;
        let mut result = vec![c.clone()];
        for _ in 0..(n - 1) {
            c = get_order_key(a, Some(&c))?;
            result.push(c.clone());
        }
        result.reverse();
        return Ok(result);
    }
    let mid = n / 2;
    let c = get_order_key(a, b)?;
    let mut left = get_order_keys(a, Some(&c), mid)?;
    let right = get_order_keys(Some(&c), b, n - mid - 1)?;
    let mut res = Vec::with_capacity(left.len() + 1 + right.len());
    res.append(&mut left);
    res.push(c);
    res.extend(right);
    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_ok(a: Option<&str>, b: Option<&str>, expected: &str) {
        let result = get_order_key(a, b);
        match result {
            Ok(actual) => assert_eq!(actual, expected, "case ({:?}, {:?})", a, b),
            Err(e) => panic!(
                "expected Ok({}), got error {:?} for case ({:?}, {:?})",
                expected, e, a, b
            ),
        }
    }

    fn assert_err(a: Option<&str>, b: Option<&str>, expected_err: &str) {
        let result = get_order_key(a, b);
        match expected_err {
            "InvalidOrderKey" => assert!(
                matches!(result, Err(FractionalError::InvalidOrderKey { .. })),
                "expected InvalidOrderKey for case ({:?}, {:?}), got {:?}",
                a,
                b,
                result
            ),
            "InvalidComparison" => assert!(
                matches!(result, Err(FractionalError::InvalidComparison { .. })),
                "expected InvalidComparison for case ({:?}, {:?}), got {:?}",
                a,
                b,
                result
            ),
            other => panic!("unknown expected error kind: {}", other),
        }
    }

    macro_rules! fractional_ok_case {
        ($name:ident, $a:expr, $b:expr, $expected:expr) => {
            #[test]
            fn $name() {
                assert_ok($a, $b, $expected);
            }
        };
    }

    macro_rules! fractional_err_case {
        ($name:ident, $a:expr, $b:expr, $err:expr) => {
            #[test]
            fn $name() {
                assert_err($a, $b, $err);
            }
        };
    }

    fractional_ok_case!(case_none_none_is_a0, None, None, "a0");
    fractional_ok_case!(case_none_a0, None, Some("a0"), "a/");
    fractional_ok_case!(case_a0_none, Some("a0"), None, "a1");
    fractional_ok_case!(case_a0_a1, Some("a0"), Some("a1"), "a0P");
    fractional_ok_case!(case_a0v_a1, Some("a0V"), Some("a1"), "a0k");
    fractional_ok_case!(case_zz_a0, Some("Zz"), Some("a0"), "Z{");
    fractional_ok_case!(case_zz_a1, Some("Zz"), Some("a1"), "Z{");
    fractional_ok_case!(case_none_y00, None, Some("Y00"), "Y//");
    fractional_ok_case!(case_bzz_none, Some("bzz"), None, "bz{");
    fractional_ok_case!(case_a0_a0v, Some("a0"), Some("a0V"), "a0<");
    fractional_ok_case!(case_a0_a0g, Some("a0"), Some("a0G"), "a05");
    fractional_ok_case!(case_b125_b129, Some("b125"), Some("b129"), "b127");
    fractional_ok_case!(case_a0_a1v, Some("a0"), Some("a1V"), "a1");
    fractional_ok_case!(case_zz_a01, Some("Zz"), Some("a01"), "Z{");
    fractional_ok_case!(case_none_a0v, None, Some("a0V"), "a0");
    fractional_ok_case!(case_none_b999, None, Some("b999"), "b99");
    fractional_ok_case!(
        case_none_a_min_plus_one,
        None,
        Some("A000000000000000000000000001"),
        "A00000000000000000000000000*"
    );
    fractional_ok_case!(
        case_zs_many_y_none,
        Some("zzzzzzzzzzzzzzzzzzzzzzzzzzy"),
        None,
        "zzzzzzzzzzzzzzzzzzzzzzzzzzz"
    );
    fractional_ok_case!(
        case_zs_many_none,
        Some("zzzzzzzzzzzzzzzzzzzzzzzzzzz"),
        None,
        "zzzzzzzzzzzzzzzzzzzzzzzzzz{"
    );

    fractional_err_case!(
        case_none_a_min,
        None,
        Some("A00000000000000000000000000"),
        "InvalidOrderKey"
    );
    fractional_err_case!(case_a00_none, Some("a00"), None, "InvalidOrderKey");
    fractional_err_case!(case_a00_a1, Some("a00"), Some("a1"), "InvalidOrderKey");
    fractional_err_case!(case_zero_one, Some("0"), Some("1"), "InvalidOrderKey");
    fractional_err_case!(case_a1_a0, Some("a1"), Some("a0"), "InvalidComparison");
}
