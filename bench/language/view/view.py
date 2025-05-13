from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.language.core import (
    BuiltinEnum,
    BuiltinObject,
    Code,
    EnumType,
    IsClaimable,
    IsModal,
    IsNamed,
    IsOwnable,
    IsTemplatable,
    Node,
    NodeReference,
    NodeType,
    PageNode,
    Selection,
    Struct,
    StructType,
    Text,
    enum_,
    node_,
    node_component_,
    p_node_parent,
    p_regular,
    struct_,
)
from bench.pb2 import ViewData

if TYPE_CHECKING:
    from bench.language import Message, Page, Space, Type

# pyright: reportIncompatibleVariableOverride=false


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


@node_component_()
class IsView(BuiltinObject):
    pass


@node_(NodeType.VIEW)
class View(
    IsTemplatable,
    IsModal,
    IsNamed,
    IsOwnable,
    IsClaimable,
    PageNode[ViewData],
):
    """A View is a graphical interface."""

    parent: Union["Space", "View", "Page", None] = p_node_parent(
        4, NodeType.SPACE, NodeType.VIEW, NodeType.PAGE
    )

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

    # behavior
    focus: Optional[Node] = p_regular(
        70, default=None, require=False, array=False, references="any"
    )
    selection: Optional[Selection] = p_regular(
        71, default=None, require=False, struct=StructType.SELECTION
    )
    ...  # actions/effects/...

    # flags
    # is_input: bool = p_regular(82, default=False)
    # is_inline: bool = p_regular(83, default=False)
    # is_minimal: bool = p_regular(84, default=False)


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


@node_(NodeType.BUTTON_VIEW)
class ButtonView(View):
    """A Button view."""

    variant: ButtonVariant = p_regular(40, default=ButtonVariant.PRIMARY)


@enum_(EnumType.PICKER_VARIANT)
class PickerVariant(BuiltinEnum):
    MULTI_TOGGLE = 1
    DROPDOWN = 2
    DROPDOWN_LARGE = 3


@node_(NodeType.NUMBER_VIEW)
class NumberView(View):
    """A Number view."""

    value: Optional[float] = p_regular(40, default=None)


@node_(NodeType.TEXT_VIEW)
class TextView(View):
    """A Text view."""

    value: Optional[Text] = p_regular(
        40, default=None, array=False, require=False, struct=StructType.TEXT
    )


@node_(NodeType.CODE_VIEW)
class CodeView(View):
    """A Code view."""

    value: Optional[Code] = p_regular(
        40, default=None, array=False, require=False, struct=StructType.CODE
    )


@node_(NodeType.TOGGLE_VIEW)
class ToggleView(View):
    """A Toggle view."""

    value: Optional[bool] = p_regular(40, default=None)


@node_(NodeType.THREAD_VIEW)
class ThreadView(View):
    """A Thread view."""

    draft_text: Optional[Text] = p_regular(40, default=None)
    draft_nodes: list[Node] = p_regular(41, require=False, array=True, references="any")
    draft_reply_to: Optional["Message"] = p_regular(
        42, default=None, require=False, array=False, references=NodeType.MESSAGE
    )
