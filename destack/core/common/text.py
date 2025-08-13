from typing import TYPE_CHECKING, Optional

from ..builtin import (
    EnumType,
    FlagEnum,
    Node,
    OptionEnum,
    ReferenceType,
    Struct,
    StructType,
    declare_enum,
    declare_option,
    declare_property,
    declare_struct,
)

if TYPE_CHECKING:
    pass


@declare_enum(EnumType.TEXT_SPAN_TYPE)
class TextSpanType(OptionEnum):
    TEXT = declare_option(1, description="Formatted text")
    HARD_BREAK = declare_option(2, description="Hard break")
    NODE = declare_option(10, description="Reference to a Node")
    LINK = declare_option(11, description="Hyperlink")
    EQUATION = declare_option(20, description="TeX equation")


@declare_enum(EnumType.TEXT_STYLE_FLAG)
class TextStyleFlag(FlagEnum):
    """A flag that can be applied to a TextSpan."""

    DEFAULT = declare_option(0)
    BOLD = declare_option(1)
    ITALIC = declare_option(2)
    STRIKETHROUGH = declare_option(4)
    UNDERLINE = declare_option(8)
    CODE = declare_option(16)


@declare_struct(StructType.TEXT_SPAN)
class TextSpan(Struct):
    """A span of text with optional formatting"""

    type: TextSpanType = declare_property(
        100,
        default=TextSpanType.TEXT,
        tag=None,
    )
    content: Optional[str] = declare_property(
        101,
        tag=None,
    )
    node: Optional[Node] = declare_property(
        102,
        reference_type=ReferenceType.IDENTITY,
        tag=None,
    )
    url: Optional[str] = declare_property(
        105,
        tag=None,
    )
    style_flags: TextStyleFlag = declare_property(
        110,
        tag=None,
        default=TextStyleFlag.DEFAULT,
    )


@declare_struct(StructType.TEXT)
class Text(Struct):
    """
    Rich Text; a single paragraph composed of TextSpans with inline formatting.
    """

    spans: list[TextSpan] = declare_property(
        103,
        tag=None,
    )
    style_flags: TextStyleFlag = declare_property(
        110,
        tag=None,
        default=TextStyleFlag.DEFAULT,
    )
