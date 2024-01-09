from copy import deepcopy
from typing import TYPE_CHECKING, Any, Optional

from bench.language.builtin import _match_session_sync
from bench.language.const import NodeType
from bench.language.node import Bench, Node, node, node_parent, struct_internal, struct_property
from bench.language.value import HasValue
from bench.sql.core import ColumnType

if TYPE_CHECKING:
    from bench.language import Statement


@node(NodeType.SECRET, in_module=False)
class Secret(HasValue):
    """A shared secret with a deferred value (loaded on demand)t."""

    parent: Bench = node_parent(4, NodeType.BENCH)
    type: Optional["Statement"] = struct_internal(30, array=False, references=NodeType.STATEMENT)
    value: Any | None = struct_property(
        31,
        default_factory=dict,
        copy=deepcopy,
        column_type=ColumnType.JSON,
        encrypt=True,
        ignore_conflicts_with=(HasValue,),
    )
    sha512: str = struct_internal(32)

    def __str__(self):
        return f"{self.id} ({self.sha512[:8]})"

    def __repr__(self):
        return f"<Secret {self}>"

    @_match_session_sync
    async def reveal(self) -> Any:
        if self.value is not None:
            return self.value
        self.value = await self.session._host.reveal_secret(self)
        return self.value
