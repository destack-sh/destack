from typing import TYPE_CHECKING, Union

from destack.language.core import (
    Enum,
    EnumType,
    HasIcon,
    HasName,
    HasSlug,
    Instance,
    IsDeletable,
    IsJoinable,
    Node,
    NodeType,
    Spatial,
    builtin_enum,
    builtin_node,
    property_,
    property_parent_,
)
from destack.pb2 import PermissionData

if TYPE_CHECKING:
    from destack.language import Folder

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.PERMISSION_TYPE)
class PermissionType(Enum):
    """A Type of Permission."""

    GENERAL = 1


@builtin_node(NodeType.PERMISSION)
class Permission(
    Spatial,
    Instance,
    HasName,
    HasSlug,
    HasIcon,
    IsDeletable,
    Node[PermissionData],
):
    """A Permission for something."""

    parent: Union["IsJoinable", "Folder", None] = property_parent_(node_is_customizable=False)
    type: PermissionType = property_(30)
