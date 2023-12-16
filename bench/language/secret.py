import typing
from typing import Optional

from bench.language import Node
from bench.language.builtin import _auto_async_to_sync
from bench.language.const import NodeType
from bench.language.module import node, struct_property, struct_runtime

SecretValueT = typing.TypeVar("SecretValueT")


@node(NodeType.SECRET, detached=True)
class Secret(Node, typing.Generic[SecretValueT]):
    """A proxy to a remotely stored secret."""

    sha512: str = struct_property(20)
    value: Optional[SecretValueT] = struct_runtime(default=None)

    def __str__(self):
        return f"{self.id} ({self.sha512[:8]})"

    def __repr__(self):
        return f"<Secret {self}>"

    @_auto_async_to_sync
    async def reveal(self) -> SecretValueT:
        if self.value is not None:
            return self.value
        self.value = await self.session._runtime.reveal_secret(self)
        return self.value
