from typing import TYPE_CHECKING

from destack.core import Entity, NodeType, declare_entity

if TYPE_CHECKING:
    pass


@declare_entity(NodeType.THEME)
class Theme(Entity):
    """A Theme with common Styles."""

    pass
