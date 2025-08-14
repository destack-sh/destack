from destack.core import (
    NodeType,
    declare_event,
    declare_property,
)

from .input import InputEvent


@declare_event(NodeType.KEY_EVENT, is_abstract=True)
class KeyEvent(InputEvent):
    """A KeyEvent is an InputEvent that corresponds to some direct user input with a keyboard."""

    key: str = declare_property(
        110,
        is_repr=True,
        is_interned=True,
        description="The character that was pressed (e.g. 'a', 'B', '1', 'Enter').",
        tag=None,
    )
    code: str = declare_property(
        111,
        is_repr=True,
        is_interned=True,
        description="The unaltered key code that was pressed (e.g. 'KeyA', 'KeyB', 'Digit1', 'Enter').",
        tag=None,
    )
    is_repeat: bool = declare_property(
        112,
        is_repr=True,
        description="Whether the key is being held down.",
        tag=None,
    )
    is_redacted: bool = declare_property(
        113,
        description="Whether the key was masked for some reason (e.g., security, privacy).",
        tag=None,
    )

    shift_key: bool = declare_property(
        120,
        description="Whether the Shift key was held.",
        tag=None,
    )
    alt_key: bool = declare_property(
        121,
        description="Whether the Alt key was held.",
        tag=None,
    )
    ctrl_key: bool = declare_property(
        122,
        description="Whether the Ctrl key was held.",
        tag=None,
    )
    meta_key: bool = declare_property(
        123,
        description="Whether the Meta key was held.",
        tag=None,
    )


@declare_event(NodeType.KEY_DOWN_EVENT)
class KeyDownEvent(KeyEvent):
    """A KeyDownEvent is a KeyboardEvent when a key is pressed down."""

    pass


@declare_event(NodeType.KEY_UP_EVENT)
class KeyUpEvent(KeyEvent):
    """A KeyUpEvent is a KeyboardEvent when a key is released."""

    pass


@declare_event(NodeType.KEY_PRESS_EVENT)
class KeyPressEvent(KeyEvent):
    """A KeyPressEvent is a KeyboardEvent when a key is pressed."""

    pass
