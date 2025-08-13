from typing import TYPE_CHECKING

if TYPE_CHECKING:
    pass


class Stream:
    """
    A Stream inside a Connection.
    """

    async def open(self) -> None:
        raise NotImplementedError

    async def close(self) -> None:
        raise NotImplementedError
