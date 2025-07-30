from typing import TYPE_CHECKING, Optional, final

from destack.language.core import (
    Entity,
    NodeReference,
    NodeType,
    TraitType,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import Handle, Space

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(
    NodeType.ORGANIZATION,
    is_final=True,
    traits=(TraitType.ACTOR, TraitType.JOINABLE),
)
@final
class Organization(Entity):
    """
    An Organization with Users and Teams.
    """

    parent: Optional["Space"] = builtin_property_parent()
    slug: str = builtin_property(101, is_repr=True)
    handle: Optional["Handle"] = builtin_property(111)
    if TYPE_CHECKING:
        handle_ptr: Optional[NodeReference] = None
