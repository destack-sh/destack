from typing import TYPE_CHECKING, final

from ..builtin import NodeType, StructType, TraitType, builtin_node, builtin_struct
from .method import Function, FunctionDefinition

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_struct(
    StructType.ACTION_DEFINITION,
    frozen=True,
    is_final=True,
)
@final
class ActionDefinition(FunctionDefinition):
    """Definition of a builtin Action."""

    pass


@builtin_node(
    NodeType.ACTION,
    traits=(TraitType.RUNNABLE,),
    is_final=True,
)
@final
class Action(Function):
    """
    An implementation of a unit of work, usually expressed with Code or some tool.
    May defer to a builtin or some other service in a separate system.
    """
