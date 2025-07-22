from hypothesis import HealthCheck, given, settings

from destack.language import (
    BuiltinObject,
    EventCursor,
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
        # pack/unpack as object
        packed_obj = encoder.pack_object(obj)
        packed_obj_bytes = encoder.pack_object_bytes(obj)
        unpacked_obj = encoder.unpack_object(
            packed_obj, session=session, graph=session.graph, connection=None
        )
        assert unpacked_obj.equals(obj), f"{unpacked_obj!r} != {obj!r}"
        assert unpacked_obj.hash() == obj.hash(), f"{unpacked_obj.hash()} != {obj.hash()}"

        # pack/unpack as bytes
        packed_obj_bytes = encoder.pack_object_bytes(obj)
        unpacked_obj_bytes = encoder.unpack_object_bytes(
            packed_obj_bytes, session=session, graph=session.graph, connection=None
        )
        assert unpacked_obj_bytes.equals(obj), f"{unpacked_obj_bytes!r} != {obj!r}"
        assert unpacked_obj_bytes.hash() == obj.hash(), (
            f"{unpacked_obj_bytes.hash()} != {obj.hash()}"
        )


def test_roundtrip_node_reference(session: Session, space: Space):
    """Pack and unpack a NodeReference."""
    node_ref = NodeReference(
        type=NodeType.FOLDER, id=uuid4(), space_id=uuid4(), definition_id=uuid4()
    )
    _test_roundtrip_object(node_ref, session)


def test_roundtrip_query(session: Session, space: Space):
    """Pack and unpack a Query."""
    query = Folder.search(
        sort=[Folder.property("created_at").asc()],
        limit=25,
        Cursor=EventCursor.get(
            join=Join.of(JoinType.LEFT, on=EventCursor.property("created_epoch").eq(5)),
        ),
    )
    _test_roundtrip_object(query, session)


def test_roundtrip_user(session: Session, space: Space):
    """Pack and unpack a User."""
    user = User(
        status=UserStatus.ACTIVE,
        name="Florian",
        slug="florian",
        space_ptr=NodeReference(id=uuid4(), type=NodeType.SPACE),
    )
    _test_roundtrip_object(user, session)


@given(obj=builtin_objects())
@settings(suppress_health_check=[HealthCheck.function_scoped_fixture])
def test_roundtrip_builtin_object(obj: BuiltinObject, session: Session, space: Space):
    _test_roundtrip_object(obj, session)
