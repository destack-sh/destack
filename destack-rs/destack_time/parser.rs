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
        }
    }
}

#[inline]
/// Parse two successive digits from a byte slice into a single number.
fn parse_two_digits(dt_bytes: &[u8]) -> Result<i64, ParseError> {
    if dt_bytes.len() < 2 || !dt_bytes[0].is_ascii_digit() || !dt_bytes[1].is_ascii_digit() {
        return Err(ParseError::InvalidNumber);
    }
    let first_digit = (dt_bytes[0] - b'0') as i64;
    let second_digit = (dt_bytes[1] - b'0') as i64;
    let number = 10 * first_digit + second_digit;
    Ok(number)
}

#[inline]
fn parse_hh_mm_ss(dt_bytes: &[u8]) -> Result<(i64, i64, i64), ParseError> {
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
