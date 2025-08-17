from typing import TYPE_CHECKING

from destack.core import Entity, NodeType, declare_entity

if TYPE_CHECKING:
    pass


@declare_entity(NodeType.ENVIRONMENT)
class Environment(Entity):
    """An Environment is a usage scenario."""

    pass
