from typing import TYPE_CHECKING, Union

from destack.language.core import (
    Entity,
    Global,
    IndexIn,
    IsDeletable,
    IsOwnable,
    IsTemplatable,
    Node,
    NodeType,
    Spatial,
    node_,
    property_parent_,
)
from destack.pb2 import StarData

if TYPE_CHECKING:
    from destack.language import IsStarable

# pyright: reportIncompatibleVariableOverride=false


@node_(
    NodeType.STAR,
    index=(IndexIn(columns=("parent_id", "owned_by_id"), is_unique=True),),
)
class Star(
    Global,
    Spatial,
    IsDeletable,
    IsTemplatable,
    IsOwnable,
    Entity,
    Node[StarData],
):
    """A Star is a relationship between a Subject and a Starred Node."""

    parent: Union["IsStarable", None] = property_parent_(node_is_customizable=True)
