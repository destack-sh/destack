from typing import TYPE_CHECKING, Union

from ..builtin import (
    Entity,
    Enum,
    EnumType,
    IsExtensible,
    IsTaggable,
    NodeType,
    StructType,
    builtin_enum,
    builtin_node,
    builtin_property,
    builtin_property_parent,
    builtin_struct,
)
from .definition import BuiltinDefinition

if TYPE_CHECKING:
    from destack.language import PropertyDefinition

# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@builtin_enum(EnumType.CONSTRAINT_TYPE)
class ConstraintType(Enum):
    """Type of a Constraint."""

    UNIQUE = 1
    # CHECK, ...


@builtin_node(NodeType.CONSTRAINT)
class Constraint(IsTaggable, Entity):
    """Constraint of an Entity."""

    parent: Union["IsExtensible", None] = builtin_property_parent()
    type: ConstraintType = builtin_property(100, is_repr=True)
    properties: list["PropertyDefinition"] = builtin_property(105)


@builtin_struct(StructType.CONSTRAINT_DEFINITION, frozen=True)
class ConstraintDefinition(BuiltinDefinition):
    """Definition of a builtin Constraint."""

    type: "ConstraintType" = builtin_property(100, is_repr=True)
    properties: list["PropertyDefinition"] = builtin_property(105)


@builtin_enum(EnumType.INDEX_TYPE)
class IndexType(Enum):
    """Type of an Index."""

    BTREE = 1
    # HASH, ...


@builtin_struct(StructType.INDEX_DEFINITION, frozen=True)
class IndexDefinition(BuiltinDefinition):
    """Definition of a builtin Index."""

    type: "IndexType" = builtin_property(100, is_repr=True)
    properties: list["PropertyDefinition"] = builtin_property(105)


@builtin_node(NodeType.INDEX)
class Index(IsTaggable, Entity):
    """Index of an Entity."""

    parent: Union["IsExtensible", None] = builtin_property_parent()
    type: IndexType = builtin_property(100, is_repr=True)
    properties: list["PropertyDefinition"] = builtin_property(105)
