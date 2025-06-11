from typing import TYPE_CHECKING, Union

from destack.language.core import (
    IndexIn,
    IsEntity,
    IsOwnable,
    Node,
    NodeType,
    node_,
    property_,
    property_parent_,
)
from destack.pb2 import ReactionData

if TYPE_CHECKING:
    from destack.language import IsReactable

# pyright: reportIncompatibleVariableOverride=false


@node_(
    NodeType.REACTION,
    index=(IndexIn(columns=("parent_id", "owned_by_id", "content"), is_unique=True),),
)
class Reaction(
    IsOwnable,
    IsEntity,
    Node[ReactionData],
):
    """A Reaction is a relationship between a Subject and a Reaction Node."""

    parent: Union["IsReactable", None] = property_parent_(node_is_customizable=True)

    content: str = property_(40)
