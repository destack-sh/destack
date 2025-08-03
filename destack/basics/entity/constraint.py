from typing import TYPE_CHECKING

from destack.core import ConstraintType, Entity, NodeType, builtin_entity, builtin_property

if TYPE_CHECKING:
    from destack import PropertyReference

# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@builtin_entity(NodeType.CONSTRAINT)
class Constraint(Entity):
    """Constraint of an Entity that must be satisfied."""

    type: ConstraintType = builtin_property(100, is_repr=True)
    properties: list["PropertyReference"] = builtin_property(105)
