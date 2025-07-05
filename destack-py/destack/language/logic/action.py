from typing import TYPE_CHECKING, Union

from destack.language.core import (
    IsScriptable,
    NodeType,
    builtin_node,
    builtin_property_parent,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false

from .method import Method


@builtin_node(NodeType.ACTION)
class Action(Method):
    """
    An implementation of a unit of work, usually expressed with Code or some tool.
    May defer to a builtin or some other service in a separate system.
    """

    parent: Union["IsScriptable", None] = builtin_property_parent()

    pass
