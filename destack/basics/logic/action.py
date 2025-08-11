from typing import final

from destack.core import ActionType, NodeType, TraitType, declare_entity, declare_property

from .function import Function


@declare_entity(
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

    type: ActionType = declare_property(
        100,
        tag=None,
    )
