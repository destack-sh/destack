from typing import TYPE_CHECKING

from destack.core import Entity, IndexType, NodeType, builtin_entity, builtin_property

if TYPE_CHECKING:
    from destack import PropertyReference

# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@builtin_entity(NodeType.INDEX)
class Index(Entity):
    """Index of an Entity for faster querying."""

    type: IndexType = builtin_property(100, is_repr=True)
    properties: list["PropertyReference"] = builtin_property(105)
