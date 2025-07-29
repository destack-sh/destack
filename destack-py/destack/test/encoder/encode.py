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
    Vector3,
)
from destack.utils.uuid import uuid4


def _do_test_roundtrip_object(
    obj: Object, session: Session, encoder: Encoder, encoding: Encoding
) -> None:
    # pack/unpack
    packed_obj = encoder.pack_object(obj.__kind__, obj.metatype, obj, Encoder.TAGGED)
    writer = BinaryWriter()
    packed_obj_bytes = encoder.pack_object_binary(
        obj.__kind__, obj.metatype, obj, writer, Encoder.TAGGED
    )
    unpacked_obj = encoder.unpack_object(
        obj.__kind__, obj.metatype, packed_obj, session, Encoder.TAGGED
    )
    assert unpacked_obj.equals(obj), f"{unpacked_obj!r} != {obj!r}"
    assert unpacked_obj.hash() == obj.hash(), f"{unpacked_obj.hash()} != {obj.hash()}"

    # pack/unpack as bytes
    writer = BinaryWriter()
    encoder.pack_object_binary(obj.__kind__, obj.metatype, obj, writer, Encoder.TAGGED)
    packed_obj_bytes = writer.to_bytes()
    reader = BinaryReader(packed_obj_bytes)
    unpacked_obj_bytes = encoder.unpack_object_binary(
        obj.__kind__, obj.metatype, reader, session, Encoder.TAGGED
    )
    assert unpacked_obj_bytes.equals(obj), f"{unpacked_obj_bytes!r} != {obj!r}"
    assert unpacked_obj_bytes.hash() == obj.hash(), f"{unpacked_obj_bytes.hash()} != {obj.hash()}"

    print(repr(obj))  # noqa: T201
    print(len(packed_obj_bytes))  # noqa: T201
    if encoding == Encoding.KOMPAKT:
        print(packed_obj_bytes.hex(sep=" "))  # noqa: T201


def _do_test_roundtrip_value(
    type: Type, value: Any, session: Session, encoder: Encoder, encoding: Encoding
) -> None:
    # pack/unpack as bytes
    writer = BinaryWriter()
    encoder.pack_value_binary(type, value, writer, Encoder.TAGGED)
    packed_value_bytes = writer.to_bytes()
    reader = BinaryReader(packed_value_bytes)
    unpacked_value_bytes = encoder.unpack_value_binary(type, reader, session, Encoder.TAGGED)
    assert unpacked_value_bytes == value, f"{unpacked_value_bytes!r} != {value!r}"

    print(repr(value))  # noqa: T201
    print(len(packed_value_bytes))  # noqa: T201
    if encoding == Encoding.KOMPAKT:
        print(packed_value_bytes.hex(sep=" "))  # noqa: T201


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


def test_roundtrip_vector3_list(session: Session, space: Space):
    """Pack and unpack a Vector3."""
    vectors = [Vector3(x=i * 0.1, y=i * 0.2, z=i * 0.3) for i in range(100)]
    type = Type.infer(vectors)
    for encoding, encoder in ENCODERS.items():
        _ = _do_test_roundtrip_value(type, vectors, session, encoder, encoding)


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
