from typing import TYPE_CHECKING, Optional, Union

from bench.language.const import NodeType, StructType
from bench.language.node import (
    Node,
    Package,
    ScopeNode,
    Struct,
    node,
    node_component,
    p_child,
    p_internal,
    p_parent,
    p_tracked,
    struct,
)
from bench.language.validation import enum_validator
from bench.utils.func import IdStrEnum
from bench.utils.casing import IdentifierType

if TYPE_CHECKING:
    from bench.language import Block, Icon, Policy


class ViewType(IdStrEnum):
    # editor
    PAGE = "PAGE", 1
    BLOCK = "BLOCK", 2

    EXPLORER = "EXPLORER", 10
    HISTORY = "HISTORY", 11
    WATCH = "WATCH", 12
    TESTING = "TESTING", 13
    BENCH = "BENCH", 14
    INSPECTOR = "INSPECTOR", 15
    LIBRARY = "LIBRARY", 16
    ACCESS = "ACCESS", 17

    # containers
    WINDOW_GROUP = "WINDOW_GROUP", 40
    WINDOW = "WINDOW", 41
    PANEL = "PANEL", 42
    # ...

    # separators
    SPACER = "SPACER", 60
    DIVIDER = "DIVIDER", 61

    # controls
    # ...


@node_component
class HasViews(Node):
    views: list["View"] = p_child(NodeType.VIEW)


@node(NodeType.VIEW, identifier=IdentifierType.VARIABLE)
class View(ScopeNode, HasViews):
    """A view of a user interface in a Bench."""

    parent: Union["Space", "View", "Block"] = p_parent(
        4, NodeType.SPACE, NodeType.VIEW, NodeType.BLOCK
    )
    type: ViewType = p_tracked(30, require=True, validate=enum_validator(ViewType))
    name: Optional[str] = p_tracked(31, default=None)
    icon: Optional["Icon"] = p_tracked(
        32, default=None, require=False, array=False, struct=StructType.ICON
    )


class PageViewMode(IdStrEnum):
    NOTEBOOK = "NOTEBOOK", 1
    SCRIPT = "SCRIPT", 2


@node(NodeType.SPACE, identifier=IdentifierType.VARIABLE)
class Space(ScopeNode, HasViews):
    """A space for a user to interact with the Bench."""

    parent: Package = p_parent(4, NodeType.PACKAGE)
    policies: list["Policy"] | None = p_tracked(
        24, default_factory=list, struct=StructType.POLICY, array=True
    )

    name: str = p_tracked(31)
    order_key: str = p_internal(32)
    # layout/views/...
    dock: "SpaceDock" = p_tracked(34, require=True, array=False, struct=StructType.SPACE_DOCK)


class SpaceDockItemType(IdStrEnum):
    # builtins
    SEARCH = "SEARCH", 1
    CHAT = "CHAT", 2
    ASSIST = "ASSIST", 3
    HELP = "HELP", 4
    # ...?


@struct(StructType.SPACE_DOCK_ITEM)
class SpaceDockItem(Struct):
    # type: SpaceDockItemType = p_tracked(
    #     30, require=True, validate=enum_validator(SpaceDockItemType)
    # )
    hidden: bool = p_tracked(31, default=False)


@struct(StructType.SPACE_DOCK)
class SpaceDock(Struct):
    items: list[SpaceDockItem] = p_tracked(
        30, require=True, array=True, default_factory=list, struct=StructType.SPACE_DOCK_ITEM
    )
