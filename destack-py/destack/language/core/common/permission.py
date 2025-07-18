from typing import TYPE_CHECKING, Self, Union

from ..builtin import (
    Entity,
    IsExtensible,
    IsSourceable,
    NodeType,
    StructType,
    builtin_node,
    builtin_property_parent,
    builtin_struct,
)

if TYPE_CHECKING:
    from destack.language import PermissionDeclaration

# pyright: reportIncompatibleVariableOverride=false

from .definition import BuiltinDefinition


@builtin_struct(StructType.PERMISSION_DEFINITION, frozen=True)
class PermissionDefinition(BuiltinDefinition):
    """Definition of a builtin Permission for a builtin Node."""

    @classmethod
    def from_declaration(cls, declaration: "PermissionDeclaration") -> "Self":
        return cls(
            id=declaration.id,
            name=declaration.name,
        )


@builtin_node(NodeType.PERMISSION)
class Permission(
    IsSourceable,
    Entity,
):
    """A Permission for something."""

    parent: Union["IsExtensible", None] = builtin_property_parent()
