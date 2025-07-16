from typing import TYPE_CHECKING, Union

from ..builtin import (
    Entity,
    Enum,
    EnumType,
    IsJoinable,
    IsSourceable,
    NodeType,
    StructType,
    builtin_enum,
    builtin_node,
    builtin_property,
    builtin_property_parent,
    builtin_struct,
)

if TYPE_CHECKING:
    from destack.language import Folder, Icon

# pyright: reportIncompatibleVariableOverride=false

from .definition import BuiltinDefinition


@builtin_enum(EnumType.PERMISSION_TYPE)
class PermissionType(Enum):
    """A Type of Permission."""

    GENERAL = 1


@builtin_struct(StructType.PERMISSION_DEFINITION, frozen=True)
class PermissionDefinition(BuiltinDefinition):
    """Definition of a builtin Permission for a builtin Node."""

    pass


@builtin_node(NodeType.PERMISSION)
class Permission(
    IsSourceable,
    Entity,
):
    """A Permission for something."""

    parent: Union["IsJoinable", "Folder", None] = builtin_property_parent()
    type: PermissionType = builtin_property(100, is_repr=True)
    icon: "Icon | None" = builtin_property(102)
