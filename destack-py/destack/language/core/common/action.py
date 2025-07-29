from typing import TYPE_CHECKING

from ..builtin import NodeType, StructType, TraitType, builtin_node, builtin_struct
from .method import Method, MethodDefinition

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_struct(StructType.ACTION_DEFINITION, frozen=True)
class ActionDefinition(MethodDefinition):
    """Definition of a builtin Action."""

    pass


@builtin_node(
    NodeType.ACTION,
    traits=(TraitType.RUNNABLE,),
)
class Action(Method):
    """
    An implementation of a unit of work, usually expressed with Code or some tool.
    May defer to a builtin or some other service in a separate system.
    """
