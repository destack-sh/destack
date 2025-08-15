from typing import TYPE_CHECKING, Self, final

from destack.core import (
    Definition,
    Entity,
    IndexType,
    NodeType,
    StructType,
    UInt8,
    declare_entity,
    declare_property,
    declare_struct,
)

if TYPE_CHECKING:
    from destack import IndexDeclaration, IndexType, PropertyReference


type_ = type


# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@declare_struct(
    StructType.INDEX_DEFINITION,
    is_final=True,
)
@final
class IndexDefinition(Definition):
    """Definition of a builtin Index."""

    # meta
    id: UInt8 = declare_property(2, is_repr=True, tag=None)
    type: "IndexType" = declare_property(100, is_repr=True, tag=None)

    # content
    properties: list["PropertyReference"] = declare_property(120, tag=None)
    cover: list["PropertyReference"] = declare_property(121, tag=None)

    @classmethod
    def from_declaration(cls, declaration: "IndexDeclaration") -> "Self":
        return cls(
            # meta
            id=declaration.id,
            type=declaration.type,
            name=declaration.name or "Index",
        )


@declare_entity(NodeType.INDEX)
class Index(Entity):
    """Index of an Entity for faster querying."""

    type: IndexType = declare_property(
        100,
        is_repr=True,
        tag=None,
    )
    properties: list["PropertyReference"] = declare_property(
        105,
        tag=None,
    )
