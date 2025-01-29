from typing import TYPE_CHECKING, Union

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    NodeType,
    enum_,
    node_,
    p_internal,
    p_node_parent,
)
from bench.pb2.lang_pb2 import StreamData

from .resource import DynamicResource

if TYPE_CHECKING:
    from bench.language import Bench

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.STREAM_TYPE)
class StreamType(BuiltinEnum):
    TEXT = 1
    CODE = 2
    IMAGE = 3
    AUDIO = 4
    VIDEO = 5


@node_(NodeType.STREAM, has_subtypes=True)
class Stream(DynamicResource[StreamData]):
    """
    A Stream stored somewhere (like in a Drive, or externally).
    """

    parent: Union["Bench", None] = p_node_parent(4, NodeType.BENCH, is_system=True)

    type: StreamType = p_internal(30)
