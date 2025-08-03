from destack.core import (
    NodeType,
    builtin_event,
)

from .input import InputEvent

# pyright: reportIncompatibleVariableOverride=false


@builtin_event(NodeType.FOCUS_EVENT, is_abstract=True)
class FocusEvent(InputEvent):
    """A FocusEvent is an InputEvent that corresponds to some direct user input with a focus."""

    pass


@builtin_event(NodeType.FOCUS_IN_EVENT)
class FocusInEvent(FocusEvent):
    """A FocusInEvent is a FocusEvent when a focus is gained."""

    pass


@builtin_event(NodeType.FOCUS_OUT_EVENT)
class FocusOutEvent(FocusEvent):
    """A FocusOutEvent is a FocusEvent when a focus is lost."""

    pass
