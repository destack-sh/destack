from typing import TYPE_CHECKING, Self, final

from ..builtin import (
    StructType,
    UInt8,
    declare_property,
    declare_struct,
)
from .definition import Definition

if TYPE_CHECKING:
    from destack import PermissionDeclaration


@declare_struct(
    StructType.PERMISSION_DEFINITION,
    is_final=True,
)
@final
class PermissionDefinition(Definition):
    """Definition of a builtin Permission for a builtin Node."""

    id: UInt8 = declare_property(2, is_repr=True, tag=None)

    @classmethod
    def from_declaration(cls, declaration: "PermissionDeclaration") -> "Self":
        return cls(
            # meta
            id=declaration.id,
            name=declaration.name,
            description=declaration.description,
        )
