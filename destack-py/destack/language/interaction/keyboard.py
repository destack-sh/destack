from destack.language.core import (
    NodeType,
    builtin_node,
    builtin_property,
)

from .input import InputEvent

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.KEYBOARD_EVENT, frozen=True, is_abstract=True)
class KeyboardEvent(InputEvent):
    """A KeyboardEvent is an InputEvent that corresponds to some direct user input with a keyboard."""

    key: str = builtin_property(110)
    code: str = builtin_property(111)
    repeat: bool = builtin_property(112)

    shift_key: bool = builtin_property(120)
    alt_key: bool = builtin_property(121)
    ctrl_key: bool = builtin_property(122)
    meta_key: bool = builtin_property(123)


@builtin_node(NodeType.KEY_DOWN_EVENT, frozen=True)
class KeyDownEvent(KeyboardEvent):
    """A KeyDownEvent is a KeyboardEvent when a key is pressed down."""

    pass


@builtin_node(NodeType.KEY_UP_EVENT, frozen=True)
class KeyUpEvent(KeyboardEvent):
    """A KeyUpEvent is a KeyboardEvent when a key is released."""

    pass


@builtin_node(NodeType.KEY_PRESS_EVENT, frozen=True)
class KeyPressEvent(KeyboardEvent):
    """A KeyPressEvent is a KeyboardEvent when a key is pressed."""

    pass
