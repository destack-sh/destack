import json

from destack.language import (
    ENCODERS,
    BinaryReader,
    BinaryWriter,
    BuiltinObject,
    Encoder,
    Encoding,
    Folder,
    Join,
    JoinType,
    NodeReference,
    NodeType,
    Session,
    Space,
    User,
    UserStatus,
)
from destack.utils.uuid import uuid4


def _do_test_roundtrip_object(
    obj: BuiltinObject, session: Session, encoder: Encoder, encoding: Encoding
) -> None:
    if encoding == Encoding.KOMPAKT:
        return  # nocheckin

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

    print(repr(obj))
    print(json.dumps(packed_obj, indent=2))
    print(len(packed_obj_bytes))


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


def test_roundtrip_query(session: Session, space: Space):
    """Pack and unpack a Query."""
    query = Folder.search(
        sort=[Folder.property("created_at").asc()],
        limit=25,
        Subfolder=Folder.get(
            join=Join.of(JoinType.LEFT, on=Folder.property("created_epoch").eq(5)),
        ),
    )
    for encoding, encoder in ENCODERS.items():
        _ = _do_test_roundtrip_object(query, session, encoder, encoding)


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
