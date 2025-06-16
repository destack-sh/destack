from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Analytic,
    Enum,
    EnumType,
    IsFrozen,
    Json,
    Node,
    NodeType,
    Spatial,
    builtin_enum,
    builtin_node,
    property_,
    property_parent_,
)
from destack.proto import LogData

if TYPE_CHECKING:
    from destack.language import Space

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.LOG_LEVEL)
class LogLevel(Enum):
    TRACE = 1
    DEBUG = 2
    INFO = 3
    WARNING = 4
    ERROR = 5
    PANIC = 6


@builtin_node(NodeType.LOG, pretend_frozen=True)
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
    level: LogLevel = property_(42)
