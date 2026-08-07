use serde::Serialize;
use serde::ser::{
    self, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
    SerializeTupleStruct, SerializeTupleVariant,
};

use crate::{Error, Result};

/// Maximum number of bytes needed for one encoded u128 varint.
const U128_VARINT_MAX_BYTES: usize = 19;

/// Encode one value into canonical Destack binary bytes.
pub fn to_vec<T>(value: &T) -> Result<Vec<u8>>
where
    T: Serialize + ?Sized,
{
    let mut encoder = Encoder::new_vec();
    value.serialize(&mut encoder)?;

    Ok(encoder.into_bytes())
}

/// Append one value to canonical Destack binary bytes.
pub fn append_to_vec<T>(value: &T, output: &mut Vec<u8>) -> Result<()>
where
    T: Serialize + ?Sized,
{
    let mut encoder = Encoder::new_vec_ref(output);
    value.serialize(&mut encoder)?;

    Ok(())
}

/// Return the encoded length of one value.
pub fn encoded_len<T>(value: &T) -> Result<usize>
where
    T: Serialize + ?Sized,
{
    let mut encoder = Encoder::new_count();
    value.serialize(&mut encoder)?;

    Ok(encoder.len())
}

/// Encode one value into one hasher, without buffering bytes.
pub fn hash_into<T, H>(value: &T, hasher: &mut H) -> Result<()>
where
    T: Serialize + ?Sized,
    H: std::hash::Hasher,
{
    let mut encoder = Encoder::new_hasher(hasher);
    value.serialize(&mut encoder)?;
    encoder.flush_hasher();

    Ok(())
}

/// Encode one value into one caller-owned byte slice.
pub fn to_slice<'a, T>(value: &T, output: &'a mut [u8]) -> Result<&'a mut [u8]>
where
    T: Serialize + ?Sized,
{
    let len = {
        let mut encoder = Encoder::new_slice(output);
        value.serialize(&mut encoder)?;
        encoder.len()
    };

    Ok(&mut output[..len])
}

/// Stateful binary encoder.
struct Encoder<'output> {
    /// Encoded output destination.
    output: EncoderOutput<'output>,
}

/// Encoder output destination.
enum EncoderOutput<'output> {
    /// Growable byte vector.
    Vec(Vec<u8>),
    /// Borrowed growable byte vector.
    VecRef(&'output mut Vec<u8>),
    /// Caller-owned byte slice.
    Slice {
        /// Writable bytes.
        bytes: &'output mut [u8],
        /// Number of bytes written.
        len: usize,
    },
    /// Byte count only.
    Count(usize),
    /// Streaming hasher sink with block buffering.
    Hasher {
        /// The destination hasher.
        hasher: &'output mut dyn std::hash::Hasher,
        /// Buffered bytes not yet hashed.
        buffer: [u8; 64],
        /// Number of buffered bytes.
        len: usize,
    },
}

impl std::fmt::Debug for Encoder<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sink = match &self.output {
            EncoderOutput::Vec(_) => "vec",
            EncoderOutput::VecRef(_) => "vec_ref",
            EncoderOutput::Slice { .. } => "slice",
            EncoderOutput::Count(_) => "count",
            EncoderOutput::Hasher { .. } => "hasher",
        };

        formatter
            .debug_struct("Encoder")
            .field("output", &sink)
            .finish()
    }
}

