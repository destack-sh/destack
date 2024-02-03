from copy import deepcopy
from typing import TYPE_CHECKING, Any, Optional

from bench.language.const import NodeType
from bench.language.node import (
    Package,
    _Passthrough,
    node,
    p_parent,
    p_internal,
    p_tracked,
)
from bench.language.value import HasValue
from bench.sql.core import PrimitiveType

if TYPE_CHECKING:
    from bench.language import Block


@node(NodeType.SIGNAL, passthrough=(("value", _Passthrough.Full),), index_in_os=True, local=True)
class Signal(HasValue):
    """A signal received in this Bench. May be emitted by a Bench or an external source."""

    parent: Package = p_parent(4, NodeType.PACKAGE)
    type: Optional["Block"] = p_internal(
        30, require=False, array=False, references=NodeType.BLOCK, index_in_pg=True
    )
    value_packed: Any | None = p_internal(
        31, default=None, copy=deepcopy, primitive_type=PrimitiveType.JSON
    )
    # sender_run: Optional["Run"] = struct_internal(32, require=False, array=False, references=NodeType.RUN)
    # sender_block: Optional["Block"] = struct_internal(
    #     33, require=False, array=False, references=NodeType.BLOCK
    # )
