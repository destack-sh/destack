from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Analytic,
    IsFrozen,
    Json,
    Node,
    NodeType,
    Spatial,
    node_,
    property_,
    property_parent_,
)
from destack.pb2 import LogData

if TYPE_CHECKING:
    from destack.language import Space

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.LOG, pretend_frozen=True)
class Log(
    Spatial,
    Analytic,
    IsFrozen,
    Node[LogData],
):
    """A Log message."""

    parent: Optional["Space"] = property_parent_(node_is_customizable=False)
    content: str = property_(40)
    attributes: dict[str, Json] = property_(41)
