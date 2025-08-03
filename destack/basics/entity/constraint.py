from typing import TYPE_CHECKING

from destack.core import ConstraintType, Entity, NodeType, declare_entity, declare_property

if TYPE_CHECKING:
    from destack import PropertyReference

# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@declare_entity(NodeType.CONSTRAINT)
class Constraint(Entity):
    """Constraint of an Entity that must be satisfied."""

    type: ConstraintType = declare_property(100, is_repr=True)
    properties: list["PropertyReference"] = declare_property(105)
