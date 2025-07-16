from typing import TYPE_CHECKING, Union

from ..builtin import (
    IsRunnable,
    IsScriptable,
    NodeType,
    StructType,
    builtin_node,
    builtin_property_parent,
    builtin_struct,
)
from .method import Method, MethodDefinition

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_struct(StructType.ACTION_DEFINITION, frozen=True)
class ActionDefinition(MethodDefinition):
    """Definition of a builtin Action."""

    pass


@builtin_node(NodeType.ACTION)
class Action(IsRunnable, Method):
    """
    An implementation of a unit of work, usually expressed with Code or some tool.
    May defer to a builtin or some other service in a separate system.
    """

    parent: Union["IsScriptable", None] = builtin_property_parent()
