from typing import TYPE_CHECKING, Union

from destack.language.core import (
    Entity,
    IndexIn,
    IsDeletable,
    IsGlobal,
    IsOwned,
    IsSpatial,
    NodeType,
    builtin_node,
    builtin_property_parent,
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
    IsOwned,
    Entity,
):
    """A Follow is a relationship between a Subject and an IsFollowable Node."""

    parent: Union["IsFollowable", None] = builtin_property_parent()
