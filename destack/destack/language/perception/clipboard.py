from destack.language.core import (
    NodeType,
    builtin_event,
)

from .input import InputEvent

# pyright: reportIncompatibleVariableOverride=false


@builtin_event(NodeType.CLIPBOARD_EVENT, is_abstract=True)
class ClipboardEvent(InputEvent):
    """A ClipboardEvent is an InputEvent that corresponds to some direct user input with a clipboard."""

    pass


@builtin_event(NodeType.COPY_EVENT)
class CopyEvent(ClipboardEvent):
    """A CopyEvent is a ClipboardEvent when a copy is performed."""

    pass


@builtin_event(NodeType.CUT_EVENT)
class CutEvent(ClipboardEvent):
    """A CutEvent is a ClipboardEvent when a cut is performed."""

    pass


@builtin_event(NodeType.PASTE_EVENT)
class PasteEvent(ClipboardEvent):
    """A PasteEvent is a ClipboardEvent when a paste is performed."""

    pass
