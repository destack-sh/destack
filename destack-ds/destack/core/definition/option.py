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
    is_final=True,
)
@final
class OptionDefinition(Definition):
    """Definition of a builtin Enum Option."""

    id: UInt8 = declare_property(2, is_repr=True, tag=None)
    type: EnumType = declare_property(100, is_repr=True, tag=None)
    taggings: list[UInt8] = declare_property(109, tag=None)

    is_internal: bool = declare_property(110, default=False, tag=None)

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
            description=declaration.description or "",
            is_internal=declaration.is_internal,
        )
