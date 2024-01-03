from typing import Any, Optional

from bench.language import Node
from bench.language.builtin import _auto_async_to_sync
from bench.language.const import NodeType
from bench.language.module import node, struct_internal, struct_property
from bench.sql.core import ColumnType


@node(NodeType.SECRET, detached=True)
class Secret(Node):
    """A shared secret with a deferred value (loaded on demand)t."""

    sha512: str = struct_property(30)
    value: Optional[str] = struct_internal(31, default=None, defer=True, encrypt=True)

    def __str__(self):
        return f"{self.id} ({self.sha512[:8]})"

    def __repr__(self):
        return f"<Secret {self}>"

    @_auto_async_to_sync
    async def reveal(self) -> Any:
        if self.value is not None:
            return self.value
        self.value = await self.session._runtime.reveal_secret(self)
        return self.value
