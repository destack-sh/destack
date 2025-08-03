from typing import TYPE_CHECKING

from destack.core import (
    Enum,
    EnumType,
    Event,
    NodeType,
    Value,
    builtin_enum,
    builtin_event,
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


@builtin_event(NodeType.LOG_EVENT)
class LogEvent(Event):
    """A Log message."""

    custom_values: dict[str, "Value"] | None = builtin_property(
        45,
        description="The custom Values of this Entity, keyed by custom Property name.",
    )

    name: str = builtin_property(101)
    level: LogLevel = builtin_property(110)
