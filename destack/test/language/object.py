from itertools import chain

from destack import Folder
from destack.registry import NODE_CLASS_BY_TYPE, STRUCT_CLASS_BY_TYPE


def test_object_slots():
    for object_cls in chain(STRUCT_CLASS_BY_TYPE.values(), NODE_CLASS_BY_TYPE.values()):
        assert object_cls.__slots__ and "__dict__" not in object_cls.__slots__


def test_resolve_property():
    assert Folder.property("deleted_at").name == "deleted_at"
    assert Folder.property("deletedAt").name == "deleted_at"
    assert Folder.property("DeletedAt").name == "deleted_at"
    assert Folder.property("parent_ptr").name == "parent"
    assert Folder.property("parentPtr").name == "parent"
    assert Folder.property("parent").name == "parent"
    assert Folder.property("Parent").name == "parent"
    assert Folder.property("ParentPtr").name == "parent"
