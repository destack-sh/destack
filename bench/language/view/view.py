from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    IsClaimable,
    IsModal,
    IsNamed,
    IsOwnable,
    IsTemplatable,
    LocalNodeList,
    Node,
    NodeReference,
    NodeType,
    PageNode,
    PrimitiveType,
    Selection,
    Struct,
    StructType,
    enum_,
    node_,
    p_node_children,
    p_node_parent,
    p_regular,
    struct_,
)
from bench.pb2 import ViewData

if TYPE_CHECKING:
    from bench.language import Page, Space, Type

# pyright: reportIncompatibleVariableOverride=false

# TODO :Architecture! :PolyViews: make Views "polymorphic" with their own Nodes & traits
#  (so each 'type' of View has its own tables & proto types.. also share some traits with
#   other 'views' like maybe Block/Page/Flow/Action have some view-like traits too)


@enum_(EnumType.VIEW_TYPE)
class ViewType(BuiltinEnum):
    #
    # Intrinsics (0-30000)
    #

    # nodes (0-10000)
    COMPUTER = 1250
    PAGE = 1020
    BLOCK = 1030
    CHOICE = 1400
    CLASS = 1410
    FIELD = 1420
    FLOW = 2220
    ACTION = 2210
    TRANSITION = 2230
    SERVICE = 2200
    TABLE = 1600
    THREAD = 1810
    AGENT = 2800
    RUN = 2410
    TASK = 2000

    # objects (10000-20000)
    OBJECT = 10000
    TYPE = 10001
    SELECTION = 10006

    # helpers (20000-30000)
    WIZARD = 20001, "Wizard", "User/Bench wizard", "fas fa-wand-sparkles"
    EMPTY = 20100, "Empty view", "For debugging", "fas fa-bug"
    SIDEBAR = 20205, None, None, "fas fa-object-group"
    CONTEXT = 20206, None, None, "fas fa-question"
    # ACTIVITY = 20207, None, None, "fas fa-list-timeline"
    # CATALOG = 20208, None, None, "fas fa-th-large"
    # INBOX = 20209, None, None, "fas fa-inbox"

    #
    # Organization (30000-31000)
    #

    # layout (30000-30100)
    WINDOW = 30001, "Window", "Full window", "fas fa-window"
    TAB = 30002, "Tab", "Tabbed interface", "fas fa-sidebar"
    HISTORY = 30003, "History", "History of views", "fas fa-clock-rotate-left"
    SPLIT = 30004, "Split", "Split view", "fas fa-split"
    # STACK = 30006, "Stack", "Stacked views", "fas fa-layer-group"
    # DRAWER = 30007, "Drawer", "Drawer view", "fas fa-square-minus"
    SCROLL = 30008, "Scroll", "Scrollable view", "fas fa-arrows-alt-v"
    # SPLIT_DRAWER, GRID

    # groups (30100-30200)
    # SECTION = 30100, "Section", "Sectioned view", "fas fa-xmark-lines"
    # GROUP, FORM, ...

    # presentation (30200-30300)
    # DIVIDER = 30200, "Divider", "Divider view", "fas fa-horizontal-rule"

    # collections (30300-30400)
    # LIST = 30300, "List", "List view", "fas fa-list"
    # TABLE = 30301, "Table", "Table view", "fas fa-table"
    # TREE = 30302, "Tree", "Tree view", "fas fa-list-tree"
    # FEED = 30303, "Feed", "Feed view", "fas fa-list-timeline"
    # GALLERY = 30304, "Gallery", "Gallery view", "fas fa-th-large"
    # BOARD = 30305, "Board", "Board view", "fas fa-columns"
    # ROW, COLUMN, ...?
    # CALENDAR, MAP, ...?

    #
    # Style (31000-32000)
    #

    # navigation (31000-31100)
    # BREADCRUMB = 31001, "Breadcrumb", "Breadcrumb view", "fas fa-ellipsis-h"
    # PROGRESS = 31002, "Progress", "Progress view", "fas fa-spinner"
    # AVATAR = 31003, "Avatar", "Avatar view", "fas fa-user-circle"
    # BADGE = 31004, "Badge", "Badge view", "fas fa-badge"
    # illustration (31100-31200)
    # SHAPE = 31100, "Shape", "Shape view", "fas fa-shapes"

    # graphing (31200-31300)
    # CHART = 31200, "Chart", "Chart view", "fas fa-chart-pie"

    #
    # Action (32000-33000)
    #

    # controls (32000-32100)
    BUTTON = 32001, "Button", "Button view", "fas fa-hand-pointer"
    # MULTI_BUTTON = 32002, "Multi button", "Multi button view", "fas fa-hand-pointer"

    #
    # Content (35000-)
    #

    # numeric (35000-35100)
    NUMBER = 35001, "Number", "Number view", "fas fa-hashtag"
    SLIDER = 35002, "Slider", "Slider view", "fas fa-slider"

    # stringy (35100-35200)
    STRING = 35101, "String", "String view", "fas fa-font-case"
    TEXT = 35102, "Text", "Text view", "fas fa-text"
    TEXT_LINE = 35103, "Text line", "Text line view", "fas fa-text"
    CODE = 35110, "Code", "Code view", "fas fa-code"
    # JSON = 35104, "JSON", "JSON view", "fas fa-brackets-curly"

    # selection (35200-35300)
    TOGGLE = 35201, "Toggle", "Toggle view", "fas fa-square-check"
    PICKER = 35202, "Picker", "Picker view", "fas fa-caret-circle-down"
    COLOR = 35203, "Color", "Color view", "fas fa-palette"
    ICON = 35204, "Icon", "Icon view", "fas fa-icons"
    DATETIME = 35205, "Datetime", "Datetime view", "fas fa-calendar-days"
    DURATION = 35206, "Duration", "Duration view", "fas fa-stopwatch"

    # file (35300-35400)
    FILE = 35301, "File", "File view", "fas fa-file"
    IMAGE = 35302, "Image", "Image view", "fas fa-image"
    AUDIO = 35303, "Audio", "Audio view", "fas fa-volume"
    VIDEO = 35304, "Video", "Video view", "fas fa-video"
    DOCUMENT = 35305, "Document", "Document view", "fas fa-file-alt"

    # expression
    # ...


