from typing import TYPE_CHECKING, Optional, final

from destack.core import (
    Entity,
    NodeType,
    Struct,
    StructType,
    TraitType,
    Type,
    UInt8,
    declare_entity,
    declare_property,
    declare_struct,
)

if TYPE_CHECKING:
    from destack import TagDeclaration


@declare_struct(
    StructType.TAG_DEFINITION,
    is_final=True,
)
@final
class TagDefinition(Struct):
    """Definition of a builtin Tag to associate builtin definitions to."""

    id: UInt8 = declare_property(2, is_repr=True, tag=None)
    name: str = declare_property(101, is_repr=True, tag=None)
    description: str | None = declare_property(103, is_repr=True, tag=None)
    is_internal: bool = declare_property(104, is_repr=True, tag=None)

    @classmethod
    def from_declaration(cls, declaration: "TagDeclaration") -> "TagDefinition":
        """Create TagDefinition from a TagDeclaration."""
        return cls(
            # meta
            id=declaration.id,
            name=declaration.name,
            description=declaration.description,
            is_internal=declaration.is_internal,
        )


@declare_entity(
    NodeType.TAG,
    traits=(TraitType.ORDERED,),
)
class Tag(Entity):
    """A Tag to tag an Entity with."""

    type: Optional["Type"] = declare_property(
        100,
        description="The designated Type for Values associated with this Tag. Any if unset.",
        tag=None,
    )
