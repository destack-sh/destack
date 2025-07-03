from typing import TYPE_CHECKING, Union

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    IsDeletable,
    IsJoinable,
    IsSpatial,
    NodeType,
    builtin_enum,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import Folder, Icon

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.PERMISSION_TYPE)
class PermissionType(Enum):
    """A Type of Permission."""

    GENERAL = 1


@builtin_node(NodeType.PERMISSION)
class Permission(
    IsSpatial,
    IsDeletable,
    Entity,
):
    """A Permission for something."""

    parent: Union["IsJoinable", "Folder", None] = builtin_property_parent()
    type: PermissionType = builtin_property(100, is_repr=True)
    name: str = builtin_property(101, is_repr=True)
    icon: "Icon | None" = builtin_property(102)
