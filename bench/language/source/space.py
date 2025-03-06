from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    NAME_CONSTRAINT,
    BuiltinEnum,
    EnumType,
    IsOwnable,
    LocalNodeList,
    Node,
    NodeType,
    Selection,
    SourceNode,
    StructType,
    enum_,
    node_,
    p_internal,
    p_node_children,
    p_node_parent,
    p_regular,
)
from bench.pb2 import SpaceData
from bench.utils.fractional import INTEGER_ZERO

if TYPE_CHECKING:
    from bench.language import (
        Channel,
        Package,
        Run,
        Text,
        Thread,
        View,
    )

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.SPACE_TYPE)
class SpaceType(BuiltinEnum):
    BROWSER = 10
    DESKTOP = 20
    MOBILE = 30


@node_(NodeType.SPACE)
class Space(IsOwnable, SourceNode[SpaceData]):
    """A Space for someone/something to interact with the Bench using Views."""

    parent: Optional["Package"] = p_node_parent(4, NodeType.PACKAGE)

    type: SpaceType = p_regular(30)
    name: str = p_regular(31, constraint=NAME_CONSTRAINT)
    text: Optional["Text"] = p_regular(32, default=None, struct=StructType.TEXT)
    order_key: str = p_internal(33, default=INTEGER_ZERO)

    focus: Optional[Selection] = p_regular(
        70, default=None, require=False, struct=StructType.SELECTION
    )
    selection: Optional[Selection] = p_regular(
        71, default=None, require=False, struct=StructType.SELECTION
    )
    inspection: Optional[Node] = p_regular(
        73, default=None, require=False, array=False, references="any"
    )
    channel: Optional["Channel"] = p_regular(
        75, default=None, require=False, array=False, references=NodeType.CHANNEL
    )
    thread: Optional["Thread"] = p_regular(
        76, default=None, require=False, array=False, references=NodeType.THREAD
    )
    run: Optional["Run"] = p_regular(
        77, default=None, require=False, array=False, references=NodeType.RUN
    )

    views: LocalNodeList["View"] = p_node_children(NodeType.VIEW)
