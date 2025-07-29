from typing import TYPE_CHECKING, Optional

from ..builtin import (
    Entity,
    MethodCardinality,
    MethodType,
    NodeType,
    PlatformType,
    RuntimeLanguage,
    StructFrozen,
    StructType,
    UInt16,
    builtin_node,
    builtin_property,
    builtin_struct,
)

if TYPE_CHECKING:
    from destack.language import PropertyReference, Text

# pyright: reportIncompatibleVariableOverride=false


@builtin_struct(StructType.METHOD_DEFINITION, frozen=True)
class MethodDefinition(StructFrozen):
    """Definition of a builtin Method."""

    id: UInt16 = builtin_property(2, is_repr=True)
    type: MethodType = builtin_property(100)
    name: str = builtin_property(101)
    description: str | None = builtin_property(103, is_repr=True)

    # content
    cardinality: MethodCardinality = builtin_property(120, default=MethodCardinality.UNARY)
    input_properties: list["PropertyReference"] = builtin_property(121)
    output_properties: list["PropertyReference"] | None = builtin_property(122)
    output_property: Optional["PropertyReference"] | None = builtin_property(123)
    # runtimes/languages/...?

    platforms: list[PlatformType] | None = builtin_property(
        130,
        description="The platforms this Method is available on (all if empty).",
    )
    languages: list[RuntimeLanguage] | None = builtin_property(
        131,
        description="The languages this Method is available in (all if empty).",
    )


@builtin_node(NodeType.METHOD)
class Method(Entity):
    """
    A Method is a small piece of logic.
    """

    type: MethodType = builtin_property(100)
    text: Optional["Text"] = builtin_property(104)
    cardinality: MethodCardinality = builtin_property(110, default=MethodCardinality.UNARY)

    platforms: list[PlatformType] | None = builtin_property(
        130,
        description="The platforms this Method is available on (all if empty).",
    )
    languages: list[RuntimeLanguage] | None = builtin_property(
        131,
        description="The languages this Method is available in (all if empty).",
    )
