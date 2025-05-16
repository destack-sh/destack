from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    NAME_CONSTRAINT,
    BuiltinEnum,
    EnumType,
    IsModal,
    IsOrdered,
    IsOwnable,
    IsTemplatable,
    Node,
    NodeType,
    PackageNode,
    Selection,
    enum_,
    node_,
    p_node_parent,
    p_regular,
)
from bench.pb2 import SpaceData

if TYPE_CHECKING:
    from bench.language import Package, Page, Thread

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.SPACE_TYPE)
class SpaceType(BuiltinEnum):
    BROWSER = 10
    DESKTOP = 20
    MOBILE = 30


@node_(NodeType.SPACE)
class Space(IsOwnable, IsTemplatable, IsModal, IsOrdered, PackageNode[SpaceData]):
    """
    A Space for a User to interact with a Bench.
    Spaces to all Benches are stored in the owning User's Bench.
    """

    parent: Optional["Package"] = p_node_parent(4, NodeType.PACKAGE)

    type: SpaceType = p_regular(30)
    name: str | None = p_regular(31, constraint=NAME_CONSTRAINT)

    selection: Optional[Selection] = p_regular(
        70,
        default=None,
        description="The current selection of the Space.",
    )
    focus: Optional[Node] = p_regular(
        71,
        default=None,
        description="The current main focus.",
    )
    inspection: Optional[Node] = p_regular(
        72,
        default=None,
        description="The current inspected Node.",
    )
    container: Optional[Node] = p_regular(
        73,
        default=None,
        description="The current 'root' container Node.",
    )
    page: Optional["Page"] = p_regular(
        74,
        default=None,
        description="The current Page.",
    )
    thread: Optional["Thread"] = p_regular(
        75,
        default=None,
        description="The current Thread.",
    )
