from typing import TYPE_CHECKING, Union

from destack.language.core import (
    Entity,
    IndexIn,
    IsDeletable,
    IsGlobal,
    IsOwned,
    IsReactable,
    IsSpatial,
    NodeType,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import IsReactable

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(
    NodeType.REACTION,
    index=(IndexIn(columns=("parent_id", "owned_by_id", "content"), is_unique=True),),
)
class Reaction(
    IsGlobal,
    IsSpatial,
    IsReactable,
    IsDeletable,
    IsOwned,
    Entity,
):
    """A Reaction is a relationship between a Subject and a Reaction Node."""

    parent: Union["IsReactable", None] = builtin_property_parent()

    content: str = builtin_property(101, is_repr=True)
