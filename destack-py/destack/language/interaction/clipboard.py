from destack.language.core import (
    NodeType,
    builtin_node,
)

from .input import InputEvent

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.CLIPBOARD_EVENT, frozen=True, is_abstract=True)
class ClipboardEvent(InputEvent):
    """A ClipboardEvent is an InputEvent that corresponds to some direct user input with a clipboard."""

    pass


@builtin_node(NodeType.COPY_EVENT, frozen=True)
class CopyEvent(ClipboardEvent):
    """A CopyEvent is a ClipboardEvent when a copy is performed."""

    pass


@builtin_node(NodeType.CUT_EVENT, frozen=True)
class CutEvent(ClipboardEvent):
    """A CutEvent is a ClipboardEvent when a cut is performed."""

    pass


@builtin_node(NodeType.PASTE_EVENT, frozen=True)
class PasteEvent(ClipboardEvent):
    """A PasteEvent is a ClipboardEvent when a paste is performed."""

    pass