# copy NodeType properties to ViewType
for view_type in ViewType:
    if view_type.id < 10000:
        node_type = NodeType(view_type.id)
        view_type.title = node_type.title
        view_type.color = node_type.color
        view_type.icon = node_type.icon
        view_type.text = node_type.text


@enum_(EnumType.FONT_TYPE)
class FontType(BuiltinEnum):
    SERIF = 1
    SANS = 2
    MONO = 3


@enum_(EnumType.FONT_WEIGHT)
class FontWeight(BuiltinEnum):
    THIN = 100
    EXTRA_LIGHT = 200
    LIGHT = 300
    NORMAL = 400
    MEDIUM = 500
    SEMI_BOLD = 600
    BOLD = 700
    EXTRA_BOLD = 800
    BLACK = 900


@enum_(EnumType.FONT_SIZE)
class FontSize(BuiltinEnum):
    XS = 12
    SM = 14
    BASE = 16
    LG = 18
    XL = 20
    XL2 = 24
    XL3 = 30
    XL4 = 36
    XL5 = 48
    XL6 = 60
    XL7 = 72


@struct_(StructType.FONT)
class Font(Struct):
    """A font value."""

    type: Optional[FontType] = p_regular(31, default=None)
    weight: Optional[FontWeight] = p_regular(32, default=None)
    size: Optional[FontSize] = p_regular(33, default=None)


@enum_(EnumType.SPACING)
class Spacing(BuiltinEnum):
    """
    The spacing scale for positions, padding, margin, etc. We don't enforce this.
    """

    S1 = 1
    S2 = 2
    S3 = 3
    S4 = 4
    S6 = 6
    S8 = 8
    S10 = 10
    S12 = 12
    S14 = 14
    S16 = 16
    S20 = 20
    S24 = 24
    S28 = 28
    S32 = 32
    S36 = 36
    S40 = 40
    S44 = 44
    S48 = 48
    S52 = 52
    S56 = 56
    S60 = 60
    S64 = 64
    S72 = 72
    S80 = 80
    S96 = 96
    S128 = 128
    S160 = 160
    S192 = 192
    S224 = 224
    S256 = 256


