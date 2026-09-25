use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    Error, Field, Name, Payload, Reflect, Schema, Type, Value, Variant, encoded_len, from_slice,
    from_slice_fixed, to_slice, to_vec, to_vec_fixed,
};

/// Example value used by roundtrip tests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Example {
    /// Numeric field.
    number: i64,
    /// Optional field.
    text: Option<String>,
    /// Enum field.
    variant: ExampleVariant,
    /// Map field.
    map: BTreeMap<String, u32>,
}

/// Example enum used by roundtrip tests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum ExampleVariant {
    /// Unit variant.
    Unit,
    /// Tuple variant.
    Tuple(bool, u8),
    /// Struct variant.
    Struct { value: String },
}

/// Child item used by schema tests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
struct SchemaChild {
    /// Child value.
    value: String,
}

/// Parent item used by schema tests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
struct SchemaParent {
    /// Child field.
    child: SchemaChild,
    /// String sequence field.
    labels: Vec<String>,
    /// Map field.
    index: BTreeMap<String, u32>,
}

/// Reflect item with an omitted internal field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
struct SchemaSkippedField {
    /// Serialized field.
    visible: String,
    /// Internal field.
    #[serde(skip)]
    hidden: String,
}

/// Reflect item with renamed serialized members.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
struct SchemaRenamed {
    /// Renamed field.
    #[serde(rename = "publicField")]
    field: String,
    /// Renamed choice.
    choice: SchemaRenamedChoice,
}

/// Reflect enum with a renamed serialized variant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
enum SchemaRenamedChoice {
    /// Renamed variant.
    #[serde(rename = "publicVariant")]
    Variant,
}

/// Enum item used by schema tests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
enum SchemaChoice {
    /// Empty choice.
    Empty,
    /// Child choice.
    Child(SchemaChild),
    /// Named choice.
    Pair {
        /// Left value.
        left: u32,
        /// Right value.
        right: Option<String>,
    },
}

/// Internal payload without schema support.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SchemaInternalPayload;

/// Enum schema item with an omitted internal variant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
enum SchemaSkippedChoice {
    /// Visible choice.
    Visible,
    /// Internal choice.
    #[serde(skip)]
    Hidden(SchemaInternalPayload),
}

/// Schema item with an explicit public module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
#[reflect(module = "tspp_serde::public")]
struct SchemaPublicModule {
    /// Visible value.
    value: String,
}

/// Newtype item used by schema tests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
struct SchemaNewtype(String);

/// Tuple item used by schema tests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
struct SchemaTuple(u32, String);

/// Transparent item used by schema tests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
#[serde(transparent)]
struct SchemaTransparent {
    /// Serialized value.
    value: String,
}

/// Internally tagged enum used by codec compatibility tests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
enum TaggedChoice {
    /// Empty choice.
    Empty,
    /// Named choice.
    Pair {
        /// Left value.
        left: u32,
        /// Right value.
        right: String,
    },
}

#[test]
fn test_roundtrip_struct() {
    let mut map = BTreeMap::new();
    map.insert("b".to_string(), 2);
    map.insert("a".to_string(), 1);
    let value = Example {
        number: -42,
        text: Some("hello".to_string()),
        variant: ExampleVariant::Struct {
            value: "world".to_string(),
        },
        map,
    };

    let bytes = to_vec(&value).expect("encode");
    let decoded = from_slice::<Example>(&bytes).expect("decode");
    let fixed = to_vec_fixed(&value).expect("encode fixed-width value");
    let fixed = from_slice_fixed::<Example>(&fixed).expect("decode fixed-width value");

    assert_eq!(decoded, value);
    assert_eq!(fixed, value);
}

#[test]
fn test_roundtrip_value() {
    let value = Value::Object(vec![
        ("signed".to_string(), Value::Signed(i128::MIN)),
        ("unsigned".to_string(), Value::Unsigned(u128::MAX)),
        (
            "array".to_string(),
            Value::Array(vec![Value::Null, Value::Bool(true), Value::Float(1.5)]),
        ),
    ]);

    let bytes = to_vec(&value).expect("encode value");
    let decoded = from_slice::<Value>(&bytes).expect("decode value");

    assert_eq!(decoded, value);
}

#[test]
fn test_decode_rejects_internally_tagged_enum() {
    let value = TaggedChoice::Pair {
        left: 4,
        right: "value".to_string(),
    };

    let bytes = to_vec(&value).expect("encode");
    let error = from_slice::<TaggedChoice>(&bytes).expect_err("decode should fail");

    assert_eq!(error, Error::SelfDescribingUnsupported);
}

