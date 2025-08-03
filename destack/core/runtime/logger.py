from typing import TYPE_CHECKING

from destack.core import (
    Handle,
    HandleType,
    NodeType,
    declare_handle,
    declare_method,
    declare_property_runtime,
)

if TYPE_CHECKING:
    from destack import LogEvent


@declare_handle(
    HandleType.LOGGER,
    is_abstract=True,
    event_types=(NodeType.LOG_EVENT,),
)
class Logger(Handle):
    """A Logger logs LogEvents."""

    name: str = declare_property_runtime(401)

    @declare_method(101)
    def trace(self, name: str, **kwargs) -> "LogEvent":
        """Log a trace event with additional custom Values."""
        ...

    @declare_method(102)
    def debug(self, name: str, **kwargs) -> "LogEvent":
        """Log a debug event with additional custom Values."""
        ...

    @declare_method(103)
    def info(self, name: str, **kwargs) -> "LogEvent":
        """Log an info event with additional custom Values."""
        ...

    @declare_method(104)
    def warning(self, name: str, **kwargs) -> "LogEvent":
        """Log a warning event with additional custom Values."""
        ...

    @declare_method(105)
    def error(self, name: str, **kwargs) -> "LogEvent":
        """Log an error event with additional custom Values."""
        ...

    @declare_method(106)
    def critical(self, name: str, **kwargs) -> "LogEvent":
        """Log a critical event with additional custom Values."""
        ...
