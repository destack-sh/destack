from typing import TYPE_CHECKING, Any, Optional, Union, final

from bench.language.const import NODE_TYPES, EnumType, NodeType, StructType, enum_
from bench.language.expression import Selection
from bench.language.graph import NodeList
from bench.language.node import (
    Node,
    SomeNodeReference,
    SourceNode,
    Struct,
    local_node_,
    object_component,
    struct_,
)
from bench.language.property import (
    p_internal,
    p_node_children,
    p_node_parent,
    p_regular,
    p_value_packed,
    p_value_runtime,
)
from bench.language.validation import NAME_CONSTRAINT, TITLE_CONSTRAINT
from bench.language.value import HasValues
from bench.proto.wire import SpaceData, ViewData
from bench.utils.fractional import INTEGER_ZERO
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import Block, Expression, File, Icon, Package, Policy, Run, Text, TypeInfo

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.VIEW_TYPE)
class ViewType(IdEnum):
    #
    # Intrinsics
    #

    # 'kernel'
    # auth
    USER_WIZARD = 1
    BENCH_WIZARD = 2
    # internal
    EMPTY = 80

    # 'system'
    # nodes
    PAGE = 201
    BLOCK = 202
    FIELD = 203
    DATABASE = 204
    VIEW = 205
    FLOW = 206
    STEP = 207
    TYPE = 208
    VARIABLE = 209
    OBJECT = 210
    RUN = 211
    LOG = 212
    # helpers
    TREE = 400
    INSPECT = 403
    CREATE = 404
    CHAT = 405
    START = 406
    FEED = 407
    TIMELINE = 408
    HISTORY = 409

    #
    # General
    #

    # containers (root)
    WINDOW = 1000
    TAB = 1002
    SPLIT = 1003
    SPLIT_DRAWER = 1004
    # containers (layout)
    STACK = 1010
    DRAWER = 1011
    SCROLL = 1012
    GRID = 1013
    # ROW, COLUMN, ...?
    # containers (data)
    LIST = 1020
    TABLE = 1021
    # containers (group)
    GROUP = 1030
    SECTION = 1031

    # presentation
    SPACER = 1050
    DIVIDER = 1051
    SHAPE = 1053
    PROGRESS = 1054
    AVATAR = 1055
    BADGE = 1056
    CHART = 1057

    # controls
    BUTTON = 1100
    MULTI_BUTTON = 1101
    LINK = 1110

    # content
    VALUE = 1220  # (generic content routed according to value type)
    # numeric
    NUMBER = 1232
    SLIDER = 1233
    # stringy
    STRING = 1240
    TEXT = 1241
    CODE = 1242
    JSON = 1243
    # selection
    TOGGLE = 1250
    PICKER = 1253
    CALENDAR = 1256
    MAP = 1258
    COLOR = 1260
    ICON = 1261
    # file
    FILE = 1270  # (generic file content according to file type)
    IMAGE = 1273
    AUDIO = 1274
    VIDEO = 1275
    DOCUMENT = 1276
    ...


@enum_(EnumType.VARIANT)
class Variant(IdEnum):
    """The style variant of a view."""

    PRIMARY = 1
    SECONDARY = 2
    COMPACT = 3
    STEALTH = 4
    # ... may have more later


@enum_(EnumType.COLOR_TYPE)
class ColorType(IdEnum):
    """Built-in color types a la SwiftUI or Tailwind."""

    # surface
    PRIMARY = 1
    SECONDARY = 2
    ACCENT = 3
    CANVAS = 4
    # semantic
    SUCCESS = 10
    HINT = 11
    WARNING = 12
    DANGER = 13
    # actual
    GRAY = 30
    RED = 31
    ORANGE = 32
    AMBER = 33
    YELLOW = 34
    LIME = 35
    GREEN = 36
    EMERALD = 37
    TEAL = 38
    CYAN = 39
    SKY = 40
    BLUE = 41
    INDIGO = 42
    VIOLET = 43
    PURPLE = 44
    FUCHSIA = 45
    PINK = 46
    ROSE = 47


@enum_(EnumType.COLOR_SHADE)
class ColorShade(IdEnum):
    """Built-in color shades a la Tailwind."""

    # surface
    ...
    # actual
    S50 = 50
    S100 = 100
    S200 = 200
    S300 = 300
    S400 = 400
    S500 = 500
    S600 = 600
    S700 = 700
    S800 = 800
    S900 = 900
    S950 = 950


@struct_(StructType.COLOR)
class Color(Struct):
    """A color value."""

    type: Optional[ColorType] = p_regular(31, default=None)
    shade: Optional[ColorShade] = p_regular(32, default=None)
    hex: Optional[str] = p_regular(33, default=None)

    @staticmethod
    def new(color: "ColorIn") -> "Color":
        return to_color(color)


ColorIn = Color | ColorType | str


def to_color(color: ColorIn) -> Color:
    if isinstance(color, Color):
        return color
    elif isinstance(color, ColorType):
        return Color(type=color)
    else:
        return Color(hex=color)


@enum_(EnumType.FONT_TYPE)
class FontType(IdEnum):
    SERIF = 1
    SANS = 2
    MONO = 3


