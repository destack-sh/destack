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

    key: str = builtin_property(
        110,
        is_repr=True,
        description="The character that was pressed (e.g. 'a', 'B', '1', 'Enter').",
    )
    code: str = builtin_property(
        111,
        is_repr=True,
        description="The unaltered key code that was pressed (e.g. 'KeyA', 'KeyB', 'Digit1', 'Enter').",
    )
    is_repeat: bool = builtin_property(
        112,
        is_repr=True,
        description="Whether the key is being held down.",
    )
    is_redacted: bool = builtin_property(
        113,
        description="Whether the KeyboardEvent was masked for some reason (e.g., security, privacy).",
    )

    shift_key: bool = builtin_property(
        120,
        description="Whether the Shift key was held.",
    )
    alt_key: bool = builtin_property(
        121,
        description="Whether the Alt key was held.",
    )
    ctrl_key: bool = builtin_property(
        122,
        description="Whether the Ctrl key was held.",
    )
    meta_key: bool = builtin_property(
        123,
        description="Whether the Meta key was held.",
    )


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
