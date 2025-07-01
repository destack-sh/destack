from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    HasSlug,
    IsGlobal,
    NodeType,
    builtin_node,
    property_,
    property_parent_,
)

if TYPE_CHECKING:
    from destack.language import Space

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.HANDLE)
class Handle(IsGlobal, HasSlug, Entity):
    """A Destack @handle."""

    parent: Optional["Space"] = property_parent_(node_is_extensible=False)

    slug: str = property_(33, is_repr=True)
