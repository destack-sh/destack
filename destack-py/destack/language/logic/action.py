from typing import TYPE_CHECKING, Union

from destack.language.core import (
    IsRunnable,
    IsScriptable,
    NodeType,
    builtin_node,
    builtin_property_parent,
)

from .method import Method

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.ACTION)
class Action(IsRunnable, Method):
    """
    An implementation of a unit of work, usually expressed with Code or some tool.
    May defer to a builtin or some other service in a separate system.
    """

    parent: Union["IsScriptable", None] = builtin_property_parent()
