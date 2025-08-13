import gzip
from typing import Any, Callable

from destack import (
    BinaryReader,
    BinaryWriter,
    Encoder,
    Encoding,
    Form2D,
    NodeSpatialReference,
    NodeType,
    Object,
    Rectangle2D,
    Type,
    Value,
    Vector3,
    uuid7,
)
from destack.core.encoding.registry import get_encoder

from .conftest import wrap_binary_reader, wrap_binary_writer, wrap_encoder

# ruff: noqa: T201

_LOG_ENCODE = False
_LOG_RESULT = True

ENCODERS = {encoding: get_encoder(encoding) for encoding in Encoding}
_make_reader: Callable[[bytes], BinaryReader]
_make_writer: Callable[[], BinaryWriter]
if _LOG_ENCODE:
    ENCODERS = {encoding: wrap_encoder(encoder) for encoding, encoder in ENCODERS.items()}
    _make_writer = lambda: wrap_binary_writer(BinaryWriter())
    _make_reader = lambda buffer: wrap_binary_reader(BinaryReader(buffer=buffer))
else:
    _make_writer = BinaryWriter
    _make_reader = lambda buffer: BinaryReader(buffer=buffer)


def _do_test_roundtrip_object(obj: Object, encoder: Encoder, encoding: Encoding) -> bytes:
    if _LOG_RESULT:
        print("=" * 80)
        print(repr(obj))
        print(f" -> {encoding.name}")
        print("-" * 80)

    writer = _make_writer()
    encoder.pack_object_binary(obj, writer)
    packed_obj_bytes = writer.to_bytes()
    packed_obj_bytes_gzip = gzip.compress(packed_obj_bytes)
    if _LOG_RESULT:
        print(f"-> BYTES: {len(packed_obj_bytes)} ({len(packed_obj_bytes_gzip)} gzip)")
        print(f"-> HASH: {obj.hash()}")
        print("-" * 80)

    reader = _make_reader(packed_obj_bytes)
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

    writer = _make_writer()
    encoder.pack_value_binary(type, value, writer)
    packed_value_bytes = writer.to_bytes()
    packed_value_bytes_gzip = gzip.compress(packed_value_bytes)
    if _LOG_RESULT:
        print(f"-> BYTES: {len(packed_value_bytes)} ({len(packed_value_bytes_gzip)} gzip)")
        print("-" * 80)

    reader = _make_reader(packed_value_bytes)
    unpacked_value_bytes = encoder.unpack_value_binary(type, reader, None)
    assert unpacked_value_bytes == value, f"{unpacked_value_bytes!r} != {value!r}"
    if _LOG_RESULT:
        print(repr(unpacked_value_bytes))
    return packed_value_bytes


def test_roundtrip_node_locations():
    """Pack and unpack a NodeSpatial."""
    node_ref = NodeSpatialReference(
        type=NodeType.FOLDER,
        id=uuid7(),
        space_id=uuid7(),
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
        Type.of(bool),
        Type.of(17),
        Type.of(dict[str, Value]),
        Type.of(int | str),
        Type.of([Vector3(x=1.0, y=2.0, z=3.0)]),
    ):
        for encoding, encoder in ENCODERS.items():
            _ = _do_test_roundtrip_object(type, encoder, encoding)


def test_roundtrip_vector3_list():
    """Pack and unpack a Vector3 list."""
    vectors = [Vector3(x=i * 0.1, y=i * 0.2, z=i * 0.3) for i in range(3)]
    type = Type.of(vectors)
    for encoding, encoder in ENCODERS.items():
        _ = _do_test_roundtrip_value(type, vectors, encoder, encoding)


def test_roundtrip_vector3_list_compact():
    """Pack and unpack a Vector3 list with Kompakt."""
    num_vectors = 10
    vectors = [Vector3(x=i * 0.1, y=i * 0.2, z=i * 0.3) for i in range(num_vectors)]
    type = Type.of(vectors)
    vectors_bytes = _do_test_roundtrip_value(
        type, vectors, ENCODERS[Encoding.KOMPAKT], Encoding.KOMPAKT
    )
    # num vectors * 3 floats * 4 bytes per float
    target_size = num_vectors * 3 * 4 + 1  # for array length varint
    assert len(vectors_bytes) <= target_size


def test_roundtrip_value():
    """Pack and unpack a Value."""
    for value in (
        Value.of(1),
        Value.of(Vector3(x=1.0, y=2.0, z=3.0)),
        Value.of((2, True, "Hello")),
        Value.of(Rectangle2D(width=1.0, height=2.0)),
        Value.of(Rectangle2D(width=1.0, height=2.0), type=Type.of(Form2D)),
    ):
        for encoding, encoder in ENCODERS.items():
            _ = _do_test_roundtrip_object(value, encoder, encoding)
            _ = _do_test_roundtrip_value(value.type, value.value, encoder, encoding)
