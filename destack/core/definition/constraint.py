from typing import TYPE_CHECKING, Self, final

from ..builtin import (
    Object,
    StructType,
    UInt8,
    declare_property,
    declare_struct,
)
from .definition import Definition

if TYPE_CHECKING:
    from destack.core import ConstraintDeclaration, ConstraintType, PropertyReference


type_ = type


@declare_struct(
    StructType.CONSTRAINT_DEFINITION,
    frozen=True,
    is_final=True,
)
@final
class ConstraintDefinition(Definition):
    """Definition of a builtin Constraint."""

    id: UInt8 = declare_property(2, is_repr=True)
    type: "ConstraintType" = declare_property(100, is_repr=True)

    # content
    properties: list["PropertyReference"] = declare_property(120)

    @classmethod
    def from_declaration(
        cls, object_cls: type_["Object"], declaration: "ConstraintDeclaration"
    ) -> "Self":
        return cls(
            # meta
            id=declaration.id,
            type=declaration.type,
            name=declaration.name or "Constraint",
            # content
            properties=[object_cls.property(p).to_ref() for p in declaration.properties],
        )
