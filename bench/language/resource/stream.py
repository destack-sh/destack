from typing import TYPE_CHECKING, Union

from bench.language.core import NodeType, node_, p_node_parent
from bench.language.resource.resource import DynamicResource
from bench.proto.wire.lang_pb2 import StreamData

if TYPE_CHECKING:
    from bench.language import Bench

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.STREAM)
class Stream(DynamicResource[StreamData]):
    """
    A Stream stored somewhere (like in a Drive, or externally).
    """

    parent: Union["Bench", None] = p_node_parent(4, NodeType.BENCH, is_system=True)
