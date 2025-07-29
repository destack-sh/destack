from typing import TYPE_CHECKING, Optional, final

from ..builtin import (
    Entity,
    FunctionCardinality,
    FunctionType,
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


@builtin_struct(
    StructType.FUNCTION_DEFINITION,
    frozen=True,
    is_abstract=True,
)
class FunctionDefinition(StructFrozen):
    """Definition of a builtin Function."""

    id: UInt16 = builtin_property(2, is_repr=True)
    type: FunctionType = builtin_property(100)
    name: str = builtin_property(101)
    description: str | None = builtin_property(103, is_repr=True)

    # content
    cardinality: FunctionCardinality = builtin_property(120, default=FunctionCardinality.UNARY)
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


@builtin_struct(
    StructType.METHOD_DEFINITION,
    frozen=True,
    is_final=True,
)
@final
class MethodDefinition(FunctionDefinition):
    """Definition of a builtin Method."""


@builtin_node(NodeType.FUNCTION, is_abstract=True)
class Function(Entity):
    type: FunctionType = builtin_property(100)
    text: Optional["Text"] = builtin_property(104)
    cardinality: FunctionCardinality = builtin_property(110, default=FunctionCardinality.UNARY)

    platforms: list[PlatformType] | None = builtin_property(
        130,
        description="The platforms this Method is available on (all if empty).",
    )
    languages: list[RuntimeLanguage] | None = builtin_property(
        131,
        description="The languages this Method is available in (all if empty).",
    )


@builtin_node(NodeType.METHOD, is_final=True)
@final
class Method(Function):
    """
    A Method is a small piece of logic.
    """
