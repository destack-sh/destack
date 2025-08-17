from typing import TYPE_CHECKING, Self, final

from destack.core import (
    ConstraintType,
    Definition,
    Entity,
    NodeType,
    StructType,
    UInt8,
    declare_entity,
    declare_property,
    declare_struct,
)

if TYPE_CHECKING:
    from destack import ConstraintDeclaration, PropertyReference

# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@declare_struct(
    StructType.CONSTRAINT_DEFINITION,
    is_final=True,
)
@final
class ConstraintDefinition(Definition):
    """Definition of a builtin Constraint."""

    id: UInt8 = declare_property(2, is_repr=True, tag=None)
    type: "ConstraintType" = declare_property(100, is_repr=True, tag=None)

    # content
    properties: list["PropertyReference"] = declare_property(120, tag=None)

    @classmethod
    def from_declaration(cls, declaration: "ConstraintDeclaration") -> "Self":
        return cls(
            # meta
            id=declaration.id,
            type=declaration.type,
            name=declaration.name or "Constraint",
            description=declaration.description or "",
        )


@declare_entity(NodeType.CONSTRAINT)
class Constraint(Entity):
    """Constraint of an Entity that must be satisfied."""

    type: ConstraintType = declare_property(
        100,
        is_repr=True,
        tag=None,
    )
    properties: list["PropertyReference"] = declare_property(
        105,
        tag=None,
    )
