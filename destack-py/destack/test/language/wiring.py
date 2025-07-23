from hypothesis import HealthCheck, given, settings

from destack.language import (
    BuiltinObject,
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
from destack.language.core.builtin.const import ENCODERS
from destack.test.strategies import builtin_objects
from destack.utils.uuid import uuid4


def _test_roundtrip_object(obj: BuiltinObject, session: Session):
    for _, encoder in ENCODERS.items():
        # pack/unpack
        packed_obj = encoder.pack_object(obj.__kind__, obj.metatype, obj)
        packed_obj_bytes = encoder.pack_object_bytes(obj.__kind__, obj.metatype, obj)
        unpacked_obj = encoder.unpack_object(obj.__kind__, obj.metatype, packed_obj, session)
        assert unpacked_obj.equals(obj), f"{unpacked_obj!r} != {obj!r}"
        assert unpacked_obj.hash() == obj.hash(), f"{unpacked_obj.hash()} != {obj.hash()}"

        # pack/unpack as bytes
        packed_obj_bytes = encoder.pack_object_bytes(obj.__kind__, obj.metatype, obj)
        unpacked_obj_bytes = encoder.unpack_object_bytes(
            obj.__kind__, obj.metatype, packed_obj_bytes, session
        )
        assert unpacked_obj_bytes.equals(obj), f"{unpacked_obj_bytes!r} != {obj!r}"
        assert unpacked_obj_bytes.hash() == obj.hash(), (
            f"{unpacked_obj_bytes.hash()} != {obj.hash()}"
        )


def test_roundtrip_node_reference(session: Session, space: Space):
    """Pack and unpack a NodeReference."""
    node_ref = NodeReference(
        type=NodeType.FOLDER,
        id=uuid4(),
        space_id=space.id,
        branch_id=space.branch_ptr.id,
        snapshot_id=space.snapshot_ptr.id,
    )
    _test_roundtrip_object(node_ref, session)


def test_roundtrip_query(session: Session, space: Space):
    """Pack and unpack a Query."""
    query = Folder.search(
        sort=[Folder.property("created_at").asc()],
        limit=25,
        Subfolder=Folder.get(
            join=Join.of(JoinType.LEFT, on=Folder.property("created_epoch").eq(5)),
        ),
    )
    _test_roundtrip_object(query, session)


def test_roundtrip_user(session: Session, space: Space):
    """Pack and unpack a User."""
    user = User(
        status=UserStatus.ACTIVE,
        name="Florian",
        slug="florian",
        space_ptr=space.to_ref(),
    )
    _test_roundtrip_object(user, session)


@given(obj=builtin_objects())
@settings(suppress_health_check=[HealthCheck.function_scoped_fixture])
def test_roundtrip_builtin_object(obj: BuiltinObject, session: Session, space: Space):
    _test_roundtrip_object(obj, session)
