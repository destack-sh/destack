from typing import TYPE_CHECKING

from destack.language.core import (
    Entity,
    NodeType,
    TraitType,
    builtin_entity,
    builtin_property,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_entity(NodeType.SCRIPT, traits=(TraitType.ORDERED,))
class Script(Entity):
    """A Script."""

    code: str = builtin_property(110)
    # type, language, code, ...
