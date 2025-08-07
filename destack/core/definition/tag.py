from typing import final

from ..builtin import (
    ImmutableStruct,
    StructType,
    TagDeclaration,
    UInt8,
    declare_property,
    declare_struct,
)


@declare_struct(
    StructType.TAG_DEFINITION,
    frozen=True,
    is_final=True,
)
@final
class TagDefinition(ImmutableStruct):
    """Definition of a builtin Tag to associate builtin definitions to."""

    id: UInt8 = declare_property(2, is_repr=True)
    name: str = declare_property(101, is_repr=True)
    description: str | None = declare_property(103, is_repr=True)

    @classmethod
    def from_declaration(cls, declaration: TagDeclaration) -> "TagDefinition":
        """Create TagDefinition from a TagDeclaration."""
        return cls(
            # meta
            id=declaration.id,
            name=declaration.name,
            description=declaration.description,
        )
