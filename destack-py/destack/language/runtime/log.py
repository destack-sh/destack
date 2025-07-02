from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Enum,
    EnumType,
    Event,
    Json,
    NodeType,
    builtin_enum,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

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


@builtin_node(NodeType.LOG_EVENT, pretend_frozen=True)
class LogEvent(Event):
    """A Log message."""

    parent: Optional["Space"] = builtin_property_parent(node_is_extensible=False)
    content: str = builtin_property(40)
    attributes: dict[str, Json] = builtin_property(41)
    level: LogLevel = builtin_property(42)
