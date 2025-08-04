from typing import TYPE_CHECKING

from destack.core import Entity, NodeType, TraitType, declare_entity

if TYPE_CHECKING:
    pass


@declare_entity(
    NodeType.STYLE,
    is_abstract=True,
    traits=(TraitType.TEMPLATE,),
)
class Style(Entity):
    """A Style defines a base visual appearance in some context."""

    pass
