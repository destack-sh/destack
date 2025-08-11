from typing import TYPE_CHECKING, final

from ..builtin import (
    EnumDeclaration,
    EnumType,
    StructType,
    UInt8,
    UInt32,
    declare_property,
    declare_struct,
)
from .definition import Definition

if TYPE_CHECKING:
    from .option import OptionDefinition


type_ = type


@declare_struct(
    StructType.ENUM_DEFINITION,
    is_final=True,
)
@final
class EnumDefinition(Definition):
    """Definition of a builtin Enum."""

    id: UInt32 = declare_property(2, is_repr=True, tag=None)
    type: EnumType = declare_property(100, is_repr=True, tag=None)
    taggings: list[UInt8] = declare_property(109, tag=None)
    is_flag: bool = declare_property(110, tag=None)

    # content
    options: list["OptionDefinition"] = declare_property(120, tag=None)

    @classmethod
    def from_declaration(cls, declaration: EnumDeclaration) -> "EnumDefinition":
        """Create EnumDefinition from an Enum class."""
        from .option import OptionDefinition

        return cls(
            # meta
            id=declaration.id,
            type=declaration.type,
            name=declaration.name,
            description=declaration.description,
            is_flag=declaration.is_flag,
            # content
            options=[
                OptionDefinition.from_declaration(declaration.type, option)
                for option in declaration.options
            ],
        )
