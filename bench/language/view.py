from typing import TYPE_CHECKING, Optional, Union

from bench.language.const import NodeType, StructType
from bench.language.node import (
    Node,
    Struct,
    node,
    node_component,
    p_child,
    p_internal,
    p_parent,
    p_regular,
    struct,
)
from bench.language.validation import enum_validator
from bench.utils.casing import IdentifierType
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import Block, Icon, Package, Policy, RichText


class ViewType(IdEnum):
    # editor
    PAGE = 1
    BLOCK = 2

    EXPLORER = 10
    HISTORY = 11
    WATCH = 12
    TESTING = 13
    BENCH = 14
    INSPECTOR = 15
    LIBRARY = 16
    ACCESS = 17

    # containers
    WINDOW_GROUP = 40
    WINDOW = 41
    PANEL = 42
    # ...

    # separators
    SPACER = 60
    DIVIDER = 61

    # controls
    # ...


@node_component
class HasViews(Node):
    views: list["View"] = p_child(NodeType.VIEW)


@node(NodeType.VIEW, identifier=IdentifierType.VARIABLE)
class View(HasViews):
    """A view of a user interface in a Bench."""

    parent: Union["Space", "View", "Block"] = p_parent(
        4, NodeType.SPACE, NodeType.VIEW, NodeType.BLOCK
    )
    type: ViewType = p_regular(30, require=True, validate=enum_validator(ViewType))
    name: Optional[str] = p_regular(31, default=None)
    icon: Optional["Icon"] = p_regular(
        32, default=None, require=False, array=False, struct=StructType.ICON
    )


class PageViewMode(IdEnum):
    NOTEBOOK = 1
    SCRIPT = 2


@node(NodeType.SPACE, identifier=IdentifierType.VARIABLE)
class Space(HasViews):
    """A space for a user to interact with the Bench."""

    parent: "Package" = p_parent(4, NodeType.PACKAGE)

    name: str = p_regular(31)
    text: Optional["RichText"] = p_regular(32, default=None, struct=StructType.RICH_TEXT)
    order_key: str = p_internal(33)
    policies: list["Policy"] | None = p_regular(
        34, default_factory=list, struct=StructType.POLICY, array=True
    )
    # layout/views/...
    dock: "SpaceDock" = p_regular(35, require=True, array=False, struct=StructType.SPACE_DOCK)


class SpaceDockItemType(IdEnum):
    # builtins
    SEARCH = 1
    CHAT = 2
    ASSIST = 3
    HELP = 4
    # ...?


@struct(StructType.SPACE_DOCK_ITEM)
class SpaceDockItem(Struct):
    # type: SpaceDockItemType = ...
    hidden: bool = p_regular(31, default=False)


@struct(StructType.SPACE_DOCK)
class SpaceDock(Struct):
    items: list[SpaceDockItem] = p_regular(
        30, require=True, array=True, default_factory=list, struct=StructType.SPACE_DOCK_ITEM
    )
