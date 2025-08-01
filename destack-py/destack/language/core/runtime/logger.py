from typing import TYPE_CHECKING

from destack.language.core import (
    Handle,
    HandleType,
    NodeType,
    builtin_handle,
    builtin_property_runtime,
)

if TYPE_CHECKING:
    pass

# nocheckin(py): replace structlog with logging


@builtin_handle(
    HandleType.LOGGER,
    is_abstract=True,
    event_types=(NodeType.LOG_EVENT,),
)
class Logger(Handle):
    """A Logger logs LogEvents."""

    name: str = builtin_property_runtime(401)

    def trace(self, msg: str, **kwargs):
        pass

    def debug(self, msg: str, **kwargs):
        pass

    def info(self, msg: str, **kwargs):
        pass

    def warning(self, msg: str, **kwargs):
        pass

    def error(self, msg: str, **kwargs):
        pass

    def critical(self, msg: str, **kwargs):
        pass