#[test]
fn test_build_schema_from_derive() {
    let mut schema = Schema::default();
    schema.register::<SchemaParent>();
    schema.register::<SchemaChoice>();

    let parent = Name::new(module_path!(), "SchemaParent");
    let parent = schema.items.get(&parent).expect("parent schema item");

    let Type::Struct(fields) = &parent.ty else {
        panic!("parent should be a struct");
    };

    // inspect a nested named field
    let child = &fields[0];
    let Type::Named(child_name) = &child.ty else {
        panic!("child field should be named");
    };
    assert_eq!(child.name, "child");
    assert_eq!(child_name.name, "SchemaChild");

    // inspect a collection field
    let labels = &fields[1];
    assert_eq!(labels.name, "labels");
    assert_eq!(labels.ty, Type::Sequence(Box::new(Type::String)));

    // inspect a map field
    let index = &fields[2];
    assert_eq!(index.name, "index");
    assert_eq!(
        index.ty,
        Type::Map {
            key: Box::new(Type::String),
            value: Box::new(Type::Unsigned { bits: 32 }),
        }
    );
}

#[test]
fn test_build_schema_uses_explicit_module() {
    let mut schema = Schema::default();
    schema.register::<SchemaPublicModule>();

    let name = Name::new("tspp_serde::public", "SchemaPublicModule");
    let item = schema.items.get(&name).expect("schema item");

    assert_eq!(item.name, name);
}

#[test]
fn test_build_schema_preserves_struct_representations() {
    let mut schema = Schema::default();
    schema.register::<SchemaNewtype>();
    schema.register::<SchemaTuple>();
    schema.register::<SchemaTransparent>();

    let newtype = Name::new(module_path!(), "SchemaNewtype");
    let tuple = Name::new(module_path!(), "SchemaTuple");
    let transparent = Name::new(module_path!(), "SchemaTransparent");

    assert_eq!(schema.items[&newtype].ty, Type::String);
    assert_eq!(
        schema.items[&tuple].ty,
        Type::Tuple(vec![Type::Unsigned { bits: 32 }, Type::String])
    );
    assert_eq!(schema.items[&transparent].ty, Type::String);
}

#[test]
fn test_build_schema_omits_skipped_fields() {
    let mut schema = Schema::default();
    schema.register::<SchemaSkippedField>();

    let item = Name::new(module_path!(), "SchemaSkippedField");
    let item = schema.items.get(&item).expect("skipped field schema item");

    let Type::Struct(fields) = &item.ty else {
        panic!("skipped field item should be a struct");
    };

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].name, "visible");
}

#[test]
fn test_build_schema_preserves_serde_names() {
    let mut schema = Schema::default();
    schema.register::<SchemaRenamed>();

    let item = Name::new(module_path!(), "SchemaRenamed");
    let choice = Name::new(module_path!(), "SchemaRenamedChoice");

    assert_eq!(
        schema.items[&item].ty,
        Type::Struct(vec![
            Field {
                name: "publicField".to_string(),
                docs: vec!["Renamed field.".to_string()],
                ty: Type::String,
            },
            Field {
                name: "choice".to_string(),
                docs: vec!["Renamed choice.".to_string()],
                ty: Type::Named(choice.clone()),
            },
        ])
    );
    assert_eq!(
        schema.items[&choice].ty,
        Type::Enum(vec![Variant {
            name: "publicVariant".to_string(),
            docs: vec!["Renamed variant.".to_string()],
            payload: Payload::Unit,
        }])
    );
}

#[test]
fn test_build_schema_omits_skipped_variants() {
    let mut schema = Schema::default();
    schema.register::<SchemaSkippedChoice>();
    let _internal = SchemaSkippedChoice::Hidden(SchemaInternalPayload);

    let item = Name::new(module_path!(), "SchemaSkippedChoice");
    let item = schema
        .items
        .get(&item)
        .expect("skipped variant schema item");

    let Type::Enum(variants) = &item.ty else {
        panic!("skipped variant item should be an enum");
    };

    assert_eq!(variants.len(), 1);
    assert_eq!(variants[0].name, "Visible");
}

