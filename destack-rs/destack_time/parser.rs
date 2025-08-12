use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    TooShort,
    MissingT,
    MissingTimezone,
    InvalidNumber,
    InvalidTime,
    InvalidDate(crate::date::DateParseError),
    InvalidOffset,
    TimeOutOfRange,
    FractionDigitsMustBe6,
    FractionDigitsMustBe9,
    BeforeEpoch,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::TooShort => f.write_str("too short"),
            ParseError::MissingT => f.write_str("missing 'T'"),
            ParseError::MissingTimezone => f.write_str("missing timezone"),
            ParseError::InvalidNumber => f.write_str("invalid number"),
            ParseError::InvalidTime => f.write_str("invalid time"),
            ParseError::InvalidDate(_) => f.write_str("invalid date"),
            ParseError::InvalidOffset => f.write_str("invalid offset"),
            ParseError::TimeOutOfRange => f.write_str("time out of range"),
            ParseError::FractionDigitsMustBe6 => f.write_str("microseconds must be 6 digits"),
            ParseError::FractionDigitsMustBe9 => f.write_str("nanoseconds must be 9 digits"),
            ParseError::BeforeEpoch => f.write_str("before epoch"),
        }
    }
}

#[inline]
/// Parse two successive digits from a byte slice into a single number.
pub(crate) fn parse_two_digits(dt_bytes: &[u8]) -> Result<i64, ParseError> {
    if dt_bytes.len() < 2 || !dt_bytes[0].is_ascii_digit() || !dt_bytes[1].is_ascii_digit() {
        return Err(ParseError::InvalidNumber);
    }
    let first_digit = (dt_bytes[0] - b'0') as i64;
    let second_digit = (dt_bytes[1] - b'0') as i64;
    let number = 10 * first_digit + second_digit;
    Ok(number)
}

#[inline]
/// Parse the `HH:MM:SS` part of a datetime string.
pub(crate) fn parse_hh_mm_ss(dt_bytes: &[u8]) -> Result<(i64, i64, i64), ParseError> {
    if dt_bytes.len() < 8 {
        return Err(ParseError::InvalidTime);
    }
    // hh
    let hh = parse_two_digits(dt_bytes)?;
    if dt_bytes.get(2) != Some(&b':') {
        return Err(ParseError::InvalidTime);
    }
    // mm
    let mm = parse_two_digits(&dt_bytes[3..])?;
    if dt_bytes.get(5) != Some(&b':') {
        return Err(ParseError::InvalidTime);
    }
    // ss
    let ss = parse_two_digits(&dt_bytes[6..])?;
    if hh >= 24 || mm >= 60 || ss >= 60 {
        return Err(ParseError::InvalidTime);
    }
    Ok((hh, mm, ss))
}

#[inline]
/// Parse the `ffffff` microseconds part given exactly 6 digits (no leading dot)
pub(crate) fn parse_us_digits_exact(dt_bytes: &[u8]) -> Result<i64, ParseError> {
    if dt_bytes.len() != 6 {
        return Err(ParseError::FractionDigitsMustBe6);
    }
    let mut us: i64 = 0;
    for &b in dt_bytes {
        if !b.is_ascii_digit() {
            return Err(ParseError::InvalidTime);
        }
        us = us * 10 + (b - b'0') as i64;
    }
    Ok(us)
}

#[inline]
/// Try to parse the `.ffffff` part of a time string
/// Return Ok(None) when there is no fractional part.
pub(crate) fn parse_us_maybe(dt_bytes: &[u8]) -> Result<Option<i64>, ParseError> {
    if dt_bytes.is_empty() {
        return Ok(None);
    }
    if dt_bytes[0] != b'.' {
        return Err(ParseError::InvalidTime);
    }
    let us = parse_us_digits_exact(&dt_bytes[1..])?;
    Ok(Some(us))
}

#[inline]
/// Parse the `nnnnnnnnn` nanoseconds part given exactly 9 digits (no leading dot)
pub(crate) fn parse_ns_digits_exact(dt_bytes: &[u8]) -> Result<u64, ParseError> {
    if dt_bytes.len() != 9 {
        return Err(ParseError::FractionDigitsMustBe9);
    }
    let mut ns: u64 = 0;
    for &b in dt_bytes {
        if !b.is_ascii_digit() {
            return Err(ParseError::InvalidTime);
        }
        ns = ns * 10 + (b - b'0') as u64;
    }
    Ok(ns)
}

#[inline]
/// Try to parse the `.nnnnnnnnn` part of a time string
/// Return Ok(None) when there is no fractional part.
pub(crate) fn parse_ns9_maybe(dt_bytes: &[u8]) -> Result<Option<u64>, ParseError> {
    if dt_bytes.is_empty() {
        return Ok(None);
    }
    if dt_bytes[0] != b'.' {
        return Err(ParseError::InvalidTime);
    }
    let ns = parse_ns_digits_exact(&dt_bytes[1..])?;
    Ok(Some(ns))
}

#[inline]
/// Split the time and timezone parts of a datetime/timestamp string.
pub(crate) fn split_time_and_tz(dt_str: &str) -> Result<(&str, &str), ParseError> {
    if let Some(zpos) = dt_str.rfind('Z') {
        Ok((&dt_str[..zpos], &dt_str[zpos..]))
    } else if let Some(pos) = dt_str.rfind(|c| c == '+' || c == '-') {
        Ok((&dt_str[..pos], &dt_str[pos..]))
    } else {
        Err(ParseError::MissingTimezone)
    }
}

#[inline]
/// Parse timezone offset of the form `Z` or `±HH:MM`, returning nanoseconds.
pub(crate) fn parse_tz_offset(tz_part: &str) -> Result<i128, ParseError> {
    if tz_part == "Z" {
        return Ok(0);
    }
    let tz_bytes = tz_part.as_bytes();
    if tz_bytes.len() != 6 || (tz_bytes[0] != b'+' && tz_bytes[0] != b'-') || tz_bytes[3] != b':' {
        return Err(ParseError::InvalidOffset);
    }
    let sign = if tz_bytes[0] == b'-' { -1i128 } else { 1 };
    let hh = ((tz_bytes[1] - b'0') as i128) * 10 + (tz_bytes[2] - b'0') as i128;
    let mm = ((tz_bytes[4] - b'0') as i128) * 10 + (tz_bytes[5] - b'0') as i128;
    Ok(sign * (hh * 3_600 + mm * 60) * 1_000_000_000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frac() {
        assert_eq!(parse_us_maybe(b"").unwrap(), None);
        assert_eq!(parse_us_maybe(b".123456").unwrap(), Some(123_456));
        assert!(matches!(
            parse_us_maybe(b".12345"),
            Err(ParseError::FractionDigitsMustBe6)
        ));
        assert_eq!(parse_ns9_maybe(b"").unwrap(), None);
        assert_eq!(parse_ns9_maybe(b".123456789").unwrap(), Some(123_456_789));
        assert!(matches!(
            parse_ns9_maybe(b".12345678"),
            Err(ParseError::FractionDigitsMustBe9)
        ));
    }
}
