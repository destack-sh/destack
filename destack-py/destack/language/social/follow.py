from typing import TYPE_CHECKING, Union

from destack.language.core import (
    Entity,
    IndexIn,
    IsDeletable,
    IsGlobal,
    IsOwnable,
    IsSpatial,
    IsSubject,
    NodeType,
    builtin_node,
    property_,
    property_parent_,
)

if TYPE_CHECKING:
    from destack.language import IsFollowable

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(
    NodeType.FOLLOW,
    index=(IndexIn(columns=("parent_id", "owned_by_id"), is_unique=True),),
)
class Follow(
    IsGlobal,
    IsSpatial,
    IsDeletable,
    IsOwnable,
    Entity,
):
    """A Follow is a relationship between a Subject and an IsFollowable Node."""

    parent: Union["IsFollowable", None] = property_parent_(node_is_extensible=True)
    owned_by: "IsSubject" = property_(25)