impl Encoder<'_> {
    /// Return encoded bytes.
    fn into_bytes(self) -> Vec<u8> {
        match self.output {
            EncoderOutput::Vec(bytes) => bytes,
            EncoderOutput::VecRef(_) => {
                unreachable!("borrowed vector encoders cannot return owned bytes")
            }
            _ => unreachable!("only vector encoders can return owned bytes"),
        }
    }

    /// Return the number of encoded bytes.
    fn len(&self) -> usize {
        match &self.output {
            EncoderOutput::Hasher { .. } => 0,
            EncoderOutput::Vec(bytes) => bytes.len(),
            EncoderOutput::VecRef(bytes) => bytes.len(),
            EncoderOutput::Slice { len, .. } => *len,
            EncoderOutput::Count(len) => *len,
        }
    }

    /// Create a new growable byte encoder.
    fn new_vec() -> Self {
        Self {
            output: EncoderOutput::Vec(Vec::new()),
        }
    }

    /// Create a new borrowed growable byte encoder.
    fn new_vec_ref(output: &mut Vec<u8>) -> Encoder<'_> {
        Encoder {
            output: EncoderOutput::VecRef(output),
        }
    }

    /// Create a new slice encoder.
    fn new_slice(output: &mut [u8]) -> Encoder<'_> {
        Encoder {
            output: EncoderOutput::Slice {
                bytes: output,
                len: 0,
            },
        }
    }

    /// Create a new length-counting encoder.
    fn new_count() -> Self {
        Self {
            output: EncoderOutput::Count(0),
        }
    }

    /// Create a new streaming hasher encoder.
    fn new_hasher(hasher: &mut dyn std::hash::Hasher) -> Encoder<'_> {
        Encoder {
            output: EncoderOutput::Hasher {
                hasher,
                buffer: [0; 64],
                len: 0,
            },
        }
    }

    /// Flush buffered hasher bytes.
    fn flush_hasher(&mut self) {
        if let EncoderOutput::Hasher {
            hasher,
            buffer,
            len,
        } = &mut self.output
            && *len > 0
        {
            hasher.write(&buffer[..*len]);
            *len = 0;
        }
    }

    /// Write one raw byte.
    fn write_byte(&mut self, byte: u8) -> Result<()> {
        match &mut self.output {
            EncoderOutput::Hasher {
                hasher,
                buffer,
                len,
            } => {
                if *len == buffer.len() {
                    hasher.write(buffer);
                    *len = 0;
                }
                buffer[*len] = byte;
                *len += 1;

                Ok(())
            }
            EncoderOutput::Vec(output) => {
                output.push(byte);

                Ok(())
            }
            EncoderOutput::VecRef(output) => {
                output.push(byte);

                Ok(())
            }
            EncoderOutput::Slice { bytes, len } => {
                if *len == bytes.len() {
                    return Err(Error::BufferTooSmall);
                }

                bytes[*len] = byte;
                *len += 1;

                Ok(())
            }
            EncoderOutput::Count(len) => {
                *len = len.checked_add(1).ok_or(Error::LengthOverflow)?;

                Ok(())
            }
        }
    }

    /// Write raw bytes.
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        match &mut self.output {
            EncoderOutput::Hasher {
                hasher,
                buffer,
                len,
            } => {
                if *len + bytes.len() <= buffer.len() {
                    buffer[*len..*len + bytes.len()].copy_from_slice(bytes);
                    *len += bytes.len();

                    return Ok(());
                }
                if *len > 0 {
                    hasher.write(&buffer[..*len]);
                    *len = 0;
                }
                hasher.write(bytes);

                Ok(())
            }
            EncoderOutput::Vec(output) => {
                output.extend_from_slice(bytes);

                Ok(())
            }
            EncoderOutput::VecRef(output) => {
                output.extend_from_slice(bytes);

                Ok(())
            }
            EncoderOutput::Slice { bytes: output, len } => {
                let end = len.checked_add(bytes.len()).ok_or(Error::LengthOverflow)?;
                if end > output.len() {
                    return Err(Error::BufferTooSmall);
                }

                output[*len..end].copy_from_slice(bytes);
                *len = end;

                Ok(())
            }
            EncoderOutput::Count(len) => {
                *len = len.checked_add(bytes.len()).ok_or(Error::LengthOverflow)?;

                Ok(())
            }
        }
    }

    /// Write a length-prefixed byte slice.
    fn write_byte_slice(&mut self, bytes: &[u8]) -> Result<()> {
        self.write_usize(bytes.len())?;
        self.write_bytes(bytes)
    }

    /// Write one usize as a varint.
    fn write_usize(&mut self, value: usize) -> Result<()> {
        self.write_u128(value as u128)
    }

    /// Write one unsigned integer as a varint.
    fn write_u128(&mut self, mut value: u128) -> Result<()> {
        // single byte values write straight through
        if value < 0x80 {
            return self.write_byte(value as u8);
        }

        // buffer the varint once, then write it in one call
        let mut bytes = [0u8; U128_VARINT_MAX_BYTES];
        let mut byte_count = 0usize;
        loop {
            let mut byte = (value & 0x7f) as u8;
            value >>= 7;
            if value != 0 {
                byte |= 0x80;
            }
            bytes[byte_count] = byte;
            byte_count += 1;
            if value == 0 {
                break;
            }
        }

        self.write_bytes(&bytes[..byte_count])
    }

    /// Write one signed integer as a zigzag varint.
    fn write_i128(&mut self, value: i128) -> Result<()> {
        let encoded = ((value as u128) << 1) ^ ((value >> 127) as u128);

        self.write_u128(encoded)
    }

    /// Encode one value into nested bytes.
    fn nested_bytes<T>(value: &T) -> Result<Vec<u8>>
    where
        T: Serialize + ?Sized,
    {
        let mut encoder = Encoder::new_vec();
        value.serialize(&mut encoder)?;

        Ok(encoder.into_bytes())
    }
}

