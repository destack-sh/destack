from typing import TYPE_CHECKING, Any, Optional, Union

from bench.language.const import NODE_TYPES, NodeType, StructType
from bench.language.expression import Selection
from bench.language.node import LINK_TARGET_NODE_TYPES, Node, Struct, node, node_component, struct
from bench.language.property import (
    p_internal,
    p_node_child,
    p_node_parent,
    p_regular,
    p_value_packed,
    p_value_runtime,
)
from bench.language.setup import _well_known_enum
from bench.language.validation import enum_validator, validate_name
from bench.language.value import HasValues
from bench.utils.casing import IdentifierType
from bench.utils.fractional import INTEGER_ZERO
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import Block, Icon, Package, Policy, Text, TypeInfo


@_well_known_enum
class ViewType(IdEnum):
    #
    # Intrinsics
    #

    # 'kernel'
    # auth
    USER_WIZARD = 1
    BENCH_WIZARD = 2
    CHALLENGE_WIZARD = 3
    KEYMAP = 40
    MOCK = 70

    # 'system'
    # nodes
    PAGE = 101
    BLOCK = 102
    FIELD = 103
    DATABASE = 104
    SCREEN = 105
    FLOW = 106
    STEP = 107
    # helpers
    EXPLORE = 150
    OUTLINE = 151
    INSPECT = 153
    CREATE = 154

    #
    # General
    #

    # containers (root)
    WINDOW = 500
    TAB = 502
    SPLIT = 503
    SPLIT_COLLAPSIBLE = 504
    # containers (layout)
    WIZARD = 505
    STACK = 510
    COLLAPSIBLE = 511
    GRID = 512
    ROW = 513
    COLUMN = 514
    # containers (data)
    LIST = 520
    TABLE = 521
    FEED = 522
    # containers (group)
    GROUP = 535
    SECTION = 538

    # presentation
    SPACER = 540
    DIVIDER = 541
    SHAPE = 543
    PROGRESS = 544
    AVATAR = 545
    BADGE = 546
    CHART = 547

    # controls
    BUTTON = 600
    MULTI_BUTTON = 601
    LINK = 610

    # content
    VALUE = 620  # (generic content based on ... type?)
    # numeric
    NUMBER = 632
    SLIDER = 633
    # stringy
    PLAIN_TEXT = 640
    TEXT = 641
    CODE = 642
    JSON = 643
    # selection
    TOGGLE = 650
    PICKER = 653
    DATE = 656
    TIME = 658
    CALENDAR = 660
    COLOR = 663
    # file
    FILE = 670
    ICON = 672
    IMAGE = 673
    VIDEO = 674
    AUDIO = 675
    ...


@_well_known_enum
class Variant(IdEnum):
    """The style variant of a view."""

    PRIMARY = 1
    SECONDARY = 2
    ALTERNATE = 3
    STEALTH = 4
    WEIRD = 5


@_well_known_enum
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


@_well_known_enum
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


@struct(StructType.COLOR, inline=True)
class Color(Struct):
    """A color value."""

    type: Optional[ColorType] = p_regular(31, default=None, validate=enum_validator(ColorType))
    shade: Optional[ColorShade] = p_regular(32, default=None, validate=enum_validator(ColorShade))
    hex: Optional[str] = p_regular(33, default=None)


@_well_known_enum
class FontType(IdEnum):
    SERIF = 1
    SANS = 2
    MONO = 3


@_well_known_enum
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


@_well_known_enum
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


@struct(StructType.FONT, inline=True)
class Font(Struct):
    """A font value."""

    type: Optional[FontType] = p_regular(31, default=None, validate=enum_validator(FontType))
    weight: Optional[FontWeight] = p_regular(32, default=None, validate=enum_validator(FontWeight))
    size: Optional[FontSize] = p_regular(33, default=None, validate=enum_validator(FontSize))


@_well_known_enum
class Spacing(IdEnum):
    """
    The spacing scale for positions, padding, margin, etc. We don't enforce this.
    This is reminiscent of Tailwind's spacing scale.
    """

    S1 = 1
    S2 = 2
    S3 = 3
    S4 = 4
    S5 = 5
    S6 = 6
    S7 = 7
    S8 = 8
    S9 = 9
    S10 = 10
    S11 = 11
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


@_well_known_enum
class Anchor(IdEnum):
    """An anchor in 2D space."""

    TOP_LEFT = 1
    TOP_RIGHT = 3
    BOTTOM_RIGHT = 5
    BOTTOM_LEFT = 7


@struct(StructType.OFFSET, inline=True)
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


@struct(StructType.BOX, inline=True)
class Box(Struct):
    """A box value. Absolute units are in pixels, ideally in Spacing scale."""

    width: Optional[int] = p_regular(50, default=None)
    height: Optional[int] = p_regular(51, default=None)
    width_relative: Optional[float] = p_regular(52, default=None)
    height_relative: Optional[float] = p_regular(53, default=None)


@_well_known_enum
class Orientation(IdEnum):
    """Which way to orient the contents/subviews of a view."""

    HORIZONTAL = 1
    # HORIZONTAL_REVERSED = 2
    VERTICAL = 11
    # VERTICAL_REVERSED = 12


@_well_known_enum
class Alignment(IdEnum):
    """How to align the contents/subviews of a view along its orientation."""

    START = 1
    MIDDLE = 2
    END = 3
    SPACE_BETWEEN = 4


@node_component
class HasViews(Node):
    views: list["View"] = p_node_child(NodeType.VIEW)


@node(NodeType.VIEW, identifier=IdentifierType.VARIABLE)
class View(HasViews, HasValues):
    """A view of a user interface in a Bench."""

    parent: Union["Space", "View", "Block"] = p_node_parent(
        4, NodeType.SPACE, NodeType.VIEW, NodeType.BLOCK
    )
    type: ViewType = p_regular(30, require=True, validate=enum_validator(ViewType))

    # common
    name: str = p_regular(31, validate=validate_name)
    title: Optional[str] = p_regular(32, default=None, validate=validate_name)
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
    value = p_value_runtime(packed=41)
    # TODO :Cleanup :Architecture: View.node should probably just be in View.value
    #  (with relevant Views having that type... once we have the Value system more figured out)
    node: Optional["Node"] = p_regular(
        42, default=None, require=False, array=False, references=LINK_TARGET_NODE_TYPES
    )

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

    # interaction
    selection: Optional[Selection] = p_regular(
        70, default=None, require=False, struct=StructType.SELECTION
    )
    focus: Optional[Selection] = p_regular(
        71, default=None, require=False, struct=StructType.SELECTION
    )
    expansion: Optional[Selection] = p_regular(
        72, default=None, require=False, struct=StructType.SELECTION
    )
    ...  # behavior/actions/effects/...

    # flags
    is_visible: Optional[bool] = p_regular(80, default=True)
    is_disabled: Optional[bool] = p_regular(81, default=False)
    is_loading: Optional[bool] = p_regular(82, default=False)
    is_input: Optional[bool] = p_regular(83, default=False)

    def __repr__(self):  # noqa: we want to override the default repr
        return f"<{self.type.bench_name}View {self}>"


@_well_known_enum
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

    focus: Optional[Selection] = p_regular(
        70, default=None, require=False, struct=StructType.SELECTION
    )
    inspection: Optional[Node] = p_regular(
        75, default=None, require=False, array=False, references=tuple(NODE_TYPES)
    )
    base: Optional[Node] = p_regular(
        76, default=None, require=False, array=False, references=tuple(NODE_TYPES)
    )
