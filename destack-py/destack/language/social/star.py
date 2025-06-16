from typing import TYPE_CHECKING, Union

from destack.language.core import (
    Entity,
    Global,
    IndexIn,
    IsDeletable,
    IsOwnable,
    IsSubject,
    Node,
    NodeType,
    Spatial,
    builtin_node,
    property_,
    property_parent_,
)
from destack.proto import StarData

if TYPE_CHECKING:
    from destack.language import IsStarable

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(
    NodeType.STAR,
    index=(IndexIn(columns=("parent_id", "owned_by_id"), is_unique=True),),
)
class Star(
    Global,
    Spatial,
    Entity,
    IsDeletable,
    IsOwnable,
    Node[StarData],
):
    """A Star is a relationship between a Subject and a Starred Node."""

    parent: Union["IsStarable", None] = property_parent_(node_is_customizable=True)
    owned_by: "IsSubject" = property_(25)
