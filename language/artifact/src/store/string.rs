use destack_core::StringId;
use serde::Serialize;
use serde::ser::{
    self, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
    SerializeTupleStruct, SerializeTupleVariant,
};

use crate::ArtifactStoreError;

/// Collect interned string ids referenced by one serializable payload.
pub(super) fn collect_string_ids<T: Serialize>(
    payload: &T,
) -> Result<Vec<StringId>, ArtifactStoreError> {
    let mut collector = StringIdCollector::default();
    payload.serialize(&mut collector)?;

    Ok(collector.strings)
}

/// Serde serializer that records `StringId` newtype values.
#[derive(Debug, Default)]
struct StringIdCollector {
    /// Collected string ids.
    strings: Vec<StringId>,
    /// Whether the next `u128` belongs to a `StringId` newtype.
    is_string_id: bool,
}

/// Compound serializer that recurses through aggregate values.
struct StringIdCompound<'a> {
    /// Active string id collector.
    collector: &'a mut StringIdCollector,
}

impl<'a> ser::Serializer for &'a mut StringIdCollector {
    type Ok = ();
    type Error = ArtifactStoreError;
    type SerializeSeq = StringIdCompound<'a>;
    type SerializeTuple = StringIdCompound<'a>;
    type SerializeTupleStruct = StringIdCompound<'a>;
    type SerializeTupleVariant = StringIdCompound<'a>;
    type SerializeMap = StringIdCompound<'a>;
    type SerializeStruct = StringIdCompound<'a>;
    type SerializeStructVariant = StringIdCompound<'a>;

    fn serialize_bool(self, _value: bool) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_i8(self, _value: i8) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_i16(self, _value: i16) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_i32(self, _value: i32) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_i64(self, _value: i64) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_i128(self, _value: i128) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_u8(self, _value: u8) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_u16(self, _value: u16) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_u32(self, _value: u32) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_u64(self, _value: u64) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_f32(self, _value: f32) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_f64(self, _value: f64) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_char(self, _value: char) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_str(self, _value: &str) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_bytes(self, _value: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_u128(self, value: u128) -> Result<Self::Ok, Self::Error> {
        if self.is_string_id {
            self.strings.push(StringId(value));
        }

        Ok(())
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        let was_string_id = self.is_string_id;
        let string_count = self.strings.len();
        self.is_string_id = name == "StringId";
        let result = value.serialize(&mut *self);
        self.is_string_id = was_string_id;
        result?;

        if name == "StringId" && self.strings.len() == string_count {
            return Err(ArtifactStoreError::Corrupt(
                "StringId did not serialize as u128",
            ));
        }

        Ok(())
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(StringIdCompound { collector: self })
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(StringIdCompound { collector: self })
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(StringIdCompound { collector: self })
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(StringIdCompound { collector: self })
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(StringIdCompound { collector: self })
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(StringIdCompound { collector: self })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(StringIdCompound { collector: self })
    }
}

impl<'a> SerializeSeq for StringIdCompound<'a> {
    type Ok = ();
    type Error = ArtifactStoreError;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        value.serialize(&mut *self.collector)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a> SerializeTuple for StringIdCompound<'a> {
    type Ok = ();
    type Error = ArtifactStoreError;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        value.serialize(&mut *self.collector)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a> SerializeTupleStruct for StringIdCompound<'a> {
    type Ok = ();
    type Error = ArtifactStoreError;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        value.serialize(&mut *self.collector)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a> SerializeTupleVariant for StringIdCompound<'a> {
    type Ok = ();
    type Error = ArtifactStoreError;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        value.serialize(&mut *self.collector)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a> SerializeMap for StringIdCompound<'a> {
    type Ok = ();
    type Error = ArtifactStoreError;

    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<(), Self::Error> {
        key.serialize(&mut *self.collector)
    }

    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        value.serialize(&mut *self.collector)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a> SerializeStruct for StringIdCompound<'a> {
    type Ok = ();
    type Error = ArtifactStoreError;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        _key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        value.serialize(&mut *self.collector)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a> SerializeStructVariant for StringIdCompound<'a> {
    type Ok = ();
    type Error = ArtifactStoreError;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        _key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        value.serialize(&mut *self.collector)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}
