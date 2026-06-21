use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{Error, encoded_len, from_slice, to_slice, to_vec};

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