@enum_(EnumType.FONT_WEIGHT)
class FontWeight(IdEnum):
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
class FontSize(IdEnum):
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
class Spacing(IdEnum):
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
class Anchor(IdEnum):
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
    translate_x: Optional[int] = p_regular(30, default=None)
    translate_y: Optional[int] = p_regular(31, default=None)
    # scale
    scale_x: Optional[float] = p_regular(33, default=None)
    scale_y: Optional[float] = p_regular(34, default=None)
    # skew
    skew_x: Optional[float] = p_regular(36, default=None)
    skew_y: Optional[float] = p_regular(37, default=None)
    # rotation
    rotate_x: Optional[int] = p_regular(40, default=None)


@struct_(StructType.BOX)
class Box(Struct):
    """A box value. Absolute units are in pixels, ideally in Spacing scale."""

    width: Optional[int] = p_regular(50, default=None)
    height: Optional[int] = p_regular(51, default=None)
    width_relative: Optional[float] = p_regular(52, default=None)
    height_relative: Optional[float] = p_regular(53, default=None)


@struct_(StructType.OFFSET)
class Offset(Struct):
    """A position value. Absolute units are in pixels, ideally in Spacing scale."""

    top: Optional[int] = p_regular(40, default=None)
    right: Optional[int] = p_regular(41, default=None)
    bottom: Optional[int] = p_regular(42, default=None)
    left: Optional[int] = p_regular(43, default=None)

    top_relative: Optional[float] = p_regular(44, default=None)
    right_relative: Optional[float] = p_regular(45, default=None)
    bottom_relative: Optional[float] = p_regular(46, default=None)
    left_relative: Optional[float] = p_regular(47, default=None)


@enum_(EnumType.ORIENTATION)
class Orientation(IdEnum):
    """Which way to orient the contents/subviews of a view."""

    HORIZONTAL = 1
    # HORIZONTAL_REVERSED = 2
    VERTICAL = 11
    # VERTICAL_REVERSED = 12


@enum_(EnumType.ALIGNMENT)
class Alignment(IdEnum):
    """How to align the contents/subviews of a view along its orientation."""

    START = 1
    MIDDLE = 2
    END = 3
    SPACE_BETWEEN = 4


