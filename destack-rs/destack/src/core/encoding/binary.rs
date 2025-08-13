//! Binary encoding/decoding for primitive values.
//!
//! The format is wire-compatible with `destack/core/encoding/binary.py`.
//! Types covered:
//! - booleans: 1 byte (0/1)
//! - integers: little-endian, sizes 8/16/32/64/128
//! - floats: IEEE-754 16/32/64-bit, little-endian
//! - datetime: i64 microseconds since Unix epoch (UTC)
//! - date: i64 days since Unix epoch (UTC)
//! - time: u64 nanoseconds since midnight
//! - timestamp: u64 nanoseconds since Unix epoch (UTC)
//! - duration: i64 nanoseconds
//! - string/character: u32 length prefix + UTF-8 bytes
//! - bytes: u32 length prefix + raw bytes
//! - json: tagged format matching Python encoder

use std::fmt;

use destack_time::{Date, DateTime, Duration, Time, Timestamp};
use destack_uuid::Uuid;

#[derive(Debug, Clone)]
/// Base error for binary encoding/decoding.
pub struct BinaryError(pub String);

impl fmt::Display for BinaryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for BinaryError {}

impl BinaryError {
    fn eof(pos: usize) -> Self {
        BinaryError(format!("unexpected end of buffer at {pos}"))
    }
}

#[derive(Default, Debug)]
/// Write binary primitive values in the same encoding as Python.
pub struct BinaryEncoder {
    buffer: Vec<u8>,
}

impl BinaryEncoder {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn to_bytes(&self) -> &[u8] {
        &self.buffer
    }

    // PrimitiveType.BOOLEAN
    /// Write a boolean.
    pub fn write_bool(&mut self, value: bool) {
        self.buffer.push(if value { 1 } else { 0 });
    }

