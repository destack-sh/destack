from typing import TYPE_CHECKING, Optional, final

from destack.language.core import (
    Entity,
    NodeType,
    builtin_entity,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import Space

# pyright: reportIncompatibleVariableOverride=false


@builtin_entity(NodeType.HANDLE, is_final=True)
@final
class Handle(Entity):
    """A Destack @handle."""

    parent: Optional["Space"] = builtin_property_parent()

    slug: str = builtin_property(101, is_repr=True)