impl<'a, 'output> ser::Serializer for &'a mut Encoder<'output> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = SequenceEncoder<'a, 'output>;
    type SerializeTuple = SequenceEncoder<'a, 'output>;
    type SerializeTupleStruct = SequenceEncoder<'a, 'output>;
    type SerializeTupleVariant = SequenceEncoder<'a, 'output>;
    type SerializeMap = MapEncoder<'a, 'output>;
    type SerializeStruct = SequenceEncoder<'a, 'output>;
    type SerializeStructVariant = SequenceEncoder<'a, 'output>;

    fn serialize_bool(self, value: bool) -> Result<()> {
        self.write_byte(u8::from(value))
    }

    fn serialize_i8(self, value: i8) -> Result<()> {
        self.write_byte(value as u8)
    }

    fn serialize_i16(self, value: i16) -> Result<()> {
        self.write_i128(value as i128)
    }

    fn serialize_i32(self, value: i32) -> Result<()> {
        self.write_i128(value as i128)
    }

    fn serialize_i64(self, value: i64) -> Result<()> {
        self.write_i128(value as i128)
    }

    fn serialize_i128(self, value: i128) -> Result<()> {
        self.write_i128(value)
    }

    fn serialize_u8(self, value: u8) -> Result<()> {
        self.write_byte(value)
    }

    fn serialize_u16(self, value: u16) -> Result<()> {
        self.write_u128(value as u128)
    }

    fn serialize_u32(self, value: u32) -> Result<()> {
        self.write_u128(value as u128)
    }

    fn serialize_u64(self, value: u64) -> Result<()> {
        self.write_u128(value as u128)
    }

    fn serialize_u128(self, value: u128) -> Result<()> {
        self.write_u128(value)
    }

    fn serialize_f32(self, value: f32) -> Result<()> {
        self.write_bytes(&value.to_le_bytes())
    }

    fn serialize_f64(self, value: f64) -> Result<()> {
        self.write_bytes(&value.to_le_bytes())
    }

    fn serialize_char(self, value: char) -> Result<()> {
        self.write_u128(value as u32 as u128)
    }

    fn serialize_str(self, value: &str) -> Result<()> {
        self.write_byte_slice(value.as_bytes())
    }

    fn serialize_bytes(self, value: &[u8]) -> Result<()> {
        self.write_byte_slice(value)
    }

    fn serialize_none(self) -> Result<()> {
        self.write_byte(0)
    }

    fn serialize_some<T>(self, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.write_byte(1)?;
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<()> {
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        Ok(())
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
    ) -> Result<()> {
        self.write_u128(variant_index as u128)
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.write_u128(variant_index as u128)?;
        value.serialize(self)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        let len = len.ok_or(Error::SequenceLengthRequired)?;
        self.write_usize(len)?;

        Ok(SequenceEncoder { encoder: self })
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Ok(SequenceEncoder { encoder: self })
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Ok(SequenceEncoder { encoder: self })
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        self.write_u128(variant_index as u128)?;

        Ok(SequenceEncoder { encoder: self })
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        Ok(MapEncoder {
            encoder: self,
            pending_key: None,
            entries: Vec::with_capacity(len.unwrap_or(0)),
        })
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        Ok(SequenceEncoder { encoder: self })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        self.write_u128(variant_index as u128)?;

        Ok(SequenceEncoder { encoder: self })
    }
}

/// Encoder for ordered field and element sequences.
#[derive(Debug)]
struct SequenceEncoder<'a, 'output> {
    /// Underlying encoder.
    encoder: &'a mut Encoder<'output>,
}

impl SerializeSeq for SequenceEncoder<'_, '_> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(&mut *self.encoder)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl SerializeTuple for SequenceEncoder<'_, '_> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(&mut *self.encoder)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl SerializeTupleStruct for SequenceEncoder<'_, '_> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(&mut *self.encoder)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl SerializeTupleVariant for SequenceEncoder<'_, '_> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(&mut *self.encoder)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl SerializeStruct for SequenceEncoder<'_, '_> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(&mut *self.encoder)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl SerializeStructVariant for SequenceEncoder<'_, '_> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(&mut *self.encoder)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

/// Encoder for canonical maps.
#[derive(Debug)]
struct MapEncoder<'a, 'output> {
    /// Underlying encoder.
    encoder: &'a mut Encoder<'output>,
    /// Key waiting for its value.
    pending_key: Option<Vec<u8>>,
    /// Encoded key value pairs.
    entries: Vec<(Vec<u8>, Vec<u8>)>,
}

impl SerializeMap for MapEncoder<'_, '_> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        if self.pending_key.is_some() {
            return Err(Error::MapKeyWithoutValue);
        }

        self.pending_key = Some(Encoder::nested_bytes(key)?);

        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        let Some(key) = self.pending_key.take() else {
            return Err(Error::MapValueWithoutKey);
        };
        let value = Encoder::nested_bytes(value)?;
        self.entries.push((key, value));

        Ok(())
    }

    fn end(mut self) -> Result<()> {
        if self.pending_key.is_some() {
            return Err(Error::MapEndedWithPendingKey);
        }
        self.entries.sort_by(|left, right| left.0.cmp(&right.0));
        self.encoder.write_usize(self.entries.len())?;
        for (key, value) in self.entries {
            self.encoder.write_bytes(&key)?;
            self.encoder.write_bytes(&value)?;
        }

        Ok(())
    }
}
