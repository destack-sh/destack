from copy import deepcopy
from typing import TYPE_CHECKING, Any, Optional

from bench.language.const import NodeType
from bench.language.module import (
    Node,
    node,
    node_ancestor,
    node_parent,
    struct_internal,
    struct_property,
)
from bench.language.value import HasValue
from bench.sql.core import ColumnType

if TYPE_CHECKING:
    from bench.language import Run, Session, Statement


@node(NodeType.SIGNAL, local=True, detached=True, index_in_os=True)
class Signal(HasValue):
    parent: None = node_parent(4)
    type: Optional["Statement"] = struct_internal(30, array=False, references=NodeType.STATEMENT)
    value: Any | None = struct_property(
        31, default_factory=dict, copy=deepcopy, column_type=ColumnType.JSON
    )
    source_run: Optional["Run"] = struct_internal(32, array=False, references=NodeType.RUN)
    source_statement: Optional["Statement"] = struct_internal(
        33, array=False, references=NodeType.STATEMENT
    )
    # (placeholder)


@node(NodeType.HALT, local=True)
class Halt(Node):
    parent: "Run" = node_parent(4, NodeType.RUN)
    session: "Session" = node_ancestor(30, NodeType.SESSION, store=True)
    # (placeholder)