    // PrimitiveType.INT8
    /// Write a signed 8-bit integer.
    pub fn write_int8(&mut self, value: i8) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }

    // PrimitiveType.INT16
    /// Write a signed 16-bit integer.
    pub fn write_int16(&mut self, value: i16) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }

    // PrimitiveType.INT32
    /// Write a signed 32-bit integer.
    pub fn write_int32(&mut self, value: i32) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }

    // PrimitiveType.INT64
    /// Write a signed 64-bit integer.
    pub fn write_int64(&mut self, value: i64) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }

    // PrimitiveType.INT128
    /// Write a signed 128-bit integer by splitting into two u64 little-endian parts.
    pub fn write_int128(&mut self, value: i128) {
        let low = value as u128 as u64;
        let high = ((value as u128) >> 64) as u64;
        self.buffer.extend_from_slice(&low.to_le_bytes());
        self.buffer.extend_from_slice(&high.to_le_bytes());
    }

    // PrimitiveType.UINT8
    /// Write an unsigned 8-bit integer.
    pub fn write_uint8(&mut self, value: u8) {
        self.buffer.push(value);
    }

    // PrimitiveType.UINT16
    /// Write an unsigned 16-bit integer.
    pub fn write_uint16(&mut self, value: u16) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }

    // PrimitiveType.UINT32
    /// Write an unsigned 32-bit integer.
    pub fn write_uint32(&mut self, value: u32) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }

    // PrimitiveType.UINT64
    /// Write an unsigned 64-bit integer.
    pub fn write_uint64(&mut self, value: u64) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }

    // PrimitiveType.UINT128
    /// Write an unsigned 128-bit integer by splitting into two u64 little-endian parts.
    pub fn write_uint128(&mut self, value: u128) {
        let low = value as u64;
        let high = (value >> 64) as u64;
        self.buffer.extend_from_slice(&low.to_le_bytes());
        self.buffer.extend_from_slice(&high.to_le_bytes());
    }

    // PrimitiveType.FLOAT32
    /// Write a 32-bit float.
    pub fn write_float32(&mut self, value: f32) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }

    // PrimitiveType.FLOAT64
    /// Write a 64-bit float.
    pub fn write_float64(&mut self, value: f64) {
        self.buffer.extend_from_slice(&value.to_le_bytes());
    }

    // PrimitiveType.DATETIME
    /// Write a datetime.
    pub fn write_datetime(&mut self, value: DateTime) {
        // DateTime is i64 microseconds since epoch
        self.buffer
            .extend_from_slice(&i64::from(value).to_le_bytes());
    }

    // PrimitiveType.DATE
    /// Write a date.
    pub fn write_date(&mut self, value: Date) {
        self.buffer
            .extend_from_slice(&i64::from(value).to_le_bytes());
    }

    // PrimitiveType.TIME
    /// Write a time.
    pub fn write_time(&mut self, value: Time) {
        self.buffer
            .extend_from_slice(&u64::from(value).to_le_bytes());
    }

    // PrimitiveType.TIMESTAMP
    /// Write a timestamp.
    pub fn write_timestamp(&mut self, value: Timestamp) {
        self.buffer
            .extend_from_slice(&u64::from(value).to_le_bytes());
    }

    // PrimitiveType.DURATION
    /// Write a duration.
    pub fn write_duration(&mut self, value: Duration) {
        self.buffer
            .extend_from_slice(&i64::from(value).to_le_bytes());
    }

    // PrimitiveType.STRING
    /// Write a UTF-8 string.
    pub fn write_string(&mut self, value: &str) {
        let bytes = value.as_bytes();
        let len = bytes.len() as u32;
        self.buffer.extend_from_slice(&len.to_le_bytes());
        self.buffer.extend_from_slice(bytes);
    }

    // PrimitiveType.CHARACTER
    /// Write a single Unicode character as UTF-8 with length prefix.
    pub fn write_character(&mut self, value: char) {
        let mut buf = [0u8; 4];
        let s = value.encode_utf8(&mut buf);
        self.write_string(s);
    }

    // PrimitiveType.UUID
    /// Write a UUID as 16 raw bytes in big-endian byte order (matches Python UUID.bytes).
    pub fn write_uuid(&mut self, value: Uuid) {
        let be = value.0.to_be_bytes();
        self.buffer.extend_from_slice(&be);
    }

    // PrimitiveType.BYTES
    /// Write raw bytes with a u32 length prefix.
    pub fn write_bytes(&mut self, value: &[u8]) {
        let len = value.len() as u32;
        self.buffer.extend_from_slice(&len.to_le_bytes());
        self.buffer.extend_from_slice(value);
    }

    // PrimitiveType.JSON
    /// Write JSON in the tagged format used by Python.
    pub fn write_json(&mut self, value: &serde_json::Value) -> Result<(), BinaryError> {
        use serde_json::Value as J;
        match value {
            J::Null => self.buffer.push(0),
            J::Bool(false) => self.buffer.push(1),
            J::Bool(true) => self.buffer.push(2),
            J::Number(n) => {
                if let Some(i) = n.as_i64() {
                    self.buffer.push(3);
                    self.write_int64(i);
                } else if let Some(f) = n.as_f64() {
                    self.buffer.push(4);
                    self.write_float64(f);
                } else {
                    return Err(BinaryError("unsupported JSON number".to_string()));
                }
            }
            J::String(s) => {
                self.buffer.push(5);
                self.write_string(s);
            }
            J::Array(arr) => {
                self.buffer.push(6);
                let len = arr.len() as u32;
                self.buffer.extend_from_slice(&len.to_le_bytes());
                for item in arr {
                    self.write_json(item)?;
                }
            }
            J::Object(map) => {
                self.buffer.push(7);
                let len = map.len() as u32;
                self.buffer.extend_from_slice(&len.to_le_bytes());
                for (k, v) in map {
                    self.write_string(k);
                    self.write_json(v)?;
                }
            }
        }
        Ok(())
    }
}

/// Read binary primitive values in the same encoding as Python.
#[derive(Debug)]
pub struct BinaryDecoder<'a> {
    buffer: &'a [u8],
    pos: usize,
}

impl<'a> BinaryDecoder<'a> {
    pub fn new(buffer: &'a [u8]) -> Self {
        Self { buffer, pos: 0 }
    }

    pub fn remaining(&self) -> usize {
        self.buffer.len().saturating_sub(self.pos)
    }

    #[inline]
    fn take(&mut self, n: usize) -> Result<&'a [u8], BinaryError> {
        if self.pos + n > self.buffer.len() {
            return Err(BinaryError::eof(self.pos));
        }
        let s = &self.buffer[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }

