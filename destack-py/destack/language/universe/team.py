from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    IsActor,
    IsJoinable,
    IsScriptable,
    NodeType,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import Organization

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.TEAM)
class Team(
    IsActor,
    IsJoinable,
    IsScriptable,
    Entity,
):
    """
    An Team with Users and Teams.
    """

    parent: Optional["Organization"] = builtin_property_parent()
    slug: str = builtin_property(102, is_repr=True)
