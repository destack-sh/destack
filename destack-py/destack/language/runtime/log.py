from typing import TYPE_CHECKING

from destack.language.core import (
    Analytic,
    IsFrozen,
    Json,
    Node,
    NodeType,
    Spatial,
    node_,
    property_,
)
from destack.pb2 import LogData

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.LOG, pretend_frozen=True)
class Log(
    Spatial,
    Analytic,
    IsFrozen,
    Node[LogData],
):
    """A Log message."""

    content: str = property_(40)
    attributes: dict[str, Json] = property_(41)
