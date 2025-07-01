from typing import TYPE_CHECKING, Union

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    HasIcon,
    HasName,
    HasSlug,
    IsDeletable,
    IsJoinable,
    IsSpatial,
    NodeType,
    builtin_enum,
    builtin_node,
    property_,
    property_parent_,
)

if TYPE_CHECKING:
    from destack.language import Folder

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.PERMISSION_TYPE)
class PermissionType(Enum):
    """A Type of Permission."""

    GENERAL = 1


@builtin_node(NodeType.PERMISSION)
class Permission(
    IsSpatial,
    HasName,
    HasSlug,
    HasIcon,
    IsDeletable,
    Entity,
):
    """A Permission for something."""

    parent: Union["IsJoinable", "Folder", None] = property_parent_(node_is_extensible=False)
    type: PermissionType = property_(30)
