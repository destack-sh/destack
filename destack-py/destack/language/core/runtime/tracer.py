from typing import TYPE_CHECKING

from destack.language.core import Handle, HandleType, builtin_handle, builtin_property_runtime

if TYPE_CHECKING:
    pass


@builtin_handle(HandleType.TRACER)
class Tracer(Handle):
    """A Tracer instruments SpanEvents."""

    name: str = builtin_property_runtime(401)
