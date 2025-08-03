from typing import TYPE_CHECKING

from destack.core import (
    EnumDeclaration,
    EnumType,
    Event,
    NodeType,
    Value,
    declare_enum,
    declare_event,
    declare_property,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@declare_enum(EnumType.LOG_LEVEL)
class LogLevel(EnumDeclaration):
    TRACE = 1
    DEBUG = 2
    INFO = 3
    WARNING = 4
    ERROR = 5
    PANIC = 6


@declare_event(NodeType.LOG_EVENT)
class LogEvent(Event):
    """A Log message."""

    custom_values: dict[str, "Value"] | None = declare_property(
        45,
        description="The custom Values of this Entity, keyed by custom Property name.",
    )

    name: str = declare_property(101)
    level: LogLevel = declare_property(110)
