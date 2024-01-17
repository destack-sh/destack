from copy import deepcopy
from typing import TYPE_CHECKING, Any, Optional

from bench.language.const import NodeType
from bench.language.node import Bench, node, node_parent, struct_internal, struct_property, Module
from bench.language.value import HasValue
from bench.sql.core import ColumnType
from bench.utils.func import _auto_async_to_sync

if TYPE_CHECKING:
    from bench.language import Statement


@node(NodeType.SECRET, in_module=True, in_bench=True)
class Secret(HasValue):
    """A shared secret with a deferred value (loaded on demand)t."""

    parent: Module = node_parent(4, NodeType.MODULE)
    type: Optional["Statement"] = struct_internal(
        30, require=False, array=False, references=NodeType.STATEMENT
    )
    value: Any | None = struct_property(
        31,
        default_factory=dict,
        copy=deepcopy,
        column_type=ColumnType.JSON,
        encrypt=True,
        ignore_conflicts_with=(HasValue,),
    )
    sha512: str = struct_internal(32)
    name: Optional[str] = struct_property(33)

    def __str__(self):
        return f"{self.id} ({self.sha512[:8]})"

    def __repr__(self):
        return f"<Secret {self}>"

    @_auto_async_to_sync
    async def reveal(self) -> Any:
        if self.value is not None:
            return self.value
        self.value = await self.session.host.reveal_secret(self)
        return self.value
