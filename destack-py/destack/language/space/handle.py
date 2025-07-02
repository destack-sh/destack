from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    HasSlug,
    IsGlobal,
    NodeType,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import Space

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.HANDLE)
class Handle(IsGlobal, HasSlug, Entity):
    """A Destack @handle."""

    parent: Optional["Space"] = builtin_property_parent(node_is_extensible=False)

    slug: str = builtin_property(33, is_repr=True)
