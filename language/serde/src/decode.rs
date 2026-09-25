use std::str;

use serde::Deserialize;
use serde::de::{self, EnumAccess, IntoDeserializer, MapAccess, SeqAccess, VariantAccess, Visitor};

use crate::{Error, IntegerEncoding, Result};

/// Maximum number of bytes needed for one encoded u128 varint.
const U128_VARINT_MAX_BYTES: usize = 19;
/// Maximum final byte value for one encoded u128 varint.
const U128_VARINT_LAST_BYTE_MAX: u8 = 0x03;

/// Decode one value from canonical TS++ binary bytes.
pub fn from_slice<'de, T>(bytes: &'de [u8]) -> Result<T>
where
    T: Deserialize<'de>,
{
    let mut decoder = Decoder::new(bytes, IntegerEncoding::Compact);
    let value = T::deserialize(&mut decoder)?;
    decoder.finish()?;

    Ok(value)
}

/// Decode one value encoded with fixed-width integers.
pub fn from_slice_fixed<'de, T>(bytes: &'de [u8]) -> Result<T>
where
    T: Deserialize<'de>,
{
    let mut decoder = Decoder::new(bytes, IntegerEncoding::Fixed);
    let value = T::deserialize(&mut decoder)?;
    decoder.finish()?;

    Ok(value)
}

/// Stateful binary decoder.
#[derive(Debug)]
struct Decoder<'de> {
    /// Encoded input bytes.
    bytes: &'de [u8],
    /// Current read offset.
    offset: usize,
    /// Integer representation read by this decoder.
    integers: IntegerEncoding,
}

impl<'de> Decoder<'de> {
    /// Create a new decoder.
    fn new(bytes: &'de [u8], integers: IntegerEncoding) -> Self {
        Self {
            bytes,
            offset: 0,
            integers,
        }
    }

    /// Validate that all bytes were consumed.
    fn finish(&self) -> Result<()> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(Error::TrailingBytes)
        }
    }

    /// Read one byte.
    fn read_byte(&mut self) -> Result<u8> {
        let Some(byte) = self.bytes.get(self.offset).copied() else {
            return Err(Error::UnexpectedEnd);
        };
        self.offset += 1;

        Ok(byte)
    }

    /// Read exact raw bytes.
    fn read_bytes(&mut self, len: usize) -> Result<&'de [u8]> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or(Error::ByteRangeOverflow)?;
        if end > self.bytes.len() {
            return Err(Error::UnexpectedEnd);
        }

        let bytes = &self.bytes[self.offset..end];
        self.offset = end;

        Ok(bytes)
    }

    /// Read one length-prefixed byte slice.
    fn read_byte_slice(&mut self) -> Result<&'de [u8]> {
        let len = self.read_usize()?;

        self.read_bytes(len)
    }

    /// Read one usize.
    fn read_usize(&mut self) -> Result<usize> {
        let value = match self.integers {
            IntegerEncoding::Compact => self.read_varint()?,
            IntegerEncoding::Fixed => self.read_unsigned::<8>()?,
        };

        usize::try_from(value).map_err(|_| Error::UsizeOutOfRange)
    }

    /// Read one unsigned integer using its declared width.
    fn read_unsigned<const N: usize>(&mut self) -> Result<u128> {
        if matches!(self.integers, IntegerEncoding::Compact) {
            return self.read_varint();
        }

        let bytes = self.read_bytes(N)?;
        let mut value = [0; 16];
        value[..N].copy_from_slice(bytes);

        Ok(u128::from_le_bytes(value))
    }

    /// Read one signed integer using its declared width.
    fn read_signed<const N: usize>(&mut self) -> Result<i128> {
        if matches!(self.integers, IntegerEncoding::Compact) {
            let value = self.read_varint()?;
            let decoded = ((value >> 1) as i128) ^ (-((value & 1) as i128));

            return Ok(decoded);
        }

        let bytes = self.read_bytes(N)?;
        let fill = if bytes[N - 1] & 0x80 == 0 { 0 } else { 0xff };
        let mut value = [fill; 16];
        value[..N].copy_from_slice(bytes);

        Ok(i128::from_le_bytes(value))
    }

    /// Read one canonical unsigned varint.
    fn read_varint(&mut self) -> Result<u128> {
        let mut value = 0u128;
        let mut shift = 0u32;
        let mut byte_count = 0usize;

        loop {
            if byte_count == U128_VARINT_MAX_BYTES {
                return Err(Error::VarintTooLarge);
            }
            byte_count += 1;

            let byte = self.read_byte()?;
            let chunk = (byte & 0x7f) as u128;
            if shift == 126 && byte & 0x7f > U128_VARINT_LAST_BYTE_MAX {
                return Err(Error::VarintTooLarge);
            }
            value |= chunk << shift;

            if byte & 0x80 == 0 {
                if shift > 0 && chunk == 0 {
                    return Err(Error::NonCanonicalVarint);
                }

                return Ok(value);
            }

            shift += 7;
        }
    }
}

