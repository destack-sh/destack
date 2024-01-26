from copy import deepcopy
from typing import TYPE_CHECKING, Any, Optional

from bench.language.const import NodeType
from bench.language.node import (
    Package,
    _Passthrough,
    node,
    node_parent,
    struct_internal,
    struct_property,
)
from bench.language.value import HasValue
from bench.sql.core import PrimitiveType

if TYPE_CHECKING:
    from bench.language import Block


@node(NodeType.SIGNAL, local=True, index_in_os=True, passthrough=(("value", _Passthrough.Full),))
class Signal(HasValue):
    """A signal received in this Bench. May be emitted by a Bench or an external source."""

    parent: Package = node_parent(4, NodeType.PACKAGE)
    type: Optional["Block"] = struct_internal(
        30, require=False, array=False, references=NodeType.BLOCK, index_in_pg=True
    )
    value_packed: Any | None = struct_property(
        31, default=None, copy=deepcopy, primitive_type=PrimitiveType.JSON
    )
    # sender_run: Optional["Run"] = struct_internal(32, require=False, array=False, references=NodeType.RUN)
    # sender_block: Optional["Block"] = struct_internal(
    #     33, require=False, array=False, references=NodeType.BLOCK
    # )
