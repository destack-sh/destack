use crate::JsonValue;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsonParseError {
    Eof,
    TrailingCharacters,
    InvalidValue,
    InvalidNumber,
    InvalidString,
    InvalidEscape,
    InvalidUnicodeEscape,
    ExpectedColon,
    ExpectedCommaOrEnd,
    ExpectedKey,
}

impl fmt::Display for JsonParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JsonParseError::Eof => f.write_str("unexpected end of input"),
            JsonParseError::TrailingCharacters => {
                f.write_str("trailing characters after JSON value")
            }
            JsonParseError::InvalidValue => f.write_str("invalid JSON value"),
            JsonParseError::InvalidNumber => f.write_str("invalid number"),
            JsonParseError::InvalidString => f.write_str("invalid string"),
            JsonParseError::InvalidEscape => f.write_str("invalid escape sequence"),
            JsonParseError::InvalidUnicodeEscape => f.write_str("invalid unicode escape"),
            JsonParseError::ExpectedColon => f.write_str("expected ':'"),
            JsonParseError::ExpectedCommaOrEnd => f.write_str("expected ',' or end"),
            JsonParseError::ExpectedKey => f.write_str("expected object key string"),
        }
    }
}

#[inline]
pub fn parse_json(s: &str) -> Result<JsonValue, JsonParseError> {
    let mut p = Parser {
        s: s.as_bytes(),
        i: 0,
    };
    p.skip_ws();
    let v = p.parse_value()?;
    p.skip_ws();
    if p.i != p.s.len() {
        return Err(JsonParseError::TrailingCharacters);
    }
    Ok(v)
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl<'a> Parser<'a> {
    #[inline]
    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }

    #[inline]
    fn bump(&mut self) -> Option<u8> {
        let b = self.s.get(self.i).copied();
        if b.is_some() {
            self.i += 1;
        }
        b
    }

    #[inline]
    fn expect_byte(&mut self, b: u8) -> Result<(), JsonParseError> {
        match self.bump() {
            Some(x) if x == b => Ok(()),
            _ => Err(JsonParseError::InvalidValue),
        }
    }

    #[inline]
    fn skip_ws(&mut self) {
        while let Some(&b) = self.s.get(self.i) {
            match b {
                b' ' | b'\n' | b'\r' | b'\t' => self.i += 1,
                _ => break,
            }
        }
    }

    #[inline]
    fn parse_value(&mut self) -> Result<JsonValue, JsonParseError> {
        match self.peek().ok_or(JsonParseError::Eof)? {
            b'n' => self.parse_null(),
            b't' => self.parse_true(),
            b'f' => self.parse_false(),
            b'"' => self.parse_string().map(JsonValue::String),
            b'-' | b'0'..=b'9' => self.parse_number().map(JsonValue::Number),
            b'[' => self.parse_array(),
            b'{' => self.parse_object(),
            _ => Err(JsonParseError::InvalidValue),
        }
    }

    #[inline]
    fn parse_null(&mut self) -> Result<JsonValue, JsonParseError> {
        if self.s.get(self.i..self.i + 4) == Some(b"null") {
            self.i += 4;
            Ok(JsonValue::Null)
        } else {
            Err(JsonParseError::InvalidValue)
        }
    }

    #[inline]
    fn parse_true(&mut self) -> Result<JsonValue, JsonParseError> {
        if self.s.get(self.i..self.i + 4) == Some(b"true") {
            self.i += 4;
            Ok(JsonValue::Bool(true))
        } else {
            Err(JsonParseError::InvalidValue)
        }
    }

    #[inline]
    fn parse_false(&mut self) -> Result<JsonValue, JsonParseError> {
        if self.s.get(self.i..self.i + 5) == Some(b"false") {
            self.i += 5;
            Ok(JsonValue::Bool(false))
        } else {
            Err(JsonParseError::InvalidValue)
        }
    }

    #[inline]
    fn parse_number(&mut self) -> Result<f64, JsonParseError> {
        let start = self.i;
        if self.peek() == Some(b'-') {
            self.i += 1;
        }
        match self.peek() {
            Some(b'0') => {
                self.i += 1;
            }
            Some(b'1'..=b'9') => {
                self.i += 1;
                while let Some(b'0'..=b'9') = self.peek() {
                    self.i += 1;
                }
            }
            _ => return Err(JsonParseError::InvalidNumber),
        }
        if self.peek() == Some(b'.') {
            self.i += 1;
            let mut had_digit = false;
            while let Some(b'0'..=b'9') = self.peek() {
                self.i += 1;
                had_digit = true;
            }
            if !had_digit {
                return Err(JsonParseError::InvalidNumber);
            }
        }
        if let Some(b'e') | Some(b'E') = self.peek().map(|b| b.to_ascii_lowercase()) {
            self.i += 1;
            if let Some(b'+') | Some(b'-') = self.peek() {
                self.i += 1;
            }
            let mut had_digit = false;
            while let Some(b'0'..=b'9') = self.peek() {
                self.i += 1;
                had_digit = true;
            }
            if !had_digit {
                return Err(JsonParseError::InvalidNumber);
            }
        }
        let end = self.i;
        // SAFETY: slice is within original &str, utf8 boundary is fine since it's ascii digits/sign/dot
        let s = unsafe { std::str::from_utf8_unchecked(&self.s[start..end]) };
        s.parse::<f64>().map_err(|_| JsonParseError::InvalidNumber)
    }

    #[inline]
    fn parse_string(&mut self) -> Result<String, JsonParseError> {
        self.expect_byte(b'"')?;
        let mut out = String::new();
        let mut start = self.i;
        while let Some(b) = self.peek() {
            if b == b'"' {
                // fast path: push slice
                if start < self.i {
                    out.push_str(unsafe { std::str::from_utf8_unchecked(&self.s[start..self.i]) });
                }
                self.i += 1; // consume closing quote
                return Ok(out);
            }
            if b == b'\\' {
                // write preceding chunk
                if start < self.i {
                    out.push_str(unsafe { std::str::from_utf8_unchecked(&self.s[start..self.i]) });
                }
                self.i += 1; // consume backslash
                let esc = self.bump().ok_or(JsonParseError::InvalidEscape)?;
                match esc {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'/' => out.push('/'),
                    b'b' => out.push('\u{0008}'),
                    b'f' => out.push('\u{000C}'),
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    b'u' => {
                        let cp = self.parse_u4()?;
                        let ch = if (0xD800..=0xDBFF).contains(&cp) {
                            // high surrogate, expect low surrogate
                            if self.bump() != Some(b'\\') || self.bump() != Some(b'u') {
                                return Err(JsonParseError::InvalidUnicodeEscape);
                            }
                            let low = self.parse_u4()?;
                            if !(0xDC00..=0xDFFF).contains(&low) {
                                return Err(JsonParseError::InvalidUnicodeEscape);
                            }
                            let high_ten = cp as u32 - 0xD800;
                            let low_ten = low as u32 - 0xDC00;
                            let scalar = 0x10000 + ((high_ten << 10) | low_ten);
                            char::from_u32(scalar).ok_or(JsonParseError::InvalidUnicodeEscape)?
                        } else {
                            char::from_u32(cp as u32).ok_or(JsonParseError::InvalidUnicodeEscape)?
                        };
                        out.push(ch);
                    }
                    _ => return Err(JsonParseError::InvalidEscape),
                }
                start = self.i;
            } else if b < 0x20 {
                // control chars not allowed
                return Err(JsonParseError::InvalidString);
            } else {
                self.i += 1;
            }
        }
        Err(JsonParseError::Eof)
    }

    #[inline]
    fn parse_u4(&mut self) -> Result<u16, JsonParseError> {
        let mut val: u16 = 0;
        for _ in 0..4 {
            let b = self.bump().ok_or(JsonParseError::Eof)?;
            val <<= 4;
            val |= hex_val(b).ok_or(JsonParseError::InvalidUnicodeEscape)? as u16;
        }
        Ok(val)
    }

    #[inline]
    fn parse_array(&mut self) -> Result<JsonValue, JsonParseError> {
        self.expect_byte(b'[')?;
        self.skip_ws();
        let mut arr = Vec::new();
        if self.peek() == Some(b']') {
            self.i += 1;
            return Ok(JsonValue::Array(arr));
        }
        loop {
            self.skip_ws();
            let v = self.parse_value()?;
            arr.push(v);
            self.skip_ws();
            match self.bump() {
                Some(b',') => {
                    self.skip_ws();
                }
                Some(b']') => break,
                _ => return Err(JsonParseError::ExpectedCommaOrEnd),
            }
        }
        Ok(JsonValue::Array(arr))
    }

    #[inline]
    fn parse_object(&mut self) -> Result<JsonValue, JsonParseError> {
        self.expect_byte(b'{')?;
        self.skip_ws();
        let mut map: HashMap<String, JsonValue> = HashMap::new();
        if self.peek() == Some(b'}') {
            self.i += 1;
            return Ok(JsonValue::Object(map));
        }
        loop {
            self.skip_ws();
            if self.peek() != Some(b'"') {
                return Err(JsonParseError::ExpectedKey);
            }
            let key = self.parse_string()?;
            self.skip_ws();
            if self.bump() != Some(b':') {
                return Err(JsonParseError::ExpectedColon);
            }
            self.skip_ws();
            let val = self.parse_value()?;
            map.insert(key, val);
            self.skip_ws();
            match self.bump() {
                Some(b',') => {
                    self.skip_ws();
                }
                Some(b'}') => break,
                _ => return Err(JsonParseError::ExpectedCommaOrEnd),
            }
        }
        Ok(JsonValue::Object(map))
    }
}

