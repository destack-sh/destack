from typing import TYPE_CHECKING

from destack.core import (
    EnumType,
    Event,
    NodeType,
    OptionEnum,
    Value,
    declare_enum,
    declare_event,
    declare_option,
    declare_property,
)

if TYPE_CHECKING:
    pass


@declare_enum(EnumType.LOG_LEVEL)
class LogLevel(OptionEnum):
    TRACE = declare_option(1, "Trace", description="A Trace")
    DEBUG = declare_option(2, "Debug", description="A Debug")
    INFO = declare_option(3, "Info", description="An Info")
    WARNING = declare_option(4, "Warning", description="A Warning")
    ERROR = declare_option(5, "Error", description="An Error")
    PANIC = declare_option(6, "Panic", description="A Panic")


@declare_event(NodeType.LOG_EVENT)
class LogEvent(Event):
    """A Log message."""

    custom_values: dict[str, "Value"] | None = declare_property(
        45,
        description="The custom Values of this Entity, keyed by custom Property name.",
    )

    name: str = declare_property(101)
    level: LogLevel = declare_property(110)
