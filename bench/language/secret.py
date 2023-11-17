import typing
from typing import Optional

from bench.language import Node
from bench.language.builtin import _auto_async_to_sync
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

    @_auto_async_to_sync
    async def reveal(self) -> SecretValueT:
        if self.value is not None:
            return self.value
        self.value = await self.session.runtime.reveal_secret(self)
        return self.value
