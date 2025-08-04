from typing import TYPE_CHECKING

from destack.core import (
    Entity,
    NodeType,
    TraitType,
    declare_entity,
    declare_property,
)

if TYPE_CHECKING:
    pass


@declare_entity(NodeType.SCRIPT, traits=(TraitType.ORDERED,))
class Script(Entity):
    """A Script."""

    code: str = declare_property(110)
    # type, language, code, ...
