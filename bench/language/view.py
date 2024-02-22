from typing import TYPE_CHECKING, Optional, Union

from bench.language.const import NodeType, StructType
from bench.language.node import Node, Struct, node, node_component, struct
from bench.language.property import p_internal, p_node_child, p_node_parent, p_regular
from bench.language.validation import enum_validator, validate_name
from bench.utils.casing import IdentifierType
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import Block, Icon, Package, Policy, Text


class ViewType(IdEnum):
    # 'intrinsic' roots for editor
    PAGE = 1
    BLOCK = 2
    EXPLORER = 10
    HISTORY = 11
    WATCH = 12
    TEST = 13
    RESOURCE = 14
    INSPECTOR = 15
    LIBRARY = 16
    ACCESS = 17

    # containers
    WINDOW_GROUP = 50
    TAB_GROUP = 52
    STEP_GROUP = 55
    SPLIT = 57
    STACK = 60
    DISCLOSURE = 61
    GRID = 62
    GRID_ROW = 63
    # data
    LIST = 70
    TABLE = 71
    FEED = 72
    # group
    GROUP = 75
    FORM = 76
    MENU = 77

    # presentation
    SPACER = 80
    DIVIDER = 81
    PROGRESS = 82
    SHAPE = 83
    AVATAR = 84
    # media
    ICON = 100
    IMAGE = 101
    VIDEO = 102
    AUDIO = 103
    DOCUMENT = 104

    # controls
    BUTTON = 120
    LINK = 121

    # inputs
    VALUE = 160  # (generic value based on type)
    # numeric-ish
    TOGGLE = 170
    CHECKBOX = 171
    CHECKBOX_GROUP = 172
    SLIDER = 173
    # stringy
    TEXT = 180
    CODE = 181
    JSON = 182
    # selection
    PICKER = 190
    DATE_PICKER = 191
    COLOR_PICKER = 192
    FILE_PICKER = 193
    ...


class ColorType(IdEnum):
    """Built-in color types a la SwiftUI or Tailwind."""

    # purpose
    PRIMARY = 1
    SECONDARY = 2
    ACCENT = 3
    # status
    SUCCESS = 10
    HINT = 11
    INFO = 12
    WARNING = 13
    ERROR = 14
    # actual
    BLACK = 30
    BLUE = 31
    BROWN = 32
    CLEAR = 33
    CYAN = 34
    GRAY = 35
    GREEN = 36
    INDIGO = 37
    MINT = 38
    ORANGE = 39
    PINK = 40
    PURPLE = 41
    RED = 42
    TEAL = 43
    WHITE = 44
    YELLOW = 45


@struct(StructType.COLOR)
class Color(Struct):
    """A color value."""

    type: ColorType = p_regular(31, require=True, validate=enum_validator(ColorType))
    hex: Optional[str] = p_regular(33, default=None)


class SpacingType(IdEnum):
    """Built-in spacings like in Tailwind."""

    # purpose
    ...
    # actual
    ...


@node_component
class HasViews(Node):
    views: list["View"] = p_node_child(NodeType.VIEW)


@node(NodeType.VIEW, identifier=IdentifierType.VARIABLE)
class View(HasViews):
    """A view of a user interface in a Bench."""

    parent: Union["Space", "View", "Block"] = p_node_parent(
        4, NodeType.SPACE, NodeType.VIEW, NodeType.BLOCK
    )
    type: ViewType = p_regular(30, require=True, validate=enum_validator(ViewType))

    # common
    name: Optional[str] = p_regular(31, default=None, validate=validate_name)
    title: Optional[str] = p_regular(32, default=None, validate=validate_name)
    text: Optional["Text"] = p_regular(33, default=None, struct=StructType.TEXT)
    icon: Optional["Icon"] = p_regular(
        35, default=None, require=False, array=False, struct=StructType.ICON
    )

    # content
    ...  # value/value source

    # appearance
    ...  # margin/padding/colors/...

    # layout
    ...

    # interaction
    ...

    # flags
    is_visible: bool = p_regular(80, default=True)
    is_disabled: bool = p_regular(81, default=False)
    is_loading: bool = p_regular(82, default=False)


class PageViewMode(IdEnum):
    NOTEBOOK = 1
    SCRIPT = 2


@node(NodeType.SPACE, identifier=IdentifierType.VARIABLE)
class Space(HasViews):
    """A space for a user to interact with the Bench."""

    parent: "Package" = p_node_parent(4, NodeType.PACKAGE)

    name: str = p_regular(31)
    text: Optional["Text"] = p_regular(32, default=None, struct=StructType.TEXT)
    order_key: str = p_internal(33)
    policies: list["Policy"] | None = p_regular(34, struct=StructType.POLICY, array=True)
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
        30, require=True, array=True, struct=StructType.SPACE_DOCK_ITEM
    )
