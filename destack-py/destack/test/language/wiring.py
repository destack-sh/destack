import json

from hypothesis import HealthCheck, given, settings

from destack.grpc import AnyObjectProto
from destack.language import (
    BuiltinObject,
    EventCursor,
    Folder,
    Join,
    JoinType,
    NodeReference,
    NodeType,
    Query,
    Session,
    Space,
    User,
    UserStatus,
)
from destack.test.strategies import builtin_objects
from destack.utils.uuid import uuid4


def test_roundtrip_node_reference(session: Session, space: Space):
    """Pack and unpack a NodeReference."""
    node_ref = NodeReference(
        type=NodeType.FOLDER, id=uuid4(), space_id=uuid4(), definition_id=uuid4()
    )

    # proto
    node_ref_data = node_ref.to_proto()
    assert node_ref.to_proto() is node_ref_data  # cached (frozen Struct)
    node_ref_data_bytes = node_ref_data.SerializeToString()
    unpacked_node_ref_data = type(node_ref_data)()
    unpacked_node_ref_data.ParseFromString(node_ref_data_bytes)
    unpacked_node_ref = NodeReference.from_proto(unpacked_node_ref_data)
    assert unpacked_node_ref.equals(node_ref), f"{unpacked_node_ref!r} != {node_ref!r}"
    assert unpacked_node_ref.to_proto() is unpacked_node_ref_data  # cached (frozen Struct)
    assert unpacked_node_ref.hash() == node_ref.hash(), (
        f"{unpacked_node_ref.hash()} != {node_ref.hash()}"
    )

    # cson
    node_ref_cson = node_ref.to_cson()
    node_ref_cson_str = json.dumps(node_ref_cson, indent=2)
    unpacked_node_ref_cson = json.loads(node_ref_cson_str)
    unpacked_node_ref = NodeReference.from_cson(unpacked_node_ref_cson)
    assert unpacked_node_ref.equals(node_ref), f"{unpacked_node_ref!r} != {node_ref!r}"
    assert unpacked_node_ref.to_cson() is unpacked_node_ref_cson  # cached (frozen Struct)
    assert unpacked_node_ref.hash() == node_ref.hash(), (
        f"{unpacked_node_ref.hash()} != {node_ref.hash()}"
    )


def test_roundtrip_query(session: Session, space: Space):
    """Pack and unpack a Query."""
    query = Folder.search(
        sort=[Folder.property("created_at").asc()],
        limit=25,
        Cursor=EventCursor.get(
            join=Join.of(JoinType.LEFT, on=EventCursor.property("created_epoch").eq(5)),
        ),
    )

    # proto
    query_data = query.to_proto()
    query_data_bytes = query_data.SerializeToString()
    unpacked_query_data = type(query_data)()
    unpacked_query_data.ParseFromString(query_data_bytes)
    unpacked_query = Query.from_proto(unpacked_query_data)
    assert unpacked_query.equals(query), f"{unpacked_query!r} != {query!r}"
    assert unpacked_query.to_proto() is unpacked_query_data  # cached (frozen Struct)
    assert unpacked_query.hash() == query.hash(), f"{unpacked_query.hash()} != {query.hash()}"

    # cson
    query_cson = query.to_cson()
    query_cson_str = json.dumps(query_cson, indent=2)
    unpacked_query_cson = json.loads(query_cson_str)
    unpacked_query = Query.from_cson(unpacked_query_cson)
    assert unpacked_query.equals(query), f"{unpacked_query!r} != {query!r}"
    assert unpacked_query.to_cson() is unpacked_query_cson  # cached (frozen Struct)
    assert unpacked_query.hash() == query.hash(), f"{unpacked_query.hash()} != {query.hash()}"


def test_roundtrip_user(session: Session, space: Space):
    """Pack and unpack a User."""
    user = User(
        status=UserStatus.ACTIVE,
        name="Florian",
        slug="florian",
        space_ptr=NodeReference(id=uuid4(), type=NodeType.SPACE),
    )

    # proto
    user_data = user.to_proto()
    user_data_bytes = user_data.SerializeToString()
    unpacked_user_data = type(user_data)()
    unpacked_user_data.ParseFromString(user_data_bytes)
    unpacked_user = User.from_proto(unpacked_user_data)
    assert unpacked_user.equals(user), f"{unpacked_user!r} != {user!r}"
    assert unpacked_user.hash() == user.hash(), f"{unpacked_user.hash()} != {user.hash()}"

    # cson
    user_cson = user.to_cson()
    user_cson_str = json.dumps(user_cson, indent=2)
    unpacked_user_cson = json.loads(user_cson_str)
    unpacked_user = User.from_cson(unpacked_user_cson)
    assert unpacked_user.equals(user), f"{unpacked_user!r} != {user!r}"
    assert unpacked_user.hash() == user.hash(), f"{unpacked_user.hash()} != {user.hash()}"


@given(obj=builtin_objects())
@settings(suppress_health_check=[HealthCheck.function_scoped_fixture])
def test_roundtrip_builtin_object(
    obj: BuiltinObject[AnyObjectProto], session: Session, space: Space
):
    # proto
    packed_obj_data: AnyObjectProto = obj.to_proto()
    packed_bytes = packed_obj_data.SerializeToString()
    unpacked_obj_data = type(packed_obj_data)()
    unpacked_obj_data.ParseFromString(packed_bytes)
    unpacked_obj = obj.__unpack_proto__(unpacked_obj_data, _graph=session.graph)
    assert unpacked_obj.equals(obj), f"{unpacked_obj!r} != {obj!r}"
    assert unpacked_obj.hash() == obj.hash(), f"{unpacked_obj.hash()} != {obj.hash()}"

    # cson
    packed_obj_cson = obj.to_cson()
    packed_obj_cson_str = json.dumps(packed_obj_cson, indent=2)
    unpacked_obj_cson = json.loads(packed_obj_cson_str)
    unpacked_obj = obj.from_cson(unpacked_obj_cson, _graph=session.graph)
    assert unpacked_obj.equals(obj), f"{unpacked_obj!r} != {obj!r}"
    assert unpacked_obj.hash() == obj.hash(), f"{unpacked_obj.hash()} != {obj.hash()}"
