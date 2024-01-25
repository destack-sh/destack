from copy import deepcopy
from typing import TYPE_CHECKING, Any, Optional

from bench.language.const import NodeType
from bench.language.node import (
    Module,
    node,
    node_parent,
    struct_internal,
    struct_property,
    _Passthrough,
)
from bench.language.value import HasValue
from bench.sql.core import ColumnType

if TYPE_CHECKING:
    from bench.language import Statement


@node(NodeType.SIGNAL, local=True, index_in_os=True, passthrough=(("value", _Passthrough.Full),))
class Signal(HasValue):
    """A signal received in this Bench. May be emitted by a Bench or an external source."""

    parent: Module = node_parent(4, NodeType.MODULE)
    type: Optional["Statement"] = struct_internal(
        30, require=False, array=False, references=NodeType.STATEMENT, index_in_pg=True
    )
    value_packed: Any | None = struct_property(
        31, default=None, copy=deepcopy, column_type=ColumnType.JSON
    )
    # source_run: Optional["Run"] = struct_internal(32, require=False, array=False, references=NodeType.RUN)
    # source_statement: Optional["Statement"] = struct_internal(
    #     33, require=False, array=False, references=NodeType.STATEMENT
    # )
