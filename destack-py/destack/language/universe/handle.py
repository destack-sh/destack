from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    NodeType,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import Space

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.HANDLE)
class Handle(Entity):
    """A Destack @handle."""

    parent: Optional["Space"] = builtin_property_parent()

    slug: str = builtin_property(101, is_repr=True)
