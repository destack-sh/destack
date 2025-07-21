from typing import TYPE_CHECKING, Optional

from ..builtin import (
    Entity,
    MethodCardinality,
    MethodType,
    NodeType,
    PlatformType,
    RuntimeLanguage,
    StructType,
    builtin_node,
    builtin_property,
    builtin_struct,
)
from .definition import BuiltinDefinition

if TYPE_CHECKING:
    from destack.language import PropertyDefinition, Text

# pyright: reportIncompatibleVariableOverride=false


@builtin_struct(StructType.METHOD_DEFINITION, frozen=True)
class MethodDefinition(BuiltinDefinition):
    """Definition of a builtin Method."""

    type: MethodType = builtin_property(100)
    properties: list["PropertyDefinition"] = builtin_property(104)
    cardinality: MethodCardinality = builtin_property(110, default=MethodCardinality.UNARY)
    # runtimes/languages/...?
    platforms: list[PlatformType] = builtin_property(
        120,
        description="The platforms this Method is available on (all if empty).",
    )
    languages: list[RuntimeLanguage] = builtin_property(
        121,
        description="The languages this Method is available in (all if empty).",
    )


@builtin_node(NodeType.METHOD)
class Method(
    Entity,
):
    """
    A Method is a small piece of logic.
    """

    type: MethodType = builtin_property(100)
    text: Optional["Text"] = builtin_property(104)
    cardinality: MethodCardinality = builtin_property(110, default=MethodCardinality.UNARY)

    platforms: list[PlatformType] = builtin_property(
        120,
        description="The platforms this Method is available on (all if empty).",
    )
    languages: list[RuntimeLanguage] = builtin_property(
        121,
        description="The languages this Method is available in (all if empty).",
    )
