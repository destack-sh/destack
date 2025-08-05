from typing import TYPE_CHECKING, Self, final

from ..builtin import (
    StructType,
    UInt8,
    declare_property,
    declare_struct,
)
from .definition import Definition

if TYPE_CHECKING:
    from destack import IndexDeclaration, IndexType, PropertyReference


type_ = type


@declare_struct(
    StructType.INDEX_DEFINITION,
    frozen=True,
    is_final=True,
)
@final
class IndexDefinition(Definition):
    """Definition of a builtin Index."""

    # meta
    id: UInt8 = declare_property(2, is_repr=True)
    type: "IndexType" = declare_property(100, is_repr=True)

    # content
    properties: list["PropertyReference"] = declare_property(120)
    cover: list["PropertyReference"] = declare_property(121)

    @classmethod
    def from_declaration(cls, declaration: "IndexDeclaration") -> "Self":
        return cls(
            # meta
            id=declaration.id,
            type=declaration.type,
            name=declaration.name or "Index",
        )
