from typing import TYPE_CHECKING

from destack.core import (
    Event,
    NodeType,
    declare_event,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@declare_event(NodeType.INPUT_EVENT, is_abstract=True)
class InputEvent(Event):
    """An InputEvent is an Event that corresponds to some direct user input."""

    # is_handled?
