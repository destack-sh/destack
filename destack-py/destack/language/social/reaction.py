from typing import TYPE_CHECKING, Union

from destack.language.core import (
    Global,
    IndexIn,
    Instance,
    IsDeletable,
    IsOwnable,
    IsReactable,
    IsSubject,
    Node,
    NodeType,
    Spatial,
    builtin_node,
    property_,
    property_parent_,
)
from destack.pb2 import ReactionData

if TYPE_CHECKING:
    from destack.language import IsReactable

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(
    NodeType.REACTION,
    index=(IndexIn(columns=("parent_id", "owned_by_id", "content"), is_unique=True),),
)
class Reaction(
    Global,
    Spatial,
    Instance,
    IsReactable,
    IsDeletable,
    IsOwnable,
    Node[ReactionData],
):
    """A Reaction is a relationship between a Subject and a Reaction Node."""

    parent: Union["IsReactable", None] = property_parent_(node_is_customizable=True)
    owned_by: "IsSubject" = property_(22)

    content: str = property_(40, is_repr=True)
