from typing import TYPE_CHECKING, Union

from destack.language.core import (
    Global,
    IndexIn,
    Instance,
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
from destack.pb2 import FollowData

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
    Instance,
    LikeFollow,
    IsDeletable,
    IsOwnable,
    Node[FollowData],
):
    """A Follow is a relationship between a Subject and a Followred Node."""

    parent: Union["IsFollowable", None] = property_parent_(node_is_customizable=True)
    owned_by: "IsSubject" = property_(22)
