from typing import TYPE_CHECKING

from ..builtin.builtin import NodeType
from ..builtin.entity import Entity
from ..builtin.meta import ConstraintType, IndexType
from ..builtin.node import builtin_node
from ..builtin.property import builtin_property

if TYPE_CHECKING:
    from destack.language import PropertyReference

# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


type_ = type


@builtin_node(NodeType.INDEX)
class Index(Entity):
    """Index of an Entity for faster querying."""

    type: IndexType = builtin_property(100, is_repr=True)
    properties: list["PropertyReference"] = builtin_property(105)


@builtin_node(NodeType.CONSTRAINT)
class Constraint(Entity):
    """Constraint of an Entity that must be satisfied."""

    type: ConstraintType = builtin_property(100, is_repr=True)
    properties: list["PropertyReference"] = builtin_property(105)
