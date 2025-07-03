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
    builtin_property,
    builtin_property_parent,
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

    parent: Union["IsStarable", None] = builtin_property_parent()
    owned_by: "IsSubject" = builtin_property(28)
