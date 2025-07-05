from destack.language.core import (
    NodeType,
    builtin_node,
)

from .input import InputEvent

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.FOCUS_EVENT, frozen=True, is_abstract=True)
class FocusEvent(InputEvent):
    """A FocusEvent is an InputEvent that corresponds to some direct user input with a focus."""

    pass


@builtin_node(NodeType.FOCUS_IN_EVENT, frozen=True)
class FocusInEvent(FocusEvent):
    """A FocusInEvent is a FocusEvent when a focus is gained."""

    pass


@builtin_node(NodeType.FOCUS_OUT_EVENT, frozen=True)
class FocusOutEvent(FocusEvent):
    """A FocusOutEvent is a FocusEvent when a focus is lost."""

    pass
