from typing import final

from ..builtin import (
    EnumType,
    OptionDeclaration,
    StructType,
    UInt8,
    declare_property,
    declare_struct,
)
from .definition import Definition


@declare_struct(
    StructType.OPTION_DEFINITION,
    frozen=True,
    is_final=True,
)
@final
class OptionDefinition(Definition):
    """Definition of a builtin Enum Option."""

    id: UInt8 = declare_property(2, is_repr=True)
    type: EnumType = declare_property(100, is_repr=True)
    taggings: list[UInt8] = declare_property(109)

    @classmethod
    def from_declaration(
        cls, enum_type: EnumType, declaration: OptionDeclaration
    ) -> "OptionDefinition":
        """Create OptionDefinition from an Enum option."""
        return cls(
            # meta
            id=declaration.value,
            type=enum_type,
            name=declaration.name,
            description=declaration.description,
        )
