use std::ops::Range;

use serde::Serialize;
use serde::ser::{
    self, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
    SerializeTupleStruct, SerializeTupleVariant,
};

use crate::{Error, IntegerEncoding, Result};

/// Maximum number of bytes needed for one encoded u128 varint.
const U128_VARINT_MAX_BYTES: usize = 19;

/// Encode one value into canonical TS++ binary bytes.
pub fn to_vec<T>(value: &T) -> Result<Vec<u8>>
where
    T: Serialize + ?Sized,
{
    let mut encoder = Encoder::new_vec(IntegerEncoding::Compact);
    value.serialize(&mut encoder)?;

    Ok(encoder.into_bytes())
}

/// Encode one value with fixed-width integers.
pub fn to_vec_fixed<T>(value: &T) -> Result<Vec<u8>>
where
    T: Serialize + ?Sized,
{
    let mut encoder = Encoder::new_vec(IntegerEncoding::Fixed);
    value.serialize(&mut encoder)?;

    Ok(encoder.into_bytes())
}

/// Append one fixed-width value and report its named newtypes.
pub fn append_fixed<T>(
    value: &T,
    output: &mut Vec<u8>,
    visit: &mut dyn FnMut(&'static str, &[u8]) -> Result<()>,
) -> Result<()>
where
    T: Serialize + ?Sized,
{
    let mut encoder = Encoder::with_newtype_visitor(IntegerEncoding::Fixed, output, visit);
    value.serialize(&mut encoder)
}

/// Append one value to canonical TS++ binary bytes.
pub fn append_to_vec<T>(value: &T, output: &mut Vec<u8>) -> Result<()>
where
    T: Serialize + ?Sized,
{
    let mut encoder = Encoder::new_vec_ref(IntegerEncoding::Compact, output);
    value.serialize(&mut encoder)?;

    Ok(())
}

/// Return the encoded length of one value.
pub fn encoded_len<T>(value: &T) -> Result<usize>
where
    T: Serialize + ?Sized,
{
    let mut encoder = Encoder::new_count(IntegerEncoding::Compact);
    value.serialize(&mut encoder)?;

    Ok(encoder.len())
}

/// Encode one value into one hasher, without buffering bytes.
pub fn hash_into<T, H>(value: &T, hasher: &mut H) -> Result<()>
where
    T: Serialize + ?Sized,
    H: std::hash::Hasher,
{
    let mut encoder = Encoder::new_hasher(IntegerEncoding::Compact, hasher);
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
        let mut encoder = Encoder::new_slice(IntegerEncoding::Compact, output);
        value.serialize(&mut encoder)?;
        encoder.len()
    };

    Ok(&mut output[..len])
}

/// Stateful binary encoder.
struct Encoder<'output> {
    /// Integer representation written by this encoder.
    integers: IntegerEncoding,
    /// Encoded output destination.
    output: EncoderOutput<'output>,
}