#[test]
fn test_include_moves_explicit_schema_to_module_end() {
    let mut schema = Schema::default();
    schema.register::<SchemaParent>();
    schema.register::<SchemaChild>();

    let module = Name::new(module_path!(), "SchemaParent").module;
    let names = schema.modules.get(&module).expect("schema module");
    let names = names
        .iter()
        .map(|name| name.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(names, vec!["SchemaParent", "SchemaChild"]);
}

#[test]
fn test_build_enum_schema_from_derive() {
    let mut schema = Schema::default();
    schema.register::<SchemaChoice>();

    let choice = Name::new(module_path!(), "SchemaChoice");
    let choice = schema.items.get(&choice).expect("choice schema item");

    let Type::Enum(variants) = &choice.ty else {
        panic!("choice should be an enum");
    };

    // inspect each supported payload form
    assert_eq!(variants[0].name, "Empty");
    assert_eq!(variants[0].payload, Payload::Unit);

    let Payload::Value(Type::Named(child)) = &variants[1].payload else {
        panic!("child choice should be a tuple payload");
    };
    assert_eq!(child.name, "SchemaChild");

    let Payload::Struct(fields) = &variants[2].payload else {
        panic!("pair choice should be a struct payload");
    };
    assert_eq!(fields[0].name, "left");
    assert_eq!(fields[1].name, "right");
}

#[test]
fn test_encode_maps_canonically() {
    let mut left = BTreeMap::new();
    left.insert("a".to_string(), 1u32);
    left.insert("b".to_string(), 2u32);
    let mut right = BTreeMap::new();
    right.insert("b".to_string(), 2u32);
    right.insert("a".to_string(), 1u32);

    let left = to_vec(&left).expect("encode left");
    let right = to_vec(&right).expect("encode right");

    assert_eq!(left, right);
}

#[test]
fn test_encode_slice_matches_vec() {
    let value = Example {
        number: -7,
        text: Some("slice".to_string()),
        variant: ExampleVariant::Tuple(true, 3),
        map: BTreeMap::new(),
    };
    let expected = to_vec(&value).expect("encode vec");
    let expected_len = encoded_len(&value).expect("measure");
    let mut output = vec![0; expected_len];

    let encoded = to_slice(&value, &mut output).expect("encode slice");

    assert_eq!(expected_len, expected.len());
    assert_eq!(encoded, expected);
}

#[test]
fn test_encode_slice_rejects_small_output() {
    let value = Example {
        number: 42,
        text: Some("slice".to_string()),
        variant: ExampleVariant::Unit,
        map: BTreeMap::new(),
    };
    let expected_len = encoded_len(&value).expect("measure");
    let mut output = vec![0; expected_len - 1];

    let error = to_slice(&value, &mut output).expect_err("encode should fail");

    assert_eq!(error, Error::BufferTooSmall);
}

#[test]
fn test_encode_byte_scalars_as_one_byte() {
    let signed = to_vec(&-1i8).expect("encode signed");
    let unsigned = to_vec(&255u8).expect("encode unsigned");

    assert_eq!(signed, vec![255]);
    assert_eq!(unsigned, vec![255]);
    assert_eq!(from_slice::<i8>(&signed).expect("decode signed"), -1);
    assert_eq!(from_slice::<u8>(&unsigned).expect("decode unsigned"), 255);
}

#[test]
fn test_encode_varint_scalars_canonically() {
    let positive = to_vec(&128u16).expect("encode positive");
    let negative = to_vec(&-1i16).expect("encode negative");

    assert_eq!(positive, vec![0x80, 0x01]);
    assert_eq!(negative, vec![0x01]);
    assert_eq!(from_slice::<u16>(&positive).expect("decode positive"), 128);
    assert_eq!(from_slice::<i16>(&negative).expect("decode negative"), -1);
}

#[test]
fn test_decode_rejects_non_canonical_varint() {
    let error = from_slice::<u16>(&[0x80, 0x00]).expect_err("decode should fail");

    assert_eq!(error, Error::NonCanonicalVarint);
}

#[test]
fn test_decode_rejects_oversized_varint() {
    let bytes = [0xff; 19];
    let error = from_slice::<u128>(&bytes).expect_err("decode should fail");

    assert_eq!(error, Error::VarintTooLarge);
}

#[test]
fn test_decode_borrows_strings() {
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Borrowed<'a> {
        /// Borrowed text field.
        text: &'a str,
    }

    let value = Borrowed { text: "borrowed" };
    let bytes = to_vec(&value).expect("encode");
    let decoded = from_slice::<Borrowed<'_>>(&bytes).expect("decode");

    assert_eq!(decoded, value);
}
