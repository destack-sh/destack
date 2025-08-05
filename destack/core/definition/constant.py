from typing import TYPE_CHECKING, final

from ..builtin import (
    StructType,
    UInt8,
    declare_property,
    declare_property_runtime,
    declare_struct,
)
from ..common import Value
from .definition import Definition

if TYPE_CHECKING:
    from destack import ConstantDeclaration


@declare_struct(
    StructType.CONSTANT_DEFINITION,
    frozen=True,
    is_final=True,
)
@final
class ConstantDefinition(Definition):
    """Definition of a builtin Constant."""

    id: UInt8 = declare_property(2, is_repr=True)
    taggings: list[UInt8] = declare_property(109)

    # content
    value: "Value" = declare_property(120)

    _is_deferred: bool = declare_property_runtime(401)

    @classmethod
    def from_declaration(cls, declaration: "ConstantDeclaration") -> "ConstantDefinition":
        """Create ConstantDefinition from a ConstantDeclaration."""
        assert declaration.name is not None, f"{declaration!r} has no name"
        return cls(
            # meta
            id=declaration.id,
            name=declaration.name,
            description=declaration.description,
            # content
            value=Value.wrap(declaration.value),
            _is_deferred=declaration.is_deferred,
        )
