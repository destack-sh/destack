from itertools import chain

from destack import Folder
from destack.registry import HANDLE_CLASS_BY_TYPE, NODE_CLASS_BY_TYPE, STRUCT_CLASS_BY_TYPE


def test_object_slots():
    for object_cls in chain(
        STRUCT_CLASS_BY_TYPE.values(),
        NODE_CLASS_BY_TYPE.values(),
        HANDLE_CLASS_BY_TYPE.values(),
    ):
        assert "__dict__" not in object_cls.__slots__


def test_resolve_property():
    assert Folder.property("deleted_at").name == "deleted_at"
    assert Folder.property("deletedAt").name == "deleted_at"
    assert Folder.property("DeletedAt").name == "deleted_at"
    assert Folder.property("parent_ref").name == "parent"
    assert Folder.property("parentRef").name == "parent"
    assert Folder.property("parent").name == "parent"
    assert Folder.property("Parent").name == "parent"
    assert Folder.property("ParentRef").name == "parent"
