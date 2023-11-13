import typing
from typing import Optional

from asgiref.sync import async_to_sync

from bench.language import Node
from bench.language.const import MNT
from bench.language.module import node, nproperty, nruntime

SecretValueT = typing.TypeVar("SecretValueT")


@node(MNT.SECRET)
class Secret(Node, typing.Generic[SecretValueT]):
    """A proxy to a remotely stored secret."""

    sha512: str = nproperty()
    value: Optional[SecretValueT] = nruntime(default=None)

    def __str__(self):
        return f"{self.id} ({self.sha512[:8]})"

    def __repr__(self):
        return f"<Secret {self}>"

    async def areveal(self) -> SecretValueT:
        if self.value is not None:
            return self.value
        self.value = await self.session.runtime.reveal_secret(self)
        return self.value

    def reveal(self) -> SecretValueT:
        return async_to_sync(self.areveal)()
