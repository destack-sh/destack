from typing import TYPE_CHECKING

from ..builtin import (
    Handle,
    HandleType,
    declare_handle,
)

if TYPE_CHECKING:
    pass


@declare_handle(HandleType.STREAM)
class Stream(Handle):
    """
    A Stream inside a Connection.
    """

    async def open(self) -> None:
        raise NotImplementedError

    async def close(self) -> None:
        raise NotImplementedError
