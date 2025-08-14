from typing import TYPE_CHECKING

from ..builtin import Handle, HandleType, declare_handle, declare_property_runtime

if TYPE_CHECKING:
    pass


@declare_handle(HandleType.TRACER)
class Tracer(Handle):
    """A Tracer instruments SpanEvents."""

    name: str = declare_property_runtime(401)
