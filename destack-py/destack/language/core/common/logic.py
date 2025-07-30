from typing import TYPE_CHECKING, Optional, final

from ..builtin import (
    ActionType,
    Entity,
    MethodType,
    NodeType,
    PlatformType,
    RuntimeLanguage,
    TraitType,
    builtin_node,
    builtin_property,
)

if TYPE_CHECKING:
    from destack.language import Text

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.FUNCTION, is_abstract=True)
class Function(Entity):
    # meta
    type: MethodType = builtin_property(100)
    text: Optional["Text"] = builtin_property(104)

    platforms: list[PlatformType] | None = builtin_property(
        130,
        description="The platforms this Function is available on (all if empty).",
    )
    languages: list[RuntimeLanguage] | None = builtin_property(
        131,
        description="The languages this Function is available in (all if empty).",
    )


@builtin_node(NodeType.METHOD, is_final=True)
@final
class Method(Function):
    """
    A Method is a small runtime-specific piece of logic.
    """


@builtin_node(
    NodeType.ACTION,
    traits=(TraitType.RUNNABLE,),
    is_final=True,
)
@final
class Action(Function):
    """
    An implementation of a unit of work implemented for some runtimes.
    Actions are stateful and can be called and managed across runtimes.
    """

    cardinality: ActionType = builtin_property(120)
