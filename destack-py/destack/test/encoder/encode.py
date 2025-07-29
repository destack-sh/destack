from typing import Any

from destack.language import (
    ENCODERS,
    BinaryReader,
    BinaryWriter,
    Encoder,
    Encoding,
    Folder,
    Join,
    JoinType,
    NodeReference,
    NodeType,
    Object,
    Session,
    Space,
    Type,
    User,
    UserStatus,
    Value,
    Vector3,
)
from destack.utils.uuid import uuid4


def _do_test_roundtrip_object(
    obj: Object, session: Session, encoder: Encoder, encoding: Encoding
) -> bytes:
    # pack/unpack as bytes
    print("=" * 80)  # noqa: T201
    writer = BinaryWriter()
    encoder.pack_object_binary(obj.__kind__, obj.metatype, obj, writer)
    packed_obj_bytes = writer.to_bytes()
    print(repr(obj))  # noqa: T201
    print(f"bytes: {len(packed_obj_bytes)}")  # noqa: T201
    if encoding == Encoding.KOMPAKT:
        print(packed_obj_bytes.hex(sep=" "))  # noqa: T201
    reader = BinaryReader(packed_obj_bytes)
    unpacked_obj = encoder.unpack_object_binary(obj.__kind__, obj.metatype, reader, session)
    assert unpacked_obj.equals(obj), f"{unpacked_obj!r} != {obj!r}"
    assert unpacked_obj.hash() == obj.hash(), f"{unpacked_obj.hash()} != {obj.hash()}"
    print(repr(unpacked_obj))  # noqa: T201
    return packed_obj_bytes


def _do_test_roundtrip_value(
    type: Type, value: Any, session: Session, encoder: Encoder, encoding: Encoding
) -> bytes:
    # pack/unpack as bytes
    print("=" * 80)  # noqa: T201
    writer = BinaryWriter()
    encoder.pack_value_binary(type, value, writer)
    packed_value_bytes = writer.to_bytes()
    print(repr(value))  # noqa: T201
    print(f"bytes: {len(packed_value_bytes)}")  # noqa: T201
    if encoding == Encoding.KOMPAKT:
        print(packed_value_bytes.hex(sep=" "))  # noqa: T201
    reader = BinaryReader(packed_value_bytes)
    unpacked_value_bytes = encoder.unpack_value_binary(type, reader, session)
    assert unpacked_value_bytes == value, f"{unpacked_value_bytes!r} != {value!r}"
    print(repr(unpacked_value_bytes))  # noqa: T201
    return packed_value_bytes


def test_roundtrip_node_reference(session: Session, space: Space):
    """Pack and unpack a NodeReference."""
    node_ref = NodeReference(
        type=NodeType.FOLDER,
        id=uuid4(),
        space_id=space.id,
        branch_id=space.branch_ptr.id,
        snapshot_id=space.snapshot_ptr.id,
    )
    for encoding, encoder in ENCODERS.items():
        _ = _do_test_roundtrip_object(node_ref, session, encoder, encoding)


def test_roundtrip_vector3(session: Session, space: Space):
    """Pack and unpack a Vector3."""
    vector3 = Vector3(x=1.0, y=2.0, z=3.0)
    for encoding, encoder in ENCODERS.items():
        _ = _do_test_roundtrip_object(vector3, session, encoder, encoding)


def test_roundtrip_type(session: Session, space: Space):
    """Pack and unpack a Type."""
    for type in (
        Type.infer(bool),
        Type.infer(17),
        Type.infer([Vector3(x=1.0, y=2.0, z=3.0)]),
    ):
        for encoding, encoder in ENCODERS.items():
            _ = _do_test_roundtrip_object(type, session, encoder, encoding)


def test_roundtrip_vector3_list(session: Session, space: Space):
    """Pack and unpack a Vector3 list."""
    vectors = [Vector3(x=i * 0.1, y=i * 0.2, z=i * 0.3) for i in range(3)]
    type = Type.infer(vectors)
    for encoding, encoder in ENCODERS.items():
        _ = _do_test_roundtrip_value(type, vectors, session, encoder, encoding)


def test_roundtrip_vector3_list_compact(session: Session, space: Space):
    """Pack and unpack a Vector3 list with Kompakt."""
    num_vectors = 10
    vectors = [Vector3(x=i * 0.1, y=i * 0.2, z=i * 0.3) for i in range(num_vectors)]
    type = Type.infer(vectors)
    vectors_bytes = _do_test_roundtrip_value(
        type, vectors, session, ENCODERS[Encoding.KOMPAKT], Encoding.KOMPAKT
    )
    # num vectors * 3 floats * 4 bytes per float
    target_size = num_vectors * 3 * 4 + 1  # for array length varint
    assert len(vectors_bytes) <= target_size


def test_roundtrip_value(session: Session, space: Space):
    """Pack and unpack a Value."""
    for value in (
        Value.wrap(1),
        Value.wrap(Vector3(x=1.0, y=2.0, z=3.0)),
        Value.wrap((2, bool, "Hello")),
    ):
        for encoding, encoder in ENCODERS.items():
            _ = _do_test_roundtrip_object(value, session, encoder, encoding)


def test_roundtrip_query(session: Session, space: Space):
    """Pack and unpack a Query (with a custom Vlue)."""
    query = Folder.search(
        sort=[Folder.property("created_at").asc()],
        limit=25,
        Subfolder=Folder.get(
            join=Join.of(JoinType.LEFT, on=Folder.property("created_epoch").eq(5)),
        ),
    )
    for encoding, encoder in ENCODERS.items():
        _ = _do_test_roundtrip_object(query, session, encoder, encoding)


def test_roundtrip_struct_subclass(session: Session, space: Space):
    """Pack and unpack a Struct subclass."""

    ...


def test_roundtrip_custom_struct_instance(session: Session, space: Space):
    """Pack and unpack a custom Struct instance."""

    ...


def test_roundtrip_user(session: Session, space: Space):
    """Pack and unpack a User."""
    user = User(
        status=UserStatus.ACTIVE,
        name="Florian",
        slug="florian",
        space_ptr=space.to_ref(),
    )
    for encoding, encoder in ENCODERS.items():
        _ = _do_test_roundtrip_object(user, session, encoder, encoding)
