from typing import TYPE_CHECKING

from destack.language.core import (
    IsEnvironmental,
    IsFrozen,
    IsInFolder,
    Json,
    Node,
    NodeType,
    node_,
    property_,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.LOG, pretend_frozen=True)
class Log(
    IsEnvironmental,
    IsFrozen,
    IsInFolder,
    Node,
):
    """A Log message."""

    content: str = property_(40)
    attributes: dict[str, Json] = property_(41)
