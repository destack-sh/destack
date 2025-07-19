from typing import TYPE_CHECKING, Optional, Union

from ..builtin import (
    Entity,
    Enum,
    EnumType,
    IsCustomizable,
    IsScriptable,
    IsSourceable,
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
    from destack.language import Icon, PropertyDefinition, Text

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.METHOD_CARDINALITY)
class MethodCardinality(Enum):
    UNARY = 1, "Unary", "Single in, single out"
    # UNARY_STREAM = 2, "Unary Stream", "Single in, stream out"

    @property
    def is_boundary(self) -> bool:
        return self < 40


@builtin_struct(StructType.METHOD_DEFINITION, frozen=True)
class MethodDefinition(BuiltinDefinition):
    """Definition of a builtin Method."""

    properties: list["PropertyDefinition"] = builtin_property(104)


@builtin_node(NodeType.METHOD)
class Method(
    IsSourceable,
    IsCustomizable,
    Entity,
):
    """
    A Method is a small piece of logic.
    """

    parent: Union["IsScriptable", None] = builtin_property_parent()

    icon: "Icon | None" = builtin_property(102)
    text: Optional["Text"] = builtin_property(104)

    cardinality: MethodCardinality = builtin_property(110, default=MethodCardinality.UNARY)
