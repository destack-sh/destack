from typing import TYPE_CHECKING

from destack.language.core import (
    Enum,
    EnumType,
    Event,
    Json,
    NodeType,
    builtin_enum,
    builtin_node,
    builtin_property,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.LOG_LEVEL)
class LogLevel(Enum):
    TRACE = 1
    DEBUG = 2
    INFO = 3
    WARNING = 4
    ERROR = 5
    PANIC = 6


@builtin_node(NodeType.LOG_EVENT, frozen=True)
class LogEvent(Event):
    """A Log message."""

    content: str = builtin_property(110)
    attributes: dict[str, Json] = builtin_property(111)
    level: LogLevel = builtin_property(112)
