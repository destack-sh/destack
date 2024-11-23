from typing import Any, Union

from bench.language.bench import AnonymousResourceNode, Drive
from bench.language.const import NodeType
from bench.language.node import node_
from bench.language.property import p_node_parent

# pyright: reportIncompatibleVariableOverride=false

StreamData = Any  # nocheckin


@node_(NodeType.STREAM)
class Stream(AnonymousResourceNode[StreamData]):
    """
    A Stream stored somewhere (like in a Drive, or externally).
    """

    parent: Union["Drive", None] = p_node_parent(4, NodeType.DRIVE, is_system=True)
