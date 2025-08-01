from typing import TYPE_CHECKING

from ..builtin import Handle, HandleType, NodeReference, builtin_handle, builtin_property_runtime

if TYPE_CHECKING:
    pass


@builtin_handle(HandleType.STREAM)
class Stream(Handle):
    """
    A Stream inside a Connection.
    """

    space_ptr: "NodeReference" = builtin_property_runtime(500, is_repr=True)

    async def open(self) -> None:
        raise NotImplementedError

    async def close(self) -> None:
        raise NotImplementedError