@enum_(EnumType.ANCHOR)
class Anchor(BuiltinEnum):
    """An anchor in 2D space."""

    TOP = 1
    TOP_LEFT = 2
    TOP_RIGHT = 3

    RIGHT = 11
    RIGHT_TOP = 12
    RIGHT_BOTTOM = 13

    BOTTOM = 21
    BOTTOM_LEFT = 22
    BOTTOM_RIGHT = 23

    LEFT = 31
    LEFT_TOP = 32
    LEFT_BOTTOM = 33


@struct_(StructType.TRANSFORM)
class Transform(Struct):
    """A transform in 2D space."""

    # translation
    translate_x: Optional[float] = p_regular(30, default=None)
    translate_y: Optional[float] = p_regular(31, default=None)
    # scale
    scale_x: Optional[float] = p_regular(33, default=None)
    scale_y: Optional[float] = p_regular(34, default=None)
    # skew
    skew_x: Optional[float] = p_regular(36, default=None)
    skew_y: Optional[float] = p_regular(37, default=None)
    # rotation
    rotate_x: Optional[float] = p_regular(40, default=None)


@struct_(StructType.RECTANGLE)
class Rectangle(Struct):
    """A rectangle. Absolute units are contextual (often in our Spacing scale)."""

    width: Optional[int] = p_regular(50, default=None)
    height: Optional[int] = p_regular(51, default=None)
    width_relative: Optional[float] = p_regular(52, default=None)
    height_relative: Optional[float] = p_regular(53, default=None)


@struct_(StructType.OFFSET)
class Offset(Struct):
    """A position value. Absolute units are contextual (often in our Spacing scale)."""

    top: Optional[int] = p_regular(40, default=None)
    right: Optional[int] = p_regular(41, default=None)
    bottom: Optional[int] = p_regular(42, default=None)
    left: Optional[int] = p_regular(43, default=None)

    top_relative: Optional[float] = p_regular(44, default=None)
    right_relative: Optional[float] = p_regular(45, default=None)
    bottom_relative: Optional[float] = p_regular(46, default=None)
    left_relative: Optional[float] = p_regular(47, default=None)


@struct_(StructType.VECTOR2)
class Vector2(Struct):
    """A 2D vector."""

    x: float = p_regular(30)
    y: float = p_regular(31)


def vector2(x: float, y: float) -> "Vector2":
    return Vector2(x=float(x), y=float(y))


@struct_(StructType.VECTOR3)
class Vector3(Struct):
    """A 3D vector."""

    x: float = p_regular(30)
    y: float = p_regular(31)
    z: float = p_regular(32)


def vector3(x: float, y: float, z: float) -> "Vector3":
    return Vector3(x=float(x), y=float(y), z=float(z))


@struct_(StructType.VECTOR4)
class Vector4(Struct):
    """A 4D vector."""

    x: float = p_regular(30)
    y: float = p_regular(31)
    z: float = p_regular(32)
    w: float = p_regular(33)


def vector4(x: float, y: float, z: float, w: float) -> "Vector4":
    return Vector4(x=float(x), y=float(y), z=float(z), w=float(w))


@struct_(StructType.LINE)
class Line(Struct):
    """A line segment."""

    points: list[Vector2] = p_regular(30, array=True, struct=StructType.VECTOR2)


@enum_(EnumType.ORIENTATION)
class Orientation(BuiltinEnum):
    """Which way to orient the contents/subviews of a view."""

    HORIZONTAL = 1
    HORIZONTAL_REVERSED = 2
    VERTICAL = 11
    VERTICAL_REVERSED = 12


@enum_(EnumType.ALIGNMENT)
class Alignment(BuiltinEnum):
    """How to align the contents/subviews of a view along its orientation."""

    START = 1
    MIDDLE = 2
    END = 3
    SPACE_BETWEEN = 4


