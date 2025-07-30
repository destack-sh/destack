from typing import final

from ..builtin import ActionType, NodeType, TraitType, builtin_entity, builtin_property
from .function import Function

# pyright: reportIncompatibleVariableOverride=false


@builtin_entity(
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

    type: ActionType = builtin_property(100)
