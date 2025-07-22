from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    Event,
    NodeType,
    builtin_node,
    builtin_property,
)

if TYPE_CHECKING:
    from destack.language import View

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(
    NodeType.INPUT_EVENT,
    frozen=True,  # type: ignore (frozen)
    is_abstract=True,
)
class InputEvent[NodeT: View = View](Event[NodeT]):
    """An InputEvent is an Event that corresponds to some direct user input."""

    node: Optional["Entity"] = builtin_property(101)
    # is_handled?