    // PrimitiveType.BOOLEAN
    /// Read a boolean.
    pub fn read_bool(&mut self) -> Result<bool, BinaryError> {
        let b = *self.take(1)?.first().unwrap();
        Ok(b != 0)
    }

    // PrimitiveType.INT8
    /// Read a signed 8-bit integer.
    pub fn read_int8(&mut self) -> Result<i8, BinaryError> {
        let b = self.take(1)?;
        Ok(i8::from_le_bytes([b[0]]))
    }

    // PrimitiveType.INT16
    /// Read a signed 16-bit integer.
    pub fn read_int16(&mut self) -> Result<i16, BinaryError> {
        let b = self.take(2)?;
        Ok(i16::from_le_bytes([b[0], b[1]]))
    }

    // PrimitiveType.INT32
    /// Read a signed 32-bit integer.
    pub fn read_int32(&mut self) -> Result<i32, BinaryError> {
        let b = self.take(4)?;
        Ok(i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    // PrimitiveType.INT64
    /// Read a signed 64-bit integer.
    pub fn read_int64(&mut self) -> Result<i64, BinaryError> {
        let b = self.take(8)?;
        Ok(i64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    // PrimitiveType.INT128
    /// Read a signed 128-bit integer (two's complement, little-endian bytes).
    pub fn read_int128(&mut self) -> Result<i128, BinaryError> {
        let b = self.take(16)?;
        let mut arr = [0u8; 16];
        arr.copy_from_slice(b);
        Ok(i128::from_le_bytes(arr))
    }

    // PrimitiveType.UINT8
    /// Read an unsigned 8-bit integer.
    pub fn read_uint8(&mut self) -> Result<u8, BinaryError> {
        let b = self.take(1)?;
        Ok(b[0])
    }

    // PrimitiveType.UINT16
    /// Read an unsigned 16-bit integer.
    pub fn read_uint16(&mut self) -> Result<u16, BinaryError> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    // PrimitiveType.UINT32
    /// Read an unsigned 32-bit integer.
    pub fn read_uint32(&mut self) -> Result<u32, BinaryError> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    // PrimitiveType.UINT64
    /// Read an unsigned 64-bit integer.
    pub fn read_uint64(&mut self) -> Result<u64, BinaryError> {
        let b = self.take(8)?;
        Ok(u64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    // PrimitiveType.UINT128
    /// Read an unsigned 128-bit integer (two u64 little-endian parts).
    pub fn read_uint128(&mut self) -> Result<u128, BinaryError> {
        let low = self.read_uint64()?;
        let high = self.read_uint64()?;
        Ok(((high as u128) << 64) | (low as u128))
    }

    // PrimitiveType.FLOAT32
    /// Read a 32-bit float.
    pub fn read_float32(&mut self) -> Result<f32, BinaryError> {
        let b = self.take(4)?;
        Ok(f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    // PrimitiveType.FLOAT64
    /// Read a 64-bit float.
    pub fn read_float64(&mut self) -> Result<f64, BinaryError> {
        let b = self.take(8)?;
        Ok(f64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    // PrimitiveType.DATETIME
    /// Read a datetime.
    pub fn read_datetime(&mut self) -> Result<DateTime, BinaryError> {
        let micros = self.read_int64()?;
        Ok(DateTime(micros))
    }

    // PrimitiveType.DATE
    /// Read a date.
    pub fn read_date(&mut self) -> Result<Date, BinaryError> {
        let days = self.read_int64()?;
        Ok(Date(days))
    }

    // PrimitiveType.TIME
    /// Read a time.
    pub fn read_time(&mut self) -> Result<Time, BinaryError> {
        let nanos = self.read_uint64()?;
        Ok(Time(nanos))
    }

    // PrimitiveType.TIMESTAMP
    /// Read a timestamp.
    pub fn read_timestamp(&mut self) -> Result<Timestamp, BinaryError> {
        let nanos = self.read_uint64()?;
        Ok(Timestamp(nanos))
    }

    // PrimitiveType.DURATION
    /// Read a duration.
    pub fn read_duration(&mut self) -> Result<Duration, BinaryError> {
        let nanos = self.read_int64()?;
        Ok(Duration(nanos))
    }

    // PrimitiveType.STRING
    /// Read a UTF-8 string.
    pub fn read_string(&mut self) -> Result<String, BinaryError> {
        let len = self.read_uint32()? as usize;
        let s = self.take(len)?;
        let out = std::str::from_utf8(s)
            .map_err(|e| BinaryError(format!("invalid utf-8: {e}")))?
            .to_string();
        Ok(out)
    }

    // PrimitiveType.CHARACTER
    /// Read a single Unicode character encoded as UTF-8 with a u32 length prefix.
    pub fn read_character(&mut self) -> Result<char, BinaryError> {
        let s = self.read_string()?;
        let mut iter = s.chars();
        let ch = iter
            .next()
            .ok_or_else(|| BinaryError("empty character".to_string()))?;
        if iter.next().is_some() {
            return Err(BinaryError("more than one character".to_string()));
        }
        Ok(ch)
    }

    // PrimitiveType.UUID
    /// Read a UUID as 16 raw bytes, matching Python's `UUID(bytes=...)` big-endian order.
    pub fn read_uuid(&mut self) -> Result<Uuid, BinaryError> {
        let b = self.take(16)?;
        let mut arr = [0u8; 16];
        arr.copy_from_slice(b);
        Ok(Uuid(u128::from_be_bytes(arr)))
    }

    // PrimitiveType.BYTES
    /// Read raw bytes with a u32 length prefix.
    pub fn read_bytes(&mut self) -> Result<Vec<u8>, BinaryError> {
        let len = self.read_uint32()? as usize;
        Ok(self.take(len)?.to_vec())
    }

    // PrimitiveType.JSON
    /// Read JSON in the tagged format used by Python.
    pub fn read_json(&mut self) -> Result<serde_json::Value, BinaryError> {
        let tag = self.read_uint8()?;
        match tag {
            0 => Ok(serde_json::Value::Null),
            1 => Ok(serde_json::Value::Bool(false)),
            2 => Ok(serde_json::Value::Bool(true)),
            3 => Ok(serde_json::Value::from(self.read_int64()?)),
            4 => Ok(serde_json::Value::from(self.read_float64()?)),
            5 => Ok(serde_json::Value::from(self.read_string()?)),
            6 => {
                let len = self.read_uint32()? as usize;
                let mut arr = Vec::with_capacity(len);
                for _ in 0..len {
                    arr.push(self.read_json()?);
                }
                Ok(serde_json::Value::Array(arr))
            }
            7 => {
                let len = self.read_uint32()? as usize;
                let mut map = serde_json::Map::with_capacity(len);
                for _ in 0..len {
                    let key = self.read_string()?;
                    let val = self.read_json()?;
                    map.insert(key, val);
                }
                Ok(serde_json::Value::Object(map))
            }
            _ => Err(BinaryError(format!(
                "invalid JSON type tag at {}: {tag}",
                self.pos.saturating_sub(1)
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn enc_bytes<F: FnOnce(&mut BinaryEncoder)>(f: F) -> Vec<u8> {
        let mut enc = BinaryEncoder::new();
        f(&mut enc);
        enc.to_bytes().to_vec()
    }

    #[test]
    fn test_bool() {
        let bytes = enc_bytes(|e| {
            e.write_bool(true);
            e.write_bool(false);
        });
        assert_eq!(bytes.len(), 2);
        let mut d = BinaryDecoder::new(&bytes);
        assert_eq!(d.read_bool().unwrap(), true);
        assert_eq!(d.read_bool().unwrap(), false);
        assert_eq!(d.remaining(), 0);
    }

    #[test]
    fn test_int8() {
        let vals = [0i8, 1, -1, 127, -128, 42, -42];
        let bytes = enc_bytes(|e| {
            for v in vals {
                e.write_int8(v);
            }
        });
        assert_eq!(bytes.len(), 7);
        let mut d = BinaryDecoder::new(&bytes);
        for &exp in &vals {
            assert_eq!(d.read_int8().unwrap(), exp);
        }
        assert_eq!(d.remaining(), 0);
    }

    #[test]
    fn test_int16() {
        let vals = [0i16, 1, -1, 127, -128, 32767, -32768, 42, -42];
        let bytes = enc_bytes(|e| {
            for v in vals {
                e.write_int16(v);
            }
        });
        assert_eq!(bytes.len(), 18);
        let mut d = BinaryDecoder::new(&bytes);
        for &exp in &vals {
            assert_eq!(d.read_int16().unwrap(), exp);
        }
        assert_eq!(d.remaining(), 0);
    }

    #[test]
    fn test_int32() {
        let vals = [
            0i32,
            1,
            -1,
            127,
            -128,
            32767,
            -32768,
            2_147_483_647,
            -2_147_483_648,
            42,
            -42,
        ];
        let bytes = enc_bytes(|e| {
            for v in vals {
                e.write_int32(v);
            }
        });
        assert_eq!(bytes.len(), 44);
        let mut d = BinaryDecoder::new(&bytes);
        for &exp in &vals {
            assert_eq!(d.read_int32().unwrap(), exp);
        }
        assert_eq!(d.remaining(), 0);
    }

    #[test]
    fn test_int64() {
        let vals = [
            0i64,
            1,
            -1,
            127,
            -128,
            32767,
            -32768,
            2_147_483_647,
            -2_147_483_648,
            9_223_372_036_854_775_807,
            -9_223_372_036_854_775_808,
            42,
            -42,
        ];
        let bytes = enc_bytes(|e| {
            for v in vals {
                e.write_int64(v);
            }
        });
        assert_eq!(bytes.len(), 104);
        let mut d = BinaryDecoder::new(&bytes);
        for &exp in &vals {
            assert_eq!(d.read_int64().unwrap(), exp);
        }
        assert_eq!(d.remaining(), 0);
    }

    #[test]
    fn test_int128() {
        let vals: [i128; 11] = [
            0,
            1,
            -1,
            127,
            -128,
            (1i128 << 63) - 1,
            -(1i128 << 63),
            i128::MAX,
            i128::MIN,
            42,
            -42,
        ];
        let bytes = enc_bytes(|e| {
            for v in vals {
                e.write_int128(v);
            }
        });
        assert_eq!(bytes.len(), 176);
        let mut d = BinaryDecoder::new(&bytes);
        for &exp in &vals {
            assert_eq!(d.read_int128().unwrap(), exp);
        }
        assert_eq!(d.remaining(), 0);
    }

    #[test]
    fn test_uint8() {
        let vals = [0u8, 1, 127, 128, 255, 42];
        let bytes = enc_bytes(|e| {
            for v in vals {
                e.write_uint8(v);
            }
        });
        assert_eq!(bytes.len(), 6);
        let mut d = BinaryDecoder::new(&bytes);
        for &exp in &vals {
            assert_eq!(d.read_uint8().unwrap(), exp);
        }
        assert_eq!(d.remaining(), 0);
    }

    #[test]
    fn test_uint16() {
        let vals = [0u16, 1, 127, 128, 255, 256, 32767, 32768, 65535, 42];
        let bytes = enc_bytes(|e| {
            for v in vals {
                e.write_uint16(v);
            }
        });
        assert_eq!(bytes.len(), 20);
        let mut d = BinaryDecoder::new(&bytes);
        for &exp in &vals {
            assert_eq!(d.read_uint16().unwrap(), exp);
        }
        assert_eq!(d.remaining(), 0);
    }

    #[test]
    fn test_uint32() {
        let vals = [
            0u32,
            1,
            127,
            128,
            255,
            256,
            65_535,
            65_536,
            2_147_483_647,
            2_147_483_648,
            4_294_967_295,
            42,
        ];
        let bytes = enc_bytes(|e| {
            for v in vals {
                e.write_uint32(v);
            }
        });
        assert_eq!(bytes.len(), 48);
        let mut d = BinaryDecoder::new(&bytes);
        for &exp in &vals {
            assert_eq!(d.read_uint32().unwrap(), exp);
        }
        assert_eq!(d.remaining(), 0);
    }

    #[test]
    fn test_uint64() {
        let vals = [
            0u64,
            1,
            127,
            128,
            255,
            256,
            65_535,
            65_536,
            4_294_967_295,
            4_294_967_296,
            9_223_372_036_854_775_807,
            9_223_372_036_854_775_808,
            18_446_744_073_709_551_615,
            42,
        ];
        let bytes = enc_bytes(|e| {
            for v in vals {
                e.write_uint64(v);
            }
        });
        assert_eq!(bytes.len(), 112);
        let mut d = BinaryDecoder::new(&bytes);
        for &exp in &vals {
            assert_eq!(d.read_uint64().unwrap(), exp);
        }
        assert_eq!(d.remaining(), 0);
    }

    #[test]
    fn test_uint128() {
        let vals: [u128; 11] = [
            0,
            1,
            127,
            128,
            255,
            256,
            (1u128 << 64) - 1,
            1u128 << 64,
            1u128 << 100,
            u128::MAX,
            42,
        ];
        let bytes = enc_bytes(|e| {
            for v in vals {
                e.write_uint128(v);
            }
        });
        assert_eq!(bytes.len(), 176);
        let mut d = BinaryDecoder::new(&bytes);
        for &exp in &vals {
            assert_eq!(d.read_uint128().unwrap(), exp);
        }
        assert_eq!(d.remaining(), 0);
    }

    #[test]
    fn test_float32() {
        let vals = [
            0.0f32,
            -0.0,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NAN,
            1.0,
            -1.0,
            std::f32::consts::PI,
            -std::f32::consts::PI,
        ];
        let bytes = enc_bytes(|e| {
            for &v in &vals {
                e.write_float32(v);
            }
        });
        assert_eq!(bytes.len(), 36);
        let mut d = BinaryDecoder::new(&bytes);
        for &v in &vals {
            let res = d.read_float32().unwrap();
            if v.is_nan() {
                assert!(res.is_nan());
            } else if v.is_infinite() {
                assert!(res.is_infinite() && res.is_sign_positive() == v.is_sign_positive());
            } else {
                assert!((res - v).abs() <= 1e-6);
            }
        }
    }

    #[test]
    fn test_float64() {
        let vals = [
            0.0f64,
            -0.0,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
            1.0,
            -1.0,
            42.0,
            -42.0,
            1000.0,
            -1000.0,
            (1u64 << 53) as f64,
            -((1u64 << 53) as f64),
            std::f64::consts::PI,
            -std::f64::consts::PI,
            1e100,
            -1e100,
        ];
        let bytes = enc_bytes(|e| {
            for &v in &vals {
                e.write_float64(v);
            }
        });
        assert_eq!(bytes.len(), 136);
        let mut d = BinaryDecoder::new(&bytes);
        for &v in &vals {
            let res = d.read_float64().unwrap();
            if v.is_nan() {
                assert!(res.is_nan());
            } else if v.is_infinite() {
                assert!(res.is_infinite() && res.is_sign_positive() == v.is_sign_positive());
            } else {
                assert!((res - v).abs() <= 1e-9);
            }
        }
    }

    #[test]
    fn test_string_and_character() {
        let strings = ["", "hello", "Hello, 世界!", &"a".repeat(1000)];
        let bytes = enc_bytes(|e| {
            for s in &strings {
                e.write_string(s);
            }
        });
        let mut d = BinaryDecoder::new(&bytes);
        for s in &strings {
            assert_eq!(d.read_string().unwrap(), *s);
        }
        assert_eq!(d.remaining(), 0);

        let chars = ['A', 'é', '世', '🌍', '\u{0000}'];
        let bytes = enc_bytes(|e| {
            for &c in &chars {
                e.write_character(c);
            }
        });
        let mut d = BinaryDecoder::new(&bytes);
        for &c in &chars {
            assert_eq!(d.read_character().unwrap(), c);
        }
        assert_eq!(d.remaining(), 0);
    }

    #[test]
    fn test_bytes() {
        let sets: Vec<Vec<u8>> = vec![
            vec![],
            b"hello".to_vec(),
            vec![0, 1, 2, 3],
            (0..=255u8).collect(),
        ];
        let bytes = enc_bytes(|e| {
            for s in &sets {
                e.write_bytes(s);
            }
        });
        let mut d = BinaryDecoder::new(&bytes);
        for s in &sets {
            assert_eq!(d.read_bytes().unwrap(), *s);
        }
        assert_eq!(d.remaining(), 0);
    }

    #[test]
    fn test_mixed_types() {
        let bytes = enc_bytes(|e| {
            e.write_bool(true);
            e.write_int8(-42);
            e.write_uint16(65_535);
            e.write_int32(-1_234_567);
            e.write_int128(-((1i128) << 100));
            e.write_uint128(1u128 << 100);
            e.write_float32(std::f32::consts::PI);
            e.write_float64(std::f64::consts::E);
            e.write_string("test string");
            e.write_bytes(&[1, 2, 3]);
            e.write_character('Z');
        });
        let mut d = BinaryDecoder::new(&bytes);
        assert_eq!(d.read_bool().unwrap(), true);
        assert_eq!(d.read_int8().unwrap(), -42);
        assert_eq!(d.read_uint16().unwrap(), 65_535);
        assert_eq!(d.read_int32().unwrap(), -1_234_567);
        assert_eq!(d.read_int128().unwrap(), -((1i128) << 100));
        assert_eq!(d.read_uint128().unwrap(), 1u128 << 100);
        assert!((d.read_float32().unwrap() - std::f32::consts::PI).abs() <= 1e-6);
        assert!((d.read_float64().unwrap() - std::f64::consts::E).abs() <= 1e-9);
        assert_eq!(d.read_string().unwrap(), "test string");
        assert_eq!(d.read_bytes().unwrap(), vec![1, 2, 3]);
        assert_eq!(d.read_character().unwrap(), 'Z');
        assert_eq!(d.remaining(), 0);
    }

    #[test]
    fn test_datetime() {
        // unix epoch
        let vals = [
            DateTime(0),
            DateTime(1_705_532_245_123_456), // 2024-01-15T14:30:45.123456Z
            DateTime(-1),
            DateTime(4_102_444_800_000_000), // 2100-01-01
            DateTime(962_140_800_999_999),
            DateTime(-2_205_024_000_000_000), // 1900-01-01
            DateTime(946_684_800_000_000),    // 2000-01-01
            // 2400-02-29 roughly: compute via date math if needed; use large positive
            DateTime(1_496_275_200_000_000 / 10),
        ];
        let bytes = enc_bytes(|e| {
            for v in vals {
                e.write_datetime(v);
            }
        });
        let mut d = BinaryDecoder::new(&bytes);
        for &exp in &vals {
            assert_eq!(d.read_datetime().unwrap(), exp);
        }
        assert_eq!(d.remaining(), 0);
    }

    #[test]
    fn test_date() {
        let vals = [
            Date(0),
            Date(19_784),
            Date(-1),
            Date(47_468),
            Date(-25_569),
            Date(11_016),
            Date(157_814),
        ];
        let bytes = enc_bytes(|e| {
            for v in vals {
                e.write_date(v);
            }
        });
        let mut d = BinaryDecoder::new(&bytes);
        for &exp in &vals {
            assert_eq!(d.read_date().unwrap(), exp);
        }
        assert_eq!(d.remaining(), 0);
    }

    #[test]
    fn test_time() {
        let vals = [
            Time(0),
            Time(12 * 3_600_000_000_000),
            Time(23 * 3_600_000_000_000 + 59 * 60_000_000_000 + 59 * 1_000_000_000 + 999_999_000),
            Time(14 * 3_600_000_000_000 + 30 * 60_000_000_000 + 45 * 1_000_000_000 + 123_456_000),
            Time(1_000),
        ];
        let bytes = enc_bytes(|e| {
            for v in vals {
                e.write_time(v);
            }
        });
        let mut d = BinaryDecoder::new(&bytes);
        for &exp in &vals {
            assert_eq!(d.read_time().unwrap(), exp);
        }
        assert_eq!(d.remaining(), 0);
    }

    #[test]
    fn test_timestamp() {
        let vals = [
            0u64,
            1,
            999_999_999,
            1_000_000_000,
            1_000_000_001,
            (1u64 << 32) - 1,
            1u64 << 32,
            (1u64 << 63) - 1,
            1u64 << 63,
            u64::MAX,
        ];
        let bytes = enc_bytes(|e| {
            for &v in &vals {
                e.write_timestamp(Timestamp(v));
            }
        });
        let mut d = BinaryDecoder::new(&bytes);
        for &v in &vals {
            assert_eq!(d.read_timestamp().unwrap().0, v);
        }
        assert_eq!(d.remaining(), 0);
    }

    #[test]
    fn test_duration() {
        let vals = [
            Duration(0),
            Duration(86_400_000_000_000),
            Duration(5_444_500_000_000),
            Duration(1_000),
            Duration(-86_400_000_000_000),
            Duration(52 * 7 * 24 * 3_600_000_000_000i64),
            Duration(
                365 * 24 * 3_600_000_000_000i64
                    + 5 * 3_600_000_000_000
                    + 48 * 60_000_000_000
                    + 46 * 1_000_000_000,
            ),
            Duration(86_400_000_000_000 - 1_000),
            Duration(-1_000_000_000),
            Duration(86_400_000_000_000 - 1_000),
            Duration(87_600 * 3_600_000_000_000i64),
            Duration(-1_000),
            Duration(86_400_000_000_000 - 1_000),
            Duration(1_000_000 - 1),
        ];
        let bytes = enc_bytes(|e| {
            for v in vals {
                e.write_duration(v);
            }
        });
        let mut d = BinaryDecoder::new(&bytes);
        for &exp in &vals {
            assert_eq!(d.read_duration().unwrap(), exp);
        }
        assert_eq!(d.remaining(), 0);
    }

    #[test]
    fn test_uuid() {
        let ids = [
            Uuid(0),
            Uuid(0x1234_5678_1234_5678_1234_5678_1234_5678_u128),
            Uuid(0x550e_8400_e29b_41d4_a716_4466_5544_0000_u128),
            Uuid(u128::MAX),
        ];
        let bytes = enc_bytes(|e| {
            for &id in &ids {
                e.write_uuid(id);
            }
        });
        assert_eq!(bytes.len(), ids.len() * 16);
        let mut d = BinaryDecoder::new(&bytes);
        for &exp in &ids {
            assert_eq!(d.read_uuid().unwrap(), exp);
        }
        assert_eq!(d.remaining(), 0);
    }

    #[test]
    fn test_json() {
        use serde_json::json;
        let tests = vec![
            json!(null),
            json!(true),
            json!(false),
            json!(42),
            json!(std::f64::consts::PI),
            json!("hello"),
            json!([]),
            json!([1, 2, 3]),
            json!({"key": "value"}),
            json!({"nested": {"data": [1,2,{"more": "stuff"}]}}),
            json!(["mixed", 123, true, null, {"obj": "ect"}]),
            json!({
                "id": 1,
                "name": "Alice",
                "email": "alice@example.com",
                "profile": {"age": 30, "preferences": {"theme": "dark", "notifications": true, "languages": ["en","fr","es"]}, "metadata": null},
                "roles": ["admin", "user"]
            }),
            json!({
                "level1": {"level2": {"level3": {"level4": {"level5": [
                    {"data": [1,2,[3,4,[5,6]]]},
                    {"more": {"even": {"deeper": {"nesting": true}}}},
                ]}}}}
            }),
            json!({
                "unicode": "Hello 世界 🌍 🚀",
                "special_chars": "!@#$%^&*()_+-=[]{}|;':\",./<>?",
                "escaped": "line1\nline2\ttab\"quote'apostrophe\\backslash",
                "empty_string": "",
                "whitespace": "   \n\t\r   ",
            }),
            json!({
                "integers": [0, -1, 1, 2_147_483_647, -2_147_483_648],
                "floats": [0.0, -0.0, 1.5, -1.5, 1e10, 1e-10, f64::INFINITY, f64::NEG_INFINITY],
                "scientific": [1.23e-4, 5.67e8, -9.87e-6],
            }),
            json!({
                "matrix": [[true, false, null], [null, true, false], [false, null, true]],
                "flags": {"enabled": true, "disabled": false, "unknown": null, "nested_flags": {"a": true, "b": false, "c": null}},
            }),
        ];
        let bytes = enc_bytes(|e| {
            for v in &tests {
                e.write_json(v).unwrap();
            }
        });
        let mut d = BinaryDecoder::new(&bytes);
        for exp in &tests {
            let got = d.read_json().unwrap();
            assert_eq!(&got, exp);
        }
        assert_eq!(d.remaining(), 0);
    }
}