@local_node_(NodeType.VIEW, passthrough="value")
class View(SourceNode[ViewData], HasValues):
    """A view of a user interface in a Bench."""

    parent: Union["Space", "View", "Block", None] = p_node_parent(
        4, NodeType.SPACE, NodeType.VIEW, NodeType.BLOCK
    )

    # common
    type: ViewType = p_regular(30, require=True)
    name: str = p_regular(31, constraint=NAME_CONSTRAINT)
    title: Optional[str] = p_regular(32, default=None, constraint=TITLE_CONSTRAINT)
    text: Optional["Text"] = p_regular(33, default=None, struct=StructType.TEXT)
    order_key: str = p_internal(34, default=INTEGER_ZERO)
    icon: Optional["Icon"] = p_regular(
        35, default=None, require=False, array=False, struct=StructType.ICON
    )

    # content
    value_type: Optional["TypeInfo"] = p_regular(
        40, default=None, require=False, struct=StructType.TYPE_INFO
    )
    value_packed: Any = p_value_packed(41)
    value: Any = p_value_runtime(packed=41, typ=None)  # freely typed for now
    node: Optional["Node"] = p_regular(
        42, default=None, require=False, array=False, references=NODE_TYPES.tuple, rich=True
    )
    if TYPE_CHECKING:
        node_ptr: Optional["SomeNodeReference"] = None

    # style
    variant: Optional[Variant] = p_regular(50, default=None, require=False)
    font: Optional[Font] = p_regular(
        51, default=None, require=False, array=False, struct=StructType.FONT
    )
    ...  # border/corner/foreground/background/...

    # layout
    position: Optional[Offset] = p_regular(
        60, default=None, require=False, array=False, struct=StructType.OFFSET
    )
    size: Optional[Box] = p_regular(
        61, default=None, require=False, array=False, struct=StructType.BOX
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
    ...  # scroll/...

    # behavior
    selection: Optional[Selection] = p_regular(
        70, default=None, require=False, struct=StructType.SELECTION
    )
    focus: Optional[Selection] = p_regular(
        71, default=None, require=False, struct=StructType.SELECTION
    )
    expansion: Optional[Selection] = p_regular(
        72, default=None, require=False, struct=StructType.SELECTION
    )
    ...  # actions/effects/...

    # flags
    is_visible: Optional[bool] = p_regular(80, default=True)
    is_disabled: Optional[bool] = p_regular(81, default=False)
    is_input: Optional[bool] = p_regular(82, default=False)
    is_inline: Optional[bool] = p_regular(83, default=False)
    ...
    is_loading: Optional[bool] = p_regular(90, default=False)

    views: NodeList["View"] = p_node_children(NodeType.VIEW)

    @final
    def __repr__(self):  # type: ignore we want to override the default repr
        return f"<{self.type.bench_name}View {self}>"

    @staticmethod
    def new(typ: ViewType, name: str, **kwargs) -> "View":
        return View(type=typ, name=name, **kwargs)


@enum_(EnumType.SPACE_TYPE)
class SpaceType(IdEnum):
    DESKTOP = 10
    MOBILE = 20
    EXTENSION = 30


@local_node_(NodeType.SPACE)
class Space(SourceNode[SpaceData]):
    """A space for a user to interact with the Bench."""

    parent: Union["Package", "Block", None] = p_node_parent(4, NodeType.PACKAGE, NodeType.BLOCK)

    type: SpaceType = p_regular(30)
    name: str = p_regular(31, constraint=NAME_CONSTRAINT)
    text: Optional["Text"] = p_regular(32, default=None, struct=StructType.TEXT)
    order_key: str = p_internal(33, default=INTEGER_ZERO)
    policies: list["Policy"] | None = p_regular(34, struct=StructType.POLICY, array=True)

    bar_position: Optional[Anchor] = p_regular(40, default=Anchor.TOP)

    focus: Optional[Selection] = p_regular(
        70, default=None, require=False, struct=StructType.SELECTION
    )
    inspection: Optional[Node] = p_regular(
        75, default=None, require=False, array=False, references=tuple(NODE_TYPES)
    )
    base: Optional[Node] = p_regular(
        76, default=None, require=False, array=False, references=tuple(NODE_TYPES)
    )

    views: NodeList["View"] = p_node_children(NodeType.VIEW)


@enum_(EnumType.ICON_KIND)
class IconKind(IdEnum):
    EMOJI = 1
    FILE = 2
    FONT_AWESOME = 3


@struct_(StructType.ICON)
class Icon(Struct):
    """An icon to be displayed in some view."""

    kind: IconKind = p_internal(30, default=False)
    # content
    emoji: Optional[str] = p_internal(31, require=False)
    file: Optional["File"] = p_internal(
        32, require=False, array=False, references=NodeType.FILE, rich=True
    )
    fa_name: Optional[str] = p_internal(33, require=False)
    # style
    color: Optional["Color"] = p_internal(40, require=False, array=False, struct=StructType.COLOR)

    @staticmethod
    def new(icon: "IconIn") -> "Icon":
        return to_icon(icon)


IconIn = Icon | str


def to_icon(icon: IconIn) -> Icon:
    if isinstance(icon, str):
        if icon.startswith("fa-"):
            return Icon(kind=IconKind.FONT_AWESOME, fa_name=icon)
        else:
            return Icon(kind=IconKind.EMOJI, emoji=icon)
    else:
        return icon


#
# Custom view states
#


@object_component()
class ViewState(Struct):
    """Builtin special Value as the state of some specific view type (in View.value)."""

    pass


@struct_(StructType.PAGE_VIEW_STATE)
class PageViewState(ViewState):
    """The state of a Page view."""

    pass


@enum_(EnumType.TREE_VIEW_PRESET)
class TreeViewPreset(IdEnum):
    EXPLORE = 1
    OUTLINE = 2


@struct_(StructType.TREE_VIEW_STATE)
class TreeViewState(ViewState):
    """The state of a Tree view."""

    node_types: list[NodeType] = p_regular(35, array=True)
    filter_is_page: Optional[bool] = p_regular(36, default=None, require=False)
    is_default_expanded: Optional[bool] = p_regular(37, default=None, require=False)

    preset: Optional[TreeViewPreset] = p_regular(99, default=None, require=False)


@struct_(StructType.START_VIEW_STATE)
class StartViewState(ViewState):
    """The state of a Start view."""

    inputs_packed: Any = p_value_packed(30)
    last_run: "Run | None" = p_regular(40, default=None, require=False, references=NodeType.RUN)
    last_outputs_packed: Any = p_value_packed(41)
    feed: "FeedViewState | None" = p_regular(
        50, default=None, require=False, struct=StructType.FEED_VIEW_STATE
    )


@struct_(StructType.FEED_VIEW_STATE)
class FeedViewState(ViewState):
    """The state of a Feed view."""

    node_type: NodeType | None = p_regular(30, require=False)
    filter: "Expression | None" = p_regular(
        31, default=None, require=False, struct=StructType.EXPRESSION
    )
    filter_pills: list[str] = p_regular(99, array=True)


@struct_(StructType.CHART_VIEW_STATE)
class ChartViewState(ViewState):
    """The state of a Chart view."""

    pass  # ... vega stuff or something


@struct_(StructType.HISTORY_VIEW_STATE)
class HistoryViewState(ViewState):
    """The state of a History view."""

    pass


@struct_(StructType.TIMELINE_VIEW_STATE)
class TimelineViewState(ViewState):
    """The state of a Timeline view."""

    pass


@enum_(EnumType.USER_WIZARD_STAGE)
class UserWizardViewStage(IdEnum):
    """The stage of a User view."""

    SIGN_UP = 1
    LOG_IN = 2


@struct_(StructType.USER_WIZARD_VIEW_STATE)
class UserWizardViewState(ViewState):
    """The state of a User view."""

    stage: UserWizardViewStage | None = p_regular(30)
