from typing import TYPE_CHECKING

from destack.language.core import (
    Event,
    NodeType,
    builtin_event,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_event(NodeType.INPUT_EVENT, is_abstract=True)
class InputEvent(Event):
    """An InputEvent is an Event that corresponds to some direct user input."""

    # is_handled?
