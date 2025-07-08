from hypothesis import HealthCheck, given, settings

from destack.language import BuiltinObject, Folder, Session, Thread
from destack.proto import AnyObjectProto
from destack.test.fixtures import BUILTIN_OBJECTS
from destack.test.strategies import builtin_objects, examples


@given(obj=builtin_objects())
@examples([{"obj": obj} for obj in BUILTIN_OBJECTS])
@settings(suppress_health_check=[HealthCheck.function_scoped_fixture])
def test_object_slots(obj: BuiltinObject[AnyObjectProto], session: Session):
    assert not hasattr(obj, "__dict__")
    assert obj.__slots__


def test_repr_query():
    query = Thread.search(sort=[Thread.property("created_at").asc()], limit=25)
    query_repr = repr(query)
    print(query_repr)  # noqa: T201
    assert query_repr is repr(query)  # cached (frozen Struct)


def test_resolve_property():
    assert Folder.property("deleted_at").name == "deleted_at"
    assert Folder.property("deletedAt").name == "deleted_at"
    assert Folder.property("DeletedAt").name == "deleted_at"
    assert Folder.property("parent_ptr").name == "parent"
    assert Folder.property("parentPtr").name == "parent"
    assert Folder.property("parent").name == "parent"
    assert Folder.property("Parent").name == "parent"
    assert Folder.property("ParentPtr").name == "parent"


@given(obj=builtin_objects())
@examples([{"obj": obj} for obj in BUILTIN_OBJECTS])
@settings(suppress_health_check=[HealthCheck.function_scoped_fixture])
def test_repr_builtin_object(obj: BuiltinObject[AnyObjectProto], session: Session):
    print(repr(obj))  # noqa: T201
