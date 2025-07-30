from typing import TYPE_CHECKING

from ..builtin.builtin import NodeType
from ..builtin.declaration import ConstraintType
from ..builtin.entity import Entity, builtin_entity
from ..builtin.property import builtin_property

if TYPE_CHECKING:
    from destack.language import PropertyReference

# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@builtin_entity(NodeType.CONSTRAINT)
class Constraint(Entity):
    """Constraint of an Entity that must be satisfied."""

    type: ConstraintType = builtin_property(100, is_repr=True)
    properties: list["PropertyReference"] = builtin_property(105)
