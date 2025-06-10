from typing import TYPE_CHECKING

from destack.language.core import (
    IsEnvironmental,
    IsFrozen,
    IsInSpace,
    IsParticle,
    Json,
    Node,
    NodeType,
    node_,
    property_,
)
from destack.pb2 import LogData

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.LOG, pretend_frozen=True)
class Log(
    IsEnvironmental,
    IsFrozen,
    IsInSpace,
    IsParticle,
    Node[LogData],
):
    """A Log message."""

    content: str = property_(40)
    attributes: dict[str, Json] = property_(41)