impl<'de> de::Deserializer<'de> for &mut Decoder<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::SelfDescribingUnsupported)
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.read_byte()? {
            0 => visitor.visit_bool(false),
            1 => visitor.visit_bool(true),
            _ => Err(Error::InvalidBool),
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let value = self.read_byte()? as i8;

        visitor.visit_i8(value)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let value =
            i16::try_from(self.read_signed::<2>()?).map_err(|_| Error::IntegerOutOfRange("i16"))?;

        visitor.visit_i16(value)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let value =
            i32::try_from(self.read_signed::<4>()?).map_err(|_| Error::IntegerOutOfRange("i32"))?;

        visitor.visit_i32(value)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let value =
            i64::try_from(self.read_signed::<8>()?).map_err(|_| Error::IntegerOutOfRange("i64"))?;

        visitor.visit_i64(value)
    }

    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i128(self.read_signed::<16>()?)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let value = self.read_byte()?;

        visitor.visit_u8(value)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let value = u16::try_from(self.read_unsigned::<2>()?)
            .map_err(|_| Error::IntegerOutOfRange("u16"))?;

        visitor.visit_u16(value)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let value = u32::try_from(self.read_unsigned::<4>()?)
            .map_err(|_| Error::IntegerOutOfRange("u32"))?;

        visitor.visit_u32(value)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let value = u64::try_from(self.read_unsigned::<8>()?)
            .map_err(|_| Error::IntegerOutOfRange("u64"))?;

        visitor.visit_u64(value)
    }

    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u128(self.read_unsigned::<16>()?)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let bytes = self.read_bytes(4)?;
        let bytes = bytes
            .try_into()
            .map_err(|_| Error::IntegerOutOfRange("f32"))?;

        visitor.visit_f32(f32::from_le_bytes(bytes))
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let bytes = self.read_bytes(8)?;
        let bytes = bytes
            .try_into()
            .map_err(|_| Error::IntegerOutOfRange("f64"))?;

        visitor.visit_f64(f64::from_le_bytes(bytes))
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let value = u32::try_from(self.read_unsigned::<4>()?)
            .map_err(|_| Error::IntegerOutOfRange("char"))?;
        let value = char::from_u32(value).ok_or(Error::InvalidChar)?;

        visitor.visit_char(value)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let bytes = self.read_byte_slice()?;
        let value = str::from_utf8(bytes).map_err(|_| Error::InvalidUtf8)?;

        visitor.visit_borrowed_str(value)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_borrowed_bytes(self.read_byte_slice()?)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_byte_buf(self.read_byte_slice()?.to_vec())
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.read_byte()? {
            0 => visitor.visit_none(),
            1 => visitor.visit_some(self),
            _ => Err(Error::InvalidOption),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let len = self.read_usize()?;
        visitor.visit_seq(SequenceDecoder {
            decoder: self,
            remaining: len,
        })
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_seq(SequenceDecoder {
            decoder: self,
            remaining: len,
        })
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let len = self.read_usize()?;
        visitor.visit_map(MapDecoder {
            decoder: self,
            remaining: len,
            value_pending: false,
        })
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_seq(SequenceDecoder {
            decoder: self,
            remaining: fields.len(),
        })
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let variant =
            u32::try_from(self.read_unsigned::<4>()?).map_err(|_| Error::EnumVariantOutOfRange)?;

        visitor.visit_enum(EnumDecoder {
            decoder: self,
            variant,
        })
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_u32(visitor)
    }

    fn deserialize_ignored_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::IgnoredAnyUnsupported)
    }
}

/// Decoder for sequence fields and elements.
struct SequenceDecoder<'a, 'de> {
    /// Underlying decoder.
    decoder: &'a mut Decoder<'de>,
    /// Number of remaining elements.
    remaining: usize,
}

impl<'de> SeqAccess<'de> for SequenceDecoder<'_, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: de::DeserializeSeed<'de>,
    {
        if self.remaining == 0 {
            return Ok(None);
        }

        self.remaining -= 1;
        seed.deserialize(&mut *self.decoder).map(Some)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.remaining)
    }
}

/// Decoder for maps.
struct MapDecoder<'a, 'de> {
    /// Underlying decoder.
    decoder: &'a mut Decoder<'de>,
    /// Number of remaining entries.
    remaining: usize,
    /// Whether a value must be read before the next key.
    value_pending: bool,
}

impl<'de> MapAccess<'de> for MapDecoder<'_, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: de::DeserializeSeed<'de>,
    {
        if self.value_pending {
            return Err(Error::MapValueNotRead);
        }
        if self.remaining == 0 {
            return Ok(None);
        }

        self.value_pending = true;
        seed.deserialize(&mut *self.decoder).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: de::DeserializeSeed<'de>,
    {
        if !self.value_pending {
            return Err(Error::MapValueWithoutKey);
        }

        self.value_pending = false;
        self.remaining -= 1;
        seed.deserialize(&mut *self.decoder)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.remaining)
    }
}

/// Decoder for enum variants.
struct EnumDecoder<'a, 'de> {
    /// Underlying decoder.
    decoder: &'a mut Decoder<'de>,
    /// Variant index.
    variant: u32,
}

impl<'de> EnumAccess<'de> for EnumDecoder<'_, 'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: de::DeserializeSeed<'de>,
    {
        let variant = self.variant.into_deserializer();
        let value = seed.deserialize(variant)?;

        Ok((value, self))
    }
}

impl<'de> VariantAccess<'de> for EnumDecoder<'_, 'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: de::DeserializeSeed<'de>,
    {
        seed.deserialize(self.decoder)
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_tuple(self.decoder, len, visitor)
    }

    fn struct_variant<V>(self, fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_tuple(self.decoder, fields.len(), visitor)
    }
}