@struct_(StructType.RECTANGLE_CONSTRAINT)
class RectangleConstraint(Struct):
    """Constraints for a Rectangle."""

    min_width: Optional[int] = p_regular(50, default=None)
    max_width: Optional[int] = p_regular(51, default=None)
    min_height: Optional[int] = p_regular(52, default=None)
    max_height: Optional[int] = p_regular(53, default=None)


#
# Views
# NOTE :Architecture: it would be cool to have 'content slots' within Views and custom Views
#  (so you can render custom web stuff and inside of that still have our own Views,
#   for a more seamless integration of our components (like in the ProseMirror schema))
#

# nocheckin


@node_(NodeType.VIEW)
class View(
    IsTemplatable,
    IsModal,
    IsNamed,
    IsOwnable,
    IsClaimable,
    PageNode[ViewData],
):
    """A View is a graphical interface in a Bench."""

    parent: Union["Space", "View", "Page", None] = p_node_parent(
        4, NodeType.SPACE, NodeType.VIEW, NodeType.PAGE
    )

    # meta
    type: ViewType = p_regular(30, require=True, primitive_type=PrimitiveType.INT32)
    title: Optional[str] = p_regular(32, require=False)

    # content
    value_type: Optional["Type"] = p_regular(
        41, default=None, require=False, struct=StructType.TYPE
    )
    node: Optional["Node"] = p_regular(
        42, default=None, require=False, array=False, references="any"
    )
    if TYPE_CHECKING:
        node_type: Optional[NodeType] = None
        node_id: Optional[UUID] = None
        node_ptr: Optional["NodeReference"] = None

    # style
    ...  # font/variant/border/corner/foreground/background/...

    # layout
    position: Optional[Offset] = p_regular(
        60, default=None, require=False, array=False, struct=StructType.OFFSET
    )
    size: Optional[Rectangle] = p_regular(
        61, default=None, require=False, array=False, struct=StructType.RECTANGLE
    )
    # margin: Optional[Offset] = p_regular(
    #     62, default=None, require=False, array=False, struct=StructType.OFFSET
    # )
    # padding: Optional[Offset] = p_regular(
    #     63, default=None, require=False, array=False, struct=StructType.OFFSET
    # )
    orientation: Optional[Orientation] = p_regular(64, default=None, require=False)
    alignment: Optional[Alignment] = p_regular(65, default=None, require=False)
    transform: Optional[Transform] = p_regular(
        66, default=None, require=False, array=False, struct=StructType.TRANSFORM
    )
    constraint: Optional[RectangleConstraint] = p_regular(
        67, default=None, require=False, array=False, struct=StructType.RECTANGLE_CONSTRAINT
    )
    ...  # scroll/...

    # behavior
    focus: Optional[Node] = p_regular(
        70, default=None, require=False, array=False, references="any"
    )
    selection: Optional[Selection] = p_regular(
        71, default=None, require=False, struct=StructType.SELECTION
    )
    ...  # actions/effects/...

    # flags
    is_disabled: bool = p_regular(81, default=False)
    is_input: bool = p_regular(82, default=False)
    is_inline: bool = p_regular(83, default=False)
    is_minimal: bool = p_regular(84, default=False)

    views: LocalNodeList["View"] = p_node_children(NodeType.VIEW)

    @staticmethod
    def new(typ: ViewType, name: str, **kwargs) -> "View":
        return View(type=typ, name=name, **kwargs)


#
# Intrinsics (0-30000)
#


@enum_(EnumType.USER_WIZARD_STAGE)
class UserWizardViewStage(BuiltinEnum):
    """The stage of a User view."""

    SIGN_UP = 1
    LOG_IN = 2


@enum_(EnumType.CONTEXT_MODE)
class ContextMode(BuiltinEnum):
    """The mode of a Context view."""

    DETAIL = 1
    CHAT = 2
    # LOG, ...


@enum_(EnumType.BUTTON_VARIANT)
class ButtonVariant(BuiltinEnum):
    PRIMARY = 1
    SECONDARY = 2
    LINK = 3


@enum_(EnumType.PICKER_VARIANT)
class PickerVariant(BuiltinEnum):
    MULTI_TOGGLE = 1
    DROPDOWN = 2
    DROPDOWN_LARGE = 3
