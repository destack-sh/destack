from typing import TYPE_CHECKING, Self, Union

from ..builtin.common import NodeType, StructType
from ..builtin.entity import Entity
from ..builtin.meta import IndexDeclaration, IndexType
from ..builtin.node import builtin_node
from ..builtin.object import BuiltinObject
from ..builtin.property import builtin_property, builtin_property_parent
from ..builtin.struct import builtin_struct
from ..builtin.trait import IsExtensible
from .definition import BuiltinDefinition

if TYPE_CHECKING:
    from destack.language import PropertyReference

# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


type_ = type


@builtin_struct(StructType.INDEX_DEFINITION, frozen=True)
class IndexDefinition(BuiltinDefinition):
    """Definition of a builtin Index."""

    type: "IndexType" = builtin_property(100, is_repr=True)
    properties: list["PropertyReference"] = builtin_property(105)
    cover: list["PropertyReference"] = builtin_property(106)

    @classmethod
    def from_declaration(
        cls, object_cls: type_["BuiltinObject"], declaration: "IndexDeclaration"
    ) -> "Self":
        return cls(
            id=declaration.id,
            type=declaration.type,
            name=declaration.name or "Index",
            properties=[object_cls.property(p).to_ref() for p in declaration.properties],
            cover=[object_cls.property(p).to_ref() for p in declaration.cover],
        )


@builtin_node(NodeType.INDEX)
class Index(Entity):
    """Index of an Entity for faster querying."""

    parent: Union["IsExtensible", None] = builtin_property_parent()
    type: IndexType = builtin_property(100, is_repr=True)
    properties: list["PropertyReference"] = builtin_property(105)
