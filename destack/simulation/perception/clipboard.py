from destack.core import (
    NodeType,
    declare_event,
)

from .input import InputEvent

# pyright: reportIncompatibleVariableOverride=false


@declare_event(NodeType.CLIPBOARD_EVENT, is_abstract=True)
class ClipboardEvent(InputEvent):
    """A ClipboardEvent is an InputEvent that corresponds to some direct user input with a clipboard."""

    pass


@declare_event(NodeType.COPY_EVENT)
class CopyEvent(ClipboardEvent):
    """A CopyEvent is a ClipboardEvent when a copy is performed."""

    pass


@declare_event(NodeType.CUT_EVENT)
class CutEvent(ClipboardEvent):
    """A CutEvent is a ClipboardEvent when a cut is performed."""

    pass


@declare_event(NodeType.PASTE_EVENT)
class PasteEvent(ClipboardEvent):
    """A PasteEvent is a ClipboardEvent when a paste is performed."""

    pass
