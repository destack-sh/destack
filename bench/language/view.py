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
    # 'intrinsic' roots for editor
    PAGE = 1
    BLOCK = 2
    EXPLORER = 20
    HISTORY = 21
    RESOURCE = 22
    INSPECTOR = 23
    LIBRARY = 24
    LOG = 25
    ACCESS = 26
    SCREEN = 49

    # containers
    WINDOWED = 50
    WINDOW = 51  # (force window appearance)
    TABBED = 52
    STEPPED = 55
    SPLIT = 57
    STACK = 60
    DISCLOSURE = 61
    GRID = 62
    ROW = 63
    COLUMN = 64
    # data
    LIST = 70
    TABLE = 71
    FEED = 72
    # group
    GROUP = 75
    FORM = 76
    MENU = 77
    SECTION = 78

    # presentation
    SPACER = 80
    DIVIDER = 81
    SHAPE = 83
    CHART = 84
    PROGRESS = 85
    AVATAR = 86
    BADGE = 87

    # controls
    BUTTON = 100
    LINK = 101

    # content
    VALUE = 120  # (generic value based on type)
    # numeric
    SLIDER = 133
    NUMBER = 134
    PHONE = 135
    # stringy
    STRING = 140
    TEXT = 141
    CODE = 142
    JSON = 143
    # selection
    TOGGLE = 150
    CHECKBOX = 151
    CHECKBOX_GROUP = 152
    PICKER = 153
    DATE = 154
    TIME = 155
    CALENDAR = 156
    COLOR = 157
    # file
    FILE = 160
    DOCUMENT = 161
    ICON = 162
    IMAGE = 163
    VIDEO = 164
    AUDIO = 165
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
