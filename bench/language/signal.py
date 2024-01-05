from copy import deepcopy
from typing import TYPE_CHECKING, Any, Optional

from bench.language.const import NodeType
from bench.language.module import (
    node,
    node_parent,
    struct_internal,
    struct_property,
    Module,
)
from bench.language.value import HasValue
from bench.sql.core import ColumnType

if TYPE_CHECKING:
    from bench.language import Run, Statement


@node(NodeType.SIGNAL, local=True, detached=True, root=NodeType.BENCH, index_in_os=True)
class Signal(HasValue):
    parent: Module = node_parent(4, NodeType.MODULE)
    type: Optional["Statement"] = struct_internal(30, array=False, references=NodeType.STATEMENT)
    value: Any | None = struct_property(
        31, default_factory=dict, copy=deepcopy, column_type=ColumnType.JSON
    )
    source_run: Optional["Run"] = struct_internal(32, array=False, references=NodeType.RUN)
    source_statement: Optional["Statement"] = struct_internal(
        33, array=False, references=NodeType.STATEMENT
    )
