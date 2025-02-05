from typing import TYPE_CHECKING, Any, Optional, Union

from bench.language.core import (
    TITLE_CONSTRAINT,
    BuiltinEnum,
    EnumType,
    InlineSourceNode,
    LocalNodeList,
    Node,
    NodeReference,
    NodeType,
    PrimitiveType,
    Selection,
    Struct,
    StructType,
    enum_,
    node_,
    p_node_children,
    p_node_parent,
    p_regular,
    p_value_packed,
    struct_,
    subnode_,
)
from bench.pb2 import ViewData

if TYPE_CHECKING:
    from bench.language import Expression, Message, Page, Space, Text, Type

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.VIEW_TYPE)
class ViewType(BuiltinEnum):
    #
    # Intrinsics (0-30000)
    #

    # nodes (0-10000)
    MACHINE = 2100
    BROWSER = 2150
    BLOCK = 3010
    FIELD = 3012
    VIEW = 3020
    PIPE = 3031
    ACTION = 3030
    RUN = 3601

    # subnodes (10000-20000)
    PAGE = 10010
    DATABASE = 10011
    FLOW = 10012

    # objects (20000-30000)
    OBJECT = 20000
    TYPE = 20001
    FIELD_LIST = 20003
    PATH = 20004
    COMPUTED_VALUE = 20005

    # helpers (30000-40000)
    USER_WIZARD = 30001
    BENCH_WIZARD = 30002
    EMPTY = 30100
    CREATE = 30201
    CHAT = 30202
    HUB = 30205
    HELP = 30206
    ACTIVITY = 30207
    CATALOG = 30208

    #
    # Organization (40000-41000)
    #

    # layout (40000-40100)
    WINDOW = 40001
    TAB = 40002
    HISTORY = 40003
    SPLIT = 40004
    SPLIT_DRAWER = 40005
    STACK = 40006
    DRAWER = 40007
    SCROLL = 40008
    GRID = 40009

    # groups (40100-40200)
    GROUP = 40100
    SECTION = 40101
    FORM = 40102

    # presentation (40200-40300)
    SPACER = 40200
    DIVIDER = 40201

    # collections (40300-40400)
    LIST = 40300
    TABLE = 40301
    TREE = 40302
    FEED = 40303
    GALLERY = 40304
    BOARD = 40305
    # ROW, COLUMN, ...?
    # CALENDAR, MAP, ...?

    #
    # Style (41000-42000)
    #

    # navigation (41000-41100)
    BREADCRUMB = 41001
    PROGRESS = 41002
    AVATAR = 41003
    BADGE = 41004
    # illustration (41100-41200)
    SHAPE = 41100

    # graphing (41200-41300)
    CHART = 41200

    #
    # Action (42000-43000)
    #

    # controls (42000-42100)
    BUTTON = 42001
    MULTI_BUTTON = 42002
    LINK = 42003

    #
    # Content (45000-)
    #

    # numeric (45000-45100)
    NUMBER = 45001
    SLIDER = 45002

    # stringy (45100-45200)
    STRING = 45101
    TEXT = 45102
    CODE = 45103
    JSON = 45104

    # selection (45200-45300)
    TOGGLE = 45201
    PICKER = 45202
    COLOR = 45203
    ICON = 45204
    DATETIME = 45205
    DURATION = 45206

    # file (45300-45400)
    FILE = 45301  # (generic file content according to file type)
    IMAGE = 45302
    AUDIO = 45303
    VIDEO = 45304
    DOCUMENT = 45305

    # expression
    # ...


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


@struct_(StructType.VECTOR3)
class Vector3(Struct):
    """A 3D vector."""

    x: float = p_regular(30)
    y: float = p_regular(31)
    z: float = p_regular(32)


@struct_(StructType.VECTOR4)
class Vector4(Struct):
    """A 4D vector."""

    x: float = p_regular(30)
    y: float = p_regular(31)
    z: float = p_regular(32)
    w: float = p_regular(33)


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
#


@node_(NodeType.VIEW, has_subtypes=True)
class View(InlineSourceNode[ViewData]):
    """A View is a graphical interface in a Bench."""

    parent: Union["Space", "View", "Page", None] = p_node_parent(
        4, NodeType.SPACE, NodeType.VIEW, NodeType.PAGE
    )

    # meta
    type: ViewType = p_regular(30, require=True, primitive_type=PrimitiveType.INT32)
    subviews_packed: dict[str, Any] | None = p_value_packed(39)

    # content
    title: Optional[str] = p_regular(40, default=None, constraint=TITLE_CONSTRAINT)
    value_type: Optional["Type"] = p_regular(
        41, default=None, require=False, struct=StructType.TYPE
    )
    node: Optional["Node"] = p_regular(
        42, default=None, require=False, array=False, references="any"
    )
    if TYPE_CHECKING:
        node_type: Optional[NodeType] = None
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
    margin: Optional[Offset] = p_regular(
        62, default=None, require=False, array=False, struct=StructType.OFFSET
    )
    padding: Optional[Offset] = p_regular(
        63, default=None, require=False, array=False, struct=StructType.OFFSET
    )
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
    selection: Optional[Selection] = p_regular(
        70, default=None, require=False, struct=StructType.SELECTION
    )
    focus: Optional[Selection] = p_regular(
        71, default=None, require=False, struct=StructType.SELECTION
    )
    ...  # actions/effects/...

    # flags
    is_hidden: bool = p_regular(80, default=False)
    is_disabled: bool = p_regular(81, default=False)
    is_input: bool = p_regular(82, default=False)
    is_inline: bool = p_regular(83, default=False)
    is_minimal: bool = p_regular(84, default=False)
    is_loading: bool = p_regular(85, default=False)

    views: LocalNodeList["View"] = p_node_children(NodeType.VIEW)

    @staticmethod
    def new(typ: ViewType, name: str, **kwargs) -> "View":
        return View(type=typ, name=name, **kwargs)


