from typing import TYPE_CHECKING, Any, Optional, Union

from bench.language.const import NodeType, StructType
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
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import Block, Icon, Package, Policy, Text


@_well_known_enum
class ViewType(IdEnum):
    #
    # Intrinsics
    #

    # 'kernel'
    # auth
    SIGN_IN = 1
    KEYMAP = 40
    PERFORMANCE = 50

    # 'system'
    PAGE = 101
    BLOCK = 102
    EXPLORER = 120
    HISTORY = 121
    RESOURCE = 122
    INSPECTOR = 123
    LIBRARY = 124
    LOG = 125
    ACCESS = 126
    CLIENT = 127

    #
    # General
    #

    # containers
    WINDOWED = 500
    WINDOW = 501  # (force window appearance)
    TABBED = 502
    STEPPED = 505
    SPLIT = 507
    STACK = 510
    DISCLOSURE = 511
    GRID = 512
    ROW = 513
    COLUMN = 514
    # data
    LIST = 520
    TABLE = 521
    FEED = 522
    # group
    GROUP = 535
    FORM = 536
    MENU = 537
    SECTION = 538

    # presentation
    SPACER = 540
    DIVIDER = 541
    SHAPE = 543
    CHART = 544
    PROGRESS = 545
    AVATAR = 546
    BADGE = 547

    # controls
    BUTTON = 500
    LINK = 501

    # content
    VALUE = 520  # (generic value based on type)
    # numeric
    SLIDER = 533
    NUMBER = 534
    PHONE = 535
    # stringy
    STRING = 540
    TEXT = 541
    CODE = 542
    JSON = 543
    # selection
    TOGGLE = 550
    CHECKBOX = 551
    CHECKBOX_GROUP = 552
    PICKER = 553
    DATE = 554
    TIME = 555
    CALENDAR = 556
    COLOR = 557
    # file
    FILE = 560
    DOCUMENT = 561
    ICON = 562
    IMAGE = 563
    VIDEO = 564
    AUDIO = 565
    ...


@_well_known_enum
class ColorType(IdEnum):
    """Built-in color types a la SwiftUI or Tailwind."""

    # surface
    PRIMARY = 1
    SECONDARY = 2
    ACCENT = 3
    # semantic
    SUCCESS = 10
    HINT = 11
    WARNING = 12
    DANGER = 13
    # actual (like Tailwind)
    SLATE = 20
    GRAY = 21
    ZINC = 22
    NEUTRAL = 23
    STONE = 24
    RED = 30
    ORANGE = 31
    AMBER = 32
    YELLOW = 33
    LIME = 34
    GREEN = 35
    EMERALD = 36
    TEAL = 37
    CYAN = 38
    SKY = 39
    BLUE = 40
    INDIGO = 41
    VIOLET = 42
    PURPLE = 43
    FUCHSIA = 44
    PINK = 45
    ROSE = 46


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


@struct(StructType.COLOR)
class Color(Struct):
    """A color value."""

    type: Optional[ColorType] = p_regular(31, default=None, validate=enum_validator(ColorType))
    shade: Optional[ColorShade] = p_regular(32, default=None, validate=enum_validator(ColorShade))
    hex: Optional[str] = p_regular(33, default=None)


@_well_known_enum
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
class View(HasViews, HasValues):
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
    # TODO :Cleanup :Architecture: View.node should probably just be in View.value
    #  (with relevant Views having that type... once we have the Value system more figured out)
    node: Optional["Node"] = p_regular(
        36, default=None, require=False, array=False, references=LINK_TARGET_NODE_TYPES
    )

    # content
    value_packed: Any = p_value_packed(40)
    value = p_value_runtime(packed=40)
    ...  # value/value source/file/node...

    # style
    ...  # font/border/corner/foreground/background/...

    # layout
    ...  # size/position/alignment/margin/padding/...

    # interaction
    ...  # behavior/effects/...

    # flags
    is_visible: bool = p_regular(80, default=True)
    is_disabled: bool = p_regular(81, default=False)
    is_loading: bool = p_regular(82, default=False)
    is_input: bool = p_regular(83, default=False)
    is_secret: bool = p_regular(84, default=False)

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
