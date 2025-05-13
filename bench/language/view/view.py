from typing import TYPE_CHECKING, Optional, Union

from bench.language.core import (
    VIEW_NODE_TYPES,
    BuiltinEnum,
    Code,
    EnumType,
    IsModal,
    IsNamed,
    IsTemplatable,
    Node,
    NodeType,
    PageNode,
    Selection,
    StructType,
    Text,
    enum_,
    node_,
    node_component_,
    p_node_parent,
    p_regular,
)
from bench.pb2 import ViewData

if TYPE_CHECKING:
    from bench.language import Message, Page, Space

# pyright: reportIncompatibleVariableOverride=false


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


#
# Views
#


@node_component_()
class ViewBase(
    IsTemplatable,
    IsModal,
    IsNamed,
    PageNode[ViewData],
):
    """A View is a graphical interface."""

    parent: Union["Space", "ViewBase", "Page", None] = p_node_parent(
        4, NodeType.SPACE, *VIEW_NODE_TYPES.tuple, NodeType.PAGE
    )

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
class ButtonView(ViewBase):
    """A Button view."""

    variant: ButtonVariant = p_regular(40, default=ButtonVariant.PRIMARY)


@enum_(EnumType.PICKER_VARIANT)
class PickerVariant(BuiltinEnum):
    MULTI_TOGGLE = 1
    DROPDOWN = 2
    DROPDOWN_LARGE = 3


@node_(NodeType.NUMBER_VIEW)
class NumberView(ViewBase):
    """A Number view."""

    value: Optional[float] = p_regular(40, default=None)


@node_(NodeType.TEXT_VIEW)
class TextView(ViewBase):
    """A Text view."""

    value: Optional[Text] = p_regular(
        40, default=None, array=False, require=False, struct=StructType.TEXT
    )


@node_(NodeType.CODE_VIEW)
class CodeView(ViewBase):
    """A Code view."""

    value: Optional[Code] = p_regular(
        40, default=None, array=False, require=False, struct=StructType.CODE
    )


@node_(NodeType.TOGGLE_VIEW)
class ToggleView(ViewBase):
    """A Toggle view."""

    value: Optional[bool] = p_regular(40, default=None)


@node_(NodeType.THREAD_VIEW)
class ThreadView(ViewBase):
    """A Thread view."""

    draft_text: Optional[Text] = p_regular(40, default=None)
    draft_nodes: list[Node] = p_regular(41, require=False, array=True, references="any")
    draft_reply_to: Optional["Message"] = p_regular(
        42, default=None, require=False, array=False, references=NodeType.MESSAGE
    )
