import gzip
from typing import Any

from destack import (
    BinaryReader,
    BinaryWriter,
    Encoder,
    Encoding,
    Form2D,
    NodeReference,
    NodeType,
    Object,
    Rectangle2D,
    Type,
    Value,
    Vector3,
    uuid4,
)
from destack.core.encoding.registry import get_encoder

from .conftest import LoggingBinaryReader, LoggingBinaryWriter, wrap_encoder

# ruff: noqa: T201

_LOG_ENCODE = False
_LOG_RESULT = True

ENCODERS = {encoding: get_encoder(encoding) for encoding in Encoding}
if _LOG_ENCODE:
    ENCODERS = {encoding: wrap_encoder(encoder) for encoding, encoder in ENCODERS.items()}
    _BinaryWriter = LoggingBinaryWriter
    _BinaryReader = LoggingBinaryReader
else:
    _BinaryWriter = BinaryWriter
    _BinaryReader = BinaryReader


def _do_test_roundtrip_object(obj: Object, encoder: Encoder, encoding: Encoding) -> bytes:
    if _LOG_RESULT:
        print("=" * 80)
        print(repr(obj))
        print(f" -> {encoding.name}")
        print("-" * 80)

    writer = _BinaryWriter()
    encoder.pack_object_binary(obj, writer)
    packed_obj_bytes = writer.to_bytes()
    packed_obj_bytes_gzip = gzip.compress(packed_obj_bytes)
    if _LOG_RESULT:
        print(f"-> BYTES: {len(packed_obj_bytes)} ({len(packed_obj_bytes_gzip)} gzip)")
        print(f"-> HASH: {obj.hash()}")
        print("-" * 80)

    reader = _BinaryReader(buffer=packed_obj_bytes)
    unpacked_obj = encoder.unpack_object_binary(None, None, reader, None)
    assert unpacked_obj.equals(obj), f"{unpacked_obj!r} != {obj!r}"
    assert unpacked_obj.hash() == obj.hash(), f"{unpacked_obj.hash()} != {obj.hash()}"
    return packed_obj_bytes


def _do_test_roundtrip_value(type: Type, value: Any, encoder: Encoder, encoding: Encoding) -> bytes:
    if _LOG_RESULT:
        print("=" * 80)
        print(repr(value))
        print(f" -> {encoding.name}")
        print("-" * 80)

    writer = _BinaryWriter()
    encoder.pack_value_binary(type, value, writer)
    packed_value_bytes = writer.to_bytes()
    packed_value_bytes_gzip = gzip.compress(packed_value_bytes)
    if _LOG_RESULT:
        print(f"-> BYTES: {len(packed_value_bytes)} ({len(packed_value_bytes_gzip)} gzip)")
        print("-" * 80)

    reader = _BinaryReader(buffer=packed_value_bytes)
    unpacked_value_bytes = encoder.unpack_value_binary(type, reader, None)
    assert unpacked_value_bytes == value, f"{unpacked_value_bytes!r} != {value!r}"
    if _LOG_RESULT:
        print(repr(unpacked_value_bytes))
    return packed_value_bytes


def test_roundtrip_node_reference():
    """Pack and unpack a NodeReference."""
    node_ref = NodeReference(
        type=NodeType.FOLDER,
        id=uuid4(),
        space_id=uuid4(),
        branch_id=uuid4(),
        snapshot_id=uuid4(),
    )
    for encoding, encoder in ENCODERS.items():
        _ = _do_test_roundtrip_object(node_ref, encoder, encoding)


def test_roundtrip_vector3():
    """Pack and unpack a Vector3."""
    vector3 = Vector3(x=1.0, y=2.0, z=3.0)
    for encoding, encoder in ENCODERS.items():
        _ = _do_test_roundtrip_object(vector3, encoder, encoding)


def test_roundtrip_type():
    """Pack and unpack a Type."""
    for type in (
        Type.infer(bool),
        Type.infer(17),
        Type.infer(dict[str, Value]),
        Type.infer([Vector3(x=1.0, y=2.0, z=3.0)]),
    ):
        for encoding, encoder in ENCODERS.items():
            _ = _do_test_roundtrip_object(type, encoder, encoding)


def test_roundtrip_vector3_list():
    """Pack and unpack a Vector3 list."""
    vectors = [Vector3(x=i * 0.1, y=i * 0.2, z=i * 0.3) for i in range(3)]
    type = Type.infer(vectors)
    for encoding, encoder in ENCODERS.items():
        _ = _do_test_roundtrip_value(type, vectors, encoder, encoding)


def test_roundtrip_vector3_list_compact():
    """Pack and unpack a Vector3 list with Kompakt."""
    num_vectors = 10
    vectors = [Vector3(x=i * 0.1, y=i * 0.2, z=i * 0.3) for i in range(num_vectors)]
    type = Type.infer(vectors)
    vectors_bytes = _do_test_roundtrip_value(
        type, vectors, ENCODERS[Encoding.KOMPAKT], Encoding.KOMPAKT
    )
    # num vectors * 3 floats * 4 bytes per float
    target_size = num_vectors * 3 * 4 + 1  # for array length varint
    assert len(vectors_bytes) <= target_size


def test_roundtrip_value():
    """Pack and unpack a Value."""
    for value in (
        Value.wrap(1),
        Value.wrap(Vector3(x=1.0, y=2.0, z=3.0)),
        Value.wrap((2, True, "Hello")),
        Value.wrap(Rectangle2D(width=1.0, height=2.0)),
        Value.wrap(Rectangle2D(width=1.0, height=2.0), type=Type.infer(Form2D)),
    ):
        for encoding, encoder in ENCODERS.items():
            _ = _do_test_roundtrip_object(value, encoder, encoding)
            _ = _do_test_roundtrip_value(value.type, value.value, encoder, encoding)
