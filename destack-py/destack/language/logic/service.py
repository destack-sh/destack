from typing import TYPE_CHECKING

from destack.language.core import (
    Entity,
    NodeType,
    TraitType,
    builtin_action,
    builtin_node,
    builtin_property,
)

if TYPE_CHECKING:
    from destack.language import Icon


# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@builtin_node(
    NodeType.SERVICE,
    traits=(TraitType.OWNABLE, TraitType.ACTOR, TraitType.RUNNABLE),
)
class Service(Entity):
    """
    A Service provides related functionality via Actions.
    """

    icon: "Icon | None" = builtin_property(102)

    @builtin_action(101)
    async def start(self):
        """Start the Service."""
        ...

    @builtin_action(102)
    async def stop(self):
        """Stop the Service."""
        ...