#[inline]
fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(10 + (b - b'a')),
        b'A'..=b'F' => Some(10 + (b - b'A')),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_scalars() {
        assert_eq!(parse_json("null").unwrap(), JsonValue::Null);
        assert_eq!(parse_json("true").unwrap(), JsonValue::Bool(true));
        assert_eq!(parse_json("false").unwrap(), JsonValue::Bool(false));
        assert_eq!(parse_json("123").unwrap(), JsonValue::Number(123.0));
        assert_eq!(parse_json("-0.5").unwrap(), JsonValue::Number(-0.5));
        assert_eq!(
            parse_json("\"hi\\n\\t\\u0041\"").unwrap(),
            JsonValue::String("hi\n\tA".into())
        );
    }

    #[test]
    fn parse_arrays_objects() {
        let v = parse_json("[1, 2, 3]").unwrap();
        match v {
            JsonValue::Array(a) => assert_eq!(a.len(), 3),
            _ => panic!(),
        }
        let o = parse_json("{\"a\":1,\"b\":[true,false]} ").unwrap();
        match o {
            JsonValue::Object(m) => {
                assert!(m.contains_key("a"));
                assert!(m.contains_key("b"));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn errors() {
        assert!(matches!(
            parse_json("[1 2]"),
            Err(JsonParseError::ExpectedCommaOrEnd)
        ));
        assert!(matches!(
            parse_json("{\"a\" 1}"),
            Err(JsonParseError::ExpectedColon)
        ));
        assert!(matches!(parse_json("\"bad"), Err(JsonParseError::Eof)));
    }
}
