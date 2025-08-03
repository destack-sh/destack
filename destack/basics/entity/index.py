from typing import TYPE_CHECKING

from destack.core import Entity, IndexType, NodeType, declare_entity, declare_property

if TYPE_CHECKING:
    from destack import PropertyReference

# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@declare_entity(NodeType.INDEX)
class Index(Entity):
    """Index of an Entity for faster querying."""

    type: IndexType = declare_property(100, is_repr=True)
    properties: list["PropertyReference"] = declare_property(105)