/// Encoder output destination.
enum EncoderOutput<'output> {
    /// Growable byte vector.
    Vec(Vec<u8>),
    /// Growable byte vector with named newtype observation.
    Newtypes {
        /// Encoded bytes.
        bytes: &'output mut Vec<u8>,
        /// Named newtype visitor.
        visit: &'output mut dyn FnMut(&'static str, &[u8]) -> Result<()>,
    },
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
            EncoderOutput::Newtypes { .. } => "newtypes",
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
            EncoderOutput::Newtypes { .. } => {
                unreachable!("borrowed newtype encoders cannot return owned bytes")
            }
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
            EncoderOutput::Newtypes { bytes, .. } => bytes.len(),
            EncoderOutput::VecRef(bytes) => bytes.len(),
            EncoderOutput::Slice { len, .. } => *len,
            EncoderOutput::Count(len) => *len,
        }
    }

    /// Create a new growable byte encoder.
    fn new_vec(integers: IntegerEncoding) -> Self {
        Self {
            integers,
            output: EncoderOutput::Vec(Vec::new()),
        }
    }

    /// Create a new growable encoder with named newtype observation.
    fn with_newtype_visitor<'borrow>(
        integers: IntegerEncoding,
        output: &'borrow mut Vec<u8>,
        visit: &'borrow mut dyn FnMut(&'static str, &[u8]) -> Result<()>,
    ) -> Encoder<'borrow> {
        Encoder {
            integers,
            output: EncoderOutput::Newtypes {
                bytes: output,
                visit,
            },
        }
    }

    /// Create a new borrowed growable byte encoder.
    fn new_vec_ref(integers: IntegerEncoding, output: &mut Vec<u8>) -> Encoder<'_> {
        Encoder {
            integers,
            output: EncoderOutput::VecRef(output),
        }
    }

    /// Create a new slice encoder.
    fn new_slice(integers: IntegerEncoding, output: &mut [u8]) -> Encoder<'_> {
        Encoder {
            integers,
            output: EncoderOutput::Slice {
                bytes: output,
                len: 0,
            },
        }
    }

    /// Create a new length-counting encoder.
    fn new_count(integers: IntegerEncoding) -> Self {
        Self {
            integers,
            output: EncoderOutput::Count(0),
        }
    }

    /// Create a new streaming hasher encoder.
    fn new_hasher(integers: IntegerEncoding, hasher: &mut dyn std::hash::Hasher) -> Encoder<'_> {
        Encoder {
            integers,
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
            EncoderOutput::Newtypes { bytes, .. } => {
                bytes.push(byte);

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
            EncoderOutput::Newtypes { bytes: output, .. } => {
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

    /// Write one usize.
    fn write_usize(&mut self, value: usize) -> Result<()> {
        match self.integers {
            IntegerEncoding::Compact => self.write_varint(value as u128),
            IntegerEncoding::Fixed => self.write_bytes(&(value as u64).to_le_bytes()),
        }
    }

    /// Write one unsigned integer using its declared width.
    fn write_unsigned<const N: usize>(&mut self, value: u128) -> Result<()> {
        match self.integers {
            IntegerEncoding::Compact => self.write_varint(value),
            IntegerEncoding::Fixed => self.write_bytes(&value.to_le_bytes()[..N]),
        }
    }

    /// Write one signed integer using its declared width.
    fn write_signed<const N: usize>(&mut self, value: i128) -> Result<()> {
        match self.integers {
            IntegerEncoding::Compact => {
                let encoded = ((value as u128) << 1) ^ ((value >> 127) as u128);

                self.write_varint(encoded)
            }
            IntegerEncoding::Fixed => self.write_bytes(&value.to_le_bytes()[..N]),
        }
    }

    /// Write one canonical unsigned varint.
    fn write_varint(&mut self, mut value: u128) -> Result<()> {
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

    /// Append one nested value to a caller-owned buffer.
    fn append_nested<T>(&mut self, value: &T, bytes: &mut Vec<u8>) -> Result<Range<usize>>
    where
        T: Serialize + ?Sized,
    {
        let start = bytes.len();
        match &mut self.output {
            EncoderOutput::Newtypes { visit, .. } => {
                let mut encoder = Encoder::with_newtype_visitor(self.integers, bytes, *visit);
                value.serialize(&mut encoder)?;
            }
            _ => {
                let mut encoder = Encoder::new_vec_ref(self.integers, bytes);
                value.serialize(&mut encoder)?;
            }
        }

        Ok(start..bytes.len())
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
        self.write_signed::<2>(value as i128)
    }

    fn serialize_i32(self, value: i32) -> Result<()> {
        self.write_signed::<4>(value as i128)
    }

    fn serialize_i64(self, value: i64) -> Result<()> {
        self.write_signed::<8>(value as i128)
    }

    fn serialize_i128(self, value: i128) -> Result<()> {
        self.write_signed::<16>(value)
    }

    fn serialize_u8(self, value: u8) -> Result<()> {
        self.write_byte(value)
    }

    fn serialize_u16(self, value: u16) -> Result<()> {
        self.write_unsigned::<2>(value as u128)
    }

    fn serialize_u32(self, value: u32) -> Result<()> {
        self.write_unsigned::<4>(value as u128)
    }

    fn serialize_u64(self, value: u64) -> Result<()> {
        self.write_unsigned::<8>(value as u128)
    }

    fn serialize_u128(self, value: u128) -> Result<()> {
        self.write_unsigned::<16>(value)
    }

    fn serialize_f32(self, value: f32) -> Result<()> {
        self.write_bytes(&value.to_le_bytes())
    }

    fn serialize_f64(self, value: f64) -> Result<()> {
        self.write_bytes(&value.to_le_bytes())
    }

    fn serialize_char(self, value: char) -> Result<()> {
        self.write_unsigned::<4>(value as u32 as u128)
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
        self.write_unsigned::<4>(variant_index as u128)
    }

    fn serialize_newtype_struct<T>(self, name: &'static str, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        let visits_newtypes = matches!(self.output, EncoderOutput::Newtypes { .. });
        if !visits_newtypes {
            return value.serialize(self);
        }

        // report the exact bytes appended by this named value
        let start = self.len();
        value.serialize(&mut *self)?;
        if let EncoderOutput::Newtypes { bytes, visit } = &mut self.output {
            visit(name, &bytes[start..])?;
        }

        Ok(())
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
        self.write_unsigned::<4>(variant_index as u128)?;
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
        self.write_unsigned::<4>(variant_index as u128)?;

        Ok(SequenceEncoder { encoder: self })
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        Ok(MapEncoder {
            encoder: self,
            pending_key: None,
            entries: Vec::with_capacity(len.unwrap_or(0)),
            bytes: Vec::new(),
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
        self.write_unsigned::<4>(variant_index as u128)?;

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
    pending_key: Option<Range<usize>>,
    /// Encoded key and value ranges.
    entries: Vec<EncodedMapEntry>,
    /// Contiguous encoded map bytes.
    bytes: Vec<u8>,
}

/// One encoded canonical map entry.
#[derive(Debug)]
struct EncodedMapEntry {
    /// Encoded key byte range.
    key: Range<usize>,
    /// Encoded value byte range.
    value: Range<usize>,
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

        self.pending_key = Some(self.encoder.append_nested(key, &mut self.bytes)?);

        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        let Some(key) = self.pending_key.take() else {
            return Err(Error::MapValueWithoutKey);
        };
        let value = self.encoder.append_nested(value, &mut self.bytes)?;
        self.entries.push(EncodedMapEntry { key, value });

        Ok(())
    }

    fn end(mut self) -> Result<()> {
        if self.pending_key.is_some() {
            return Err(Error::MapEndedWithPendingKey);
        }
        self.entries.sort_by(|left, right| {
            self.bytes[left.key.clone()].cmp(&self.bytes[right.key.clone()])
        });
        self.encoder.write_usize(self.entries.len())?;
        for entry in self.entries {
            self.encoder.write_bytes(&self.bytes[entry.key])?;
            self.encoder.write_bytes(&self.bytes[entry.value])?;
        }

        Ok(())
    }
}