#
# Intrinsics (0-30000)
#

# nodes (0-10000)


@subnode_(ViewType.RUN)
class RunView(View):
    variables_packed: Any = p_value_packed(100)
    inputs_packed: Any = p_value_packed(101)


# subnodes (10000-20000)

# objects (20000-30000)


@subnode_(ViewType.OBJECT)
class ObjectView(View):
    expanded_sections: list[str] = p_regular(100, array=True)
    collapsed_sections: list[str] = p_regular(101, array=True)


# helpers (30000-40000)


@enum_(EnumType.USER_WIZARD_STAGE)
class UserWizardViewStage(BuiltinEnum):
    """The stage of a User view."""

    SIGN_UP = 1
    LOG_IN = 2


@subnode_(ViewType.USER_WIZARD)
class UserWizardView(View):
    stage: UserWizardViewStage | None = p_regular(100)


@subnode_(ViewType.CHAT)
class ChatView(View):
    text: Optional["Text"] = p_regular(100, require=False, struct=StructType.TEXT)
    nodes: list["Node"] = p_regular(101, require=False, array=True, references="any")
    reply_to: Optional["Message"] = p_regular(
        102, require=False, array=False, references=NodeType.MESSAGE
    )


@enum_(EnumType.HUB_ASPECT)
class HubAspect(BuiltinEnum):
    BENCH = 1
    ACTIVITY = 2
    CATALOG = 3
    LIBRARY = 4


@subnode_(ViewType.HUB)
class HubView(View):
    aspect: HubAspect | None = p_regular(100, default=None)


@enum_(EnumType.HELP_ASPECT)
class HelpAspect(BuiltinEnum):
    DETAIL = 1
    RUN = 2
    CHAT = 3
    VERSION = 4


@subnode_(ViewType.HELP)
class HelpView(View):
    aspect: HelpAspect | None = p_regular(100, default=None)


#
# Organization (40000-41000)
#

# collections (40300-40400)


@subnode_(ViewType.LIST)
class ListView(View):
    query_node_type: NodeType | None = p_regular(100, require=False)
    filter: "Expression | None" = p_regular(
        101, default=None, require=False, struct=StructType.EXPRESSION
    )
    sort: list["Expression"] = p_regular(
        102, require=False, struct=StructType.EXPRESSION, array=True
    )


@enum_(EnumType.TREE_VIEW_PRESET)
class TreeViewPreset(BuiltinEnum):
    EXPLORE = 1
    OUTLINE = 2


@subnode_(ViewType.TREE)
class TreeView(View):
    node_types: list[NodeType] = p_regular(100, array=True)
    filter_is_page: Optional[bool] = p_regular(101, default=None, require=False)
    is_default_expanded: Optional[bool] = p_regular(102, default=None, require=False)
    expanded_nodes: list[Node] = p_regular(103, require=False, array=True, references="any")
    collapsed_nodes: list[Node] = p_regular(104, require=False, array=True, references="any")

    preset: Optional[TreeViewPreset] = p_regular(110, default=None, require=False)


@subnode_(ViewType.FEED)
class FeedView(View):
    query_node_type: NodeType | None = p_regular(100, require=False)
    filter: "Expression | None" = p_regular(
        101, default=None, require=False, struct=StructType.EXPRESSION
    )


#
# Style (41000-42000)
#

# navigation (41000-41100)
# illustration (41100-41200)
# graphing (41200-41300)


#
# Action (42000-43000)
#

# controls (42000-42100)


@enum_(EnumType.BUTTON_VARIANT)
class ButtonVariant(BuiltinEnum):
    PRIMARY = 1
    SECONDARY = 2
    LINK = 3


@subnode_(ViewType.BUTTON)
class ButtonView(View):
    variant: ButtonVariant | None = p_regular(100)


#
# Content (45000-)
#

# numeric (45000-45100)

# stringy (45100-45200)

# selection (45200-45300)


@enum_(EnumType.PICKER_VARIANT)
class PickerVariant(BuiltinEnum):
    MULTI_TOGGLE = 1
    DROPDOWN = 2
    DROPDOWN_LARGE = 3


@subnode_(ViewType.PICKER)
class PickerView(View):
    variant: PickerVariant | None = p_regular(100)


@subnode_(ViewType.DATETIME)
class DatetimeView(View):
    is_relative: bool | None = p_regular(100, default=False)


@subnode_(ViewType.ICON)
class IconView(View):
    include_color: bool | None = p_regular(100, default=None)


# file (45300-45400)
