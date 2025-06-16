from typing import TYPE_CHECKING, Union

from destack.language.core import (
    Entity,
    Global,
    IndexIn,
    IsDeletable,
    IsOwnable,
    IsSubject,
    LikeFollow,
    Node,
    NodeType,
    Spatial,
    builtin_node,
    property_,
    property_parent_,
)
from destack.proto import FollowData

if TYPE_CHECKING:
    from destack.language import IsFollowable

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(
    NodeType.FOLLOW,
    index=(IndexIn(columns=("parent_id", "owned_by_id"), is_unique=True),),
)
class Follow(
    Global,
    Spatial,
    Entity,
    LikeFollow,
    IsDeletable,
    IsOwnable,
    Node[FollowData],
):
    """A Follow is a relationship between a Subject and a Followred Node."""

    parent: Union["IsFollowable", None] = property_parent_(node_is_customizable=True)
    owned_by: "IsSubject" = property_(25)
