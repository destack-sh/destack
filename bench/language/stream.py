from typing import Union

from bench.language.bench import Drive, PhysicalResourceNode
from bench.language.const import NodeType
from bench.language.node import node_
from bench.language.property import p_node_parent
from bench.proto.wire.lang_pb2 import StreamData

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.STREAM)
class Stream(PhysicalResourceNode[StreamData]):
    """
    A Stream stored somewhere (like in a Drive, or externally).
    """

    parent: Union["Drive", None] = p_node_parent(4, NodeType.DRIVE, is_system=True)
