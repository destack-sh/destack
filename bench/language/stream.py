from typing import TYPE_CHECKING, Union

from bench.language.bench import DynamicResource
from bench.language.const import NodeType
from bench.language.node import node_
from bench.language.property import p_node_parent
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
