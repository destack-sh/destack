from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    Global,
    HasIcon,
    HasName,
    HasSlug,
    IsJoinable,
    IsOwner,
    Node,
    NodeType,
    builtin_node,
    property_parent_,
)

if TYPE_CHECKING:
    from destack.language import Organization

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.TEAM, root_type=None)
class Team(
    Global,
    Entity,
    HasSlug,
    HasIcon,
    HasName,
    IsOwner,
    IsJoinable,
    Node,
):
    """
    An Team with Users and Teams.
    """

    parent: Optional["Organization"] = property_parent_(node_is_customizable=False)
