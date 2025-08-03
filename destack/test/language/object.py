from itertools import chain

from destack import Folder, Session, Space
from destack.registry import NODE_CLASS_BY_TYPE, STRUCT_CLASS_BY_TYPE


def test_object_slots(session: Session, space: Space):
    for object_cls in chain(STRUCT_CLASS_BY_TYPE.values(), NODE_CLASS_BY_TYPE.values()):
        assert object_cls.__slots__ and "__dict__" not in object_cls.__slots__


def test_repr_query(session: Session, space: Space):
    query = Folder.search(sort=[Folder.property("created_at").asc()], limit=25)
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
