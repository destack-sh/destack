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
    from destack.language import IsStarable

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(
    NodeType.STAR,
    index=(IndexIn(columns=("parent_id", "owned_by_id"), is_unique=True),),
)
class Star(
    IsGlobal,
    IsSpatial,
    IsDeletable,
    IsOwnable,
    Entity,
):
    """A Star is a relationship between a Subject and a Starred Node."""

    parent: Union["IsStarable", None] = property_parent_(node_is_customizable=True)
    owned_by: "IsSubject" = property_(25)
