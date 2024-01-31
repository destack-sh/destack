from typing import TYPE_CHECKING, Union

from bench.language.const import StructType, NodeType
from bench.language.node import (
    struct,
    Struct,
    struct_property,
    node_parent,
    struct_internal,
    node,
    Node,
    Package,
    node_children,
    node_component,
    ScopeNode,
)
from bench.language.validation import enum_validator
from bench.proto.core import ProtoStrEnum
from bench.utils.casing import IdentifierType

if TYPE_CHECKING:
    from bench.language import Block, Policy


class ViewType(ProtoStrEnum):
    # editor
    PAGE_EDITOR = "PAGE_EDITOR", 1
    BLOCK = "BLOCK", 2

    EXPLORER = "EXPLORER", 10
    HISTORY = "HISTORY", 11
    ISSUES = "ISSUES", 12
    TESTS = "TESTS", 13
    GLOBAL = "GLOBAL", 14
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
    views: list["View"] = node_children(NodeType.VIEW)


@node(NodeType.VIEW, identifier=IdentifierType.VARIABLE)
class View(ScopeNode, HasViews):
    """A view of a user interface in a Bench."""

    parent: Union["Space", "View", "Block"] = node_parent(
        4, NodeType.SPACE, NodeType.VIEW, NodeType.BLOCK
    )
    type: ViewType = struct_property(30, require=True, validate=enum_validator(ViewType))
    name: str = struct_property(31)


class BlockPageViewMode(ProtoStrEnum):
    NOTEBOOK = "NOTEBOOK", 1
    SCRIPT = "SCRIPT", 2


@node(NodeType.SPACE, identifier=IdentifierType.VARIABLE)
class Space(ScopeNode, HasViews):
    """A space for a user to interact with the Bench."""

    parent: Package = node_parent(4, NodeType.PACKAGE)
    policies: list["Policy"] | None = struct_internal(
        24, default_factory=list, struct=StructType.POLICY, array=True, sensitive=True
    )

    name: str = struct_property(31)
    order_key: str = struct_internal(32)
    # layout/views/...
    dock: "SpaceDock" = struct_internal(34, require=True, array=False, struct=StructType.SPACE_DOCK)


class SpaceDockItemType(ProtoStrEnum):
    # builtins
    SEARCH = "SEARCH", 1
    CHAT = "CHAT", 2
    ASSIST = "ASSIST", 3
    HELP = "HELP", 4
    # ...?


@struct(StructType.SPACE_DOCK_ITEM)
class SpaceDockItem(Struct):
    # type: SpaceDockItemType = struct_property(
    #     30, require=True, validate=enum_validator(SpaceDockItemType)
    # )
    hidden: bool = struct_property(31, default=False)


@struct(StructType.SPACE_DOCK)
class SpaceDock(Struct):
    items: list[SpaceDockItem] = struct_property(
        30, require=True, array=True, default_factory=list, struct=StructType.SPACE_DOCK_ITEM
    )
