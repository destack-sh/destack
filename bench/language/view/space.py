from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    NAME_CONSTRAINT,
    BuiltinEnum,
    EnumType,
    IsModal,
    IsOwnable,
    IsTemplatable,
    LocalNodeList,
    Node,
    NodeType,
    PackageNode,
    Selection,
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
    from bench.language import Package, Page, Run, Text, Thread, View

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.SPACE_TYPE)
class SpaceType(BuiltinEnum):
    BROWSER = 10
    DESKTOP = 20
    MOBILE = 30


@node_(NodeType.SPACE)
class Space(IsOwnable, IsTemplatable, IsModal, PackageNode[SpaceData]):
    """A Space for a User to interact with a Bench."""

    parent: Optional["Package"] = p_node_parent(4, NodeType.PACKAGE)

    type: SpaceType = p_regular(30)
    name: str | None = p_regular(31, constraint=NAME_CONSTRAINT)
    text: Optional["Text"] = p_regular(32, default=None, struct=StructType.TEXT)
    order_key: str = p_internal(33, default=INTEGER_ZERO)

    focus: Optional[Selection] = p_regular(
        70,
        default=None,
        require=False,
        struct=StructType.SELECTION,
        description="The current main focus.",
    )
    selection: Optional[Selection] = p_regular(
        71,
        default=None,
        require=False,
        struct=StructType.SELECTION,
        description="The current selection of the Space.",
    )
    inspection: Optional[Node] = p_regular(
        73,
        default=None,
        require=False,
        array=False,
        references="any",
        description="The current inspected Node.",
    )
    container: Optional[Node] = p_regular(
        75,
        default=None,
        require=False,
        array=False,
        references="any",
        description="The current 'root' container Node (usually a parent of the inspected Node).",
    )
    page: Optional["Page"] = p_regular(
        76,
        default=None,
        require=False,
        array=False,
        references=NodeType.PAGE,
        description="The current Page.",
    )
    thread: Optional["Thread"] = p_regular(
        77,
        default=None,
        require=False,
        array=False,
        references=NodeType.THREAD,
        description="The current main Thread.",
    )
    run: Optional["Run"] = p_regular(
        78,
        default=None,
        require=False,
        array=False,
        references=NodeType.RUN,
    )

    views: LocalNodeList["View"] = p_node_children(NodeType.VIEW)
