from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    IsJoinable,
    IsSubject,
    NodeType,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import Organization

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.TEAM, root_type=None)
class Team(IsSubject, IsJoinable, Entity):
    """
    An Team with Users and Teams.
    """

    parent: Optional["Organization"] = builtin_property_parent()
    name: str = builtin_property(101, is_repr=True)
    slug: str = builtin_property(102, is_repr=True)
