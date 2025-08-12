use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeParseError {
    TooShort,
    MissingT,
    MissingTimezone,
    InvalidFormat,
    InvalidNumber,
    InvalidTime,
    InvalidDate,
    InvalidOffset,
    TimeOutOfRange,
    FractionDigitsMustBe6,
    FractionDigitsMustBe9,
    BeforeEpoch,
    Overflow,
}

impl fmt::Display for TimeParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimeParseError::TooShort => f.write_str("too short"),
            TimeParseError::MissingT => f.write_str("missing 'T'"),
            TimeParseError::MissingTimezone => f.write_str("missing timezone"),
            TimeParseError::InvalidFormat => f.write_str("invalid format"),
            TimeParseError::InvalidNumber => f.write_str("invalid number"),
            TimeParseError::InvalidTime => f.write_str("invalid time"),
            TimeParseError::InvalidDate => f.write_str("invalid date"),
            TimeParseError::InvalidOffset => f.write_str("invalid offset"),
            TimeParseError::TimeOutOfRange => f.write_str("time out of range"),
            TimeParseError::FractionDigitsMustBe6 => f.write_str("microseconds must be 6 digits"),
            TimeParseError::FractionDigitsMustBe9 => f.write_str("nanoseconds must be 9 digits"),
            TimeParseError::BeforeEpoch => f.write_str("before epoch"),
            TimeParseError::Overflow => f.write_str("overflow"),
        }
    }
}

#[inline]
/// Parse two successive digits from a byte slice into a single number.
pub(crate) fn parse_two_digits(dt_bytes: &[u8]) -> Result<i64, TimeParseError> {
    if dt_bytes.len() < 2 || !dt_bytes[0].is_ascii_digit() || !dt_bytes[1].is_ascii_digit() {
        return Err(TimeParseError::InvalidNumber);
    }
    let first_digit = (dt_bytes[0] - b'0') as i64;
    let second_digit = (dt_bytes[1] - b'0') as i64;
    let number = 10 * first_digit + second_digit;
    Ok(number)
}

#[inline]
/// Parse the `HH:MM:SS` part of a datetime string.
pub(crate) fn parse_hh_mm_ss(dt_bytes: &[u8]) -> Result<(i64, i64, i64), TimeParseError> {
    if dt_bytes.len() < 8 {
        return Err(TimeParseError::InvalidTime);
    }
    // hh
    let hh = parse_two_digits(dt_bytes)?;
    if dt_bytes.get(2) != Some(&b':') {
        return Err(TimeParseError::InvalidTime);
    }
    // mm
    let mm = parse_two_digits(&dt_bytes[3..])?;
    if dt_bytes.get(5) != Some(&b':') {
        return Err(TimeParseError::InvalidTime);
    }
    // ss
    let ss = parse_two_digits(&dt_bytes[6..])?;
    if hh >= 24 || mm >= 60 || ss >= 60 {
        return Err(TimeParseError::InvalidTime);
    }
    Ok((hh, mm, ss))
}

#[inline]
/// Parse the `ffffff` microseconds part given exactly 6 digits (no leading dot)
pub(crate) fn parse_us_digits_exact(dt_bytes: &[u8]) -> Result<i64, TimeParseError> {
    if dt_bytes.len() != 6 {
        return Err(TimeParseError::FractionDigitsMustBe6);
    }
    let mut us: i64 = 0;
    for &b in dt_bytes {
        if !b.is_ascii_digit() {
            return Err(TimeParseError::InvalidTime);
        }
        us = us * 10 + (b - b'0') as i64;
    }
    Ok(us)
}

#[inline]
/// Try to parse the `.ffffff` part of a time string
/// Return Ok(None) when there is no fractional part.
pub(crate) fn parse_us_maybe(dt_bytes: &[u8]) -> Result<Option<i64>, TimeParseError> {
    if dt_bytes.is_empty() {
        return Ok(None);
    }
    if dt_bytes[0] != b'.' {
        return Err(TimeParseError::InvalidTime);
    }
    let us = parse_us_digits_exact(&dt_bytes[1..])?;
    Ok(Some(us))
}

#[inline]
/// Parse the `nnnnnnnnn` nanoseconds part given exactly 9 digits (no leading dot)
pub(crate) fn parse_ns_digits_exact(dt_bytes: &[u8]) -> Result<u64, TimeParseError> {
    if dt_bytes.len() != 9 {
        return Err(TimeParseError::FractionDigitsMustBe9);
    }
    let mut ns: u64 = 0;
    for &b in dt_bytes {
        if !b.is_ascii_digit() {
            return Err(TimeParseError::InvalidTime);
        }
        ns = ns * 10 + (b - b'0') as u64;
    }
    Ok(ns)
}

#[inline]
/// Try to parse the `.nnnnnnnnn` part of a time string
/// Return Ok(None) when there is no fractional part.
pub(crate) fn parse_ns9_maybe(dt_bytes: &[u8]) -> Result<Option<u64>, TimeParseError> {
    if dt_bytes.is_empty() {
        return Ok(None);
    }
    if dt_bytes[0] != b'.' {
        return Err(TimeParseError::InvalidTime);
    }
    let ns = parse_ns_digits_exact(&dt_bytes[1..])?;
    Ok(Some(ns))
}

#[inline]
/// Split the time and timezone parts of a datetime/timestamp string.
pub(crate) fn split_time_and_tz(dt_str: &str) -> Result<(&str, &str), TimeParseError> {
    if let Some(zpos) = dt_str.rfind('Z') {
        Ok((&dt_str[..zpos], &dt_str[zpos..]))
    } else if let Some(pos) = dt_str.rfind(['+', '-']) {
        Ok((&dt_str[..pos], &dt_str[pos..]))
    } else {
        Err(TimeParseError::MissingTimezone)
    }
}

#[inline]
/// Parse timezone offset of the form `Z` or `±HH:MM`, returning nanoseconds.
pub(crate) fn parse_tz_offset(tz_part: &str) -> Result<i128, TimeParseError> {
    if tz_part == "Z" {
        return Ok(0);
    }
    let tz_bytes = tz_part.as_bytes();
    if tz_bytes.len() != 6 || (tz_bytes[0] != b'+' && tz_bytes[0] != b'-') || tz_bytes[3] != b':' {
        return Err(TimeParseError::InvalidOffset);
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
            Err(TimeParseError::FractionDigitsMustBe6)
        ));
        assert_eq!(parse_ns9_maybe(b"").unwrap(), None);
        assert_eq!(parse_ns9_maybe(b".123456789").unwrap(), Some(123_456_789));
        assert!(matches!(
            parse_ns9_maybe(b".12345678"),
            Err(TimeParseError::FractionDigitsMustBe9)
        ));
    }
}
