from typing import TYPE_CHECKING, Optional

from bench.language.const import EnumType, StructType, enum_
from bench.language.node import LINK_TARGET_NODE_TYPES, Node, Struct, struct, struct_component
from bench.language.property import p_regular
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import ColorType, Icon


@struct(StructType.TEXT)
class Text(Struct):
    lines: list["TextLine"] = p_regular(32, array=True, struct=StructType.TEXT_LINE)

    def __content_str__(self):
        return "\n".join(line.__content_str__() for line in self.lines)

    @staticmethod
    def plain(text: str) -> "Text":
        return Text(lines=[TextLine.plain(text)])


@struct_component()
class TextOptions(Struct):
    # color?
    color: Optional["ColorType"] = p_regular(50, default=None)
    # flags
    is_bold: Optional[bool] = p_regular(60, default=None)
    is_italic: Optional[bool] = p_regular(61, default=None)
    is_strikethrough: Optional[bool] = p_regular(62, default=None)
    is_underline: Optional[bool] = p_regular(63, default=None)
    is_code: Optional[bool] = p_regular(64, default=None)


@enum_(EnumType.TEXT_LINE_TYPE)
class TextLineType(IdEnum):
    PLAIN = 1
    # heading
    HEADING_SMALL = 10
    HEADING_MEDIUM = 11
    HEADING_LARGE = 12
    # callout
    CALLOUT = 20
    QUOTE = 21
    # list
    LIST_BULLET = 30
    LIST_NUMBERED = 31
    # divider
    DIVIDER = 40


@struct(StructType.TEXT_LINE)
class TextLine(TextOptions):
    type: TextLineType = p_regular(30, default=TextLineType.PLAIN)
    spans: list["TextSpan"] = p_regular(33, array=True, struct=StructType.TEXT_SPAN)
    icon: Optional["Icon"] = p_regular(34, default=None, struct=StructType.ICON)

    def __content_str__(self):
        return "".join(span.__content_str__() for span in self.spans)

    @staticmethod
    def plain(text: str) -> "TextLine":
        return TextLine(type=TextLineType.PLAIN, spans=[TextSpan(content=text)])


@struct(StructType.TEXT_SPAN, inline=True)
class TextSpan(TextOptions):
    content: Optional[str] = p_regular(33, default=None)
    node: Optional[Node] = p_regular(
        34, array=False, default=None, require=False, references=LINK_TARGET_NODE_TYPES
    )

    def __content_str__(self):
        if self.content:
            return repr(self.content)
        elif self.node:
            return f"@{self.node!r}"
        else:
            return ""
