from typing import TYPE_CHECKING, Optional

from bench.language.const import StructType
from bench.language.node import LINK_TARGET_NODE_TYPES, Node, Struct, struct, struct_component
from bench.language.property import p_regular
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language.view import ColorType


@struct(StructType.TEXT)
class Text(Struct):
    lines: list["TextLine"] = p_regular(32, array=True, struct=StructType.TEXT_LINE)

    def __content_str__(self):
        return "\n".join(line.__content_str__() for line in self.lines)

    @staticmethod
    def plain(text: str) -> "Text":
        return Text(lines=[TextLine.plain(text)])


@struct_component
class TextOptions(Struct):
    # color?
    color: Optional["ColorType"] = p_regular(50, default=None)
    # flags
    is_bold: bool = p_regular(60, default=False)
    is_italic: bool = p_regular(61, default=False)
    is_strikethrough: bool = p_regular(62, default=False)
    is_underline: bool = p_regular(63, default=False)
    is_code: bool = p_regular(64, default=False)


class TextLineType(IdEnum):
    PLAIN = 1
    # heading
    HEADING_SMALL = 6
    HEADING_MEDIUM = 7
    HEADING_LARGE = 8
    # callout
    CALLOUT = 12
    # list
    LIST_BULLET = 16
    LIST_NUMBERED = 17
    # divider
    DIVIDER = 21


@struct(StructType.TEXT_LINE)
class TextLine(TextOptions):
    type: TextLineType = p_regular(30, default=TextLineType.PLAIN)
    spans: list["TextSpan"] = p_regular(33, array=True, struct=StructType.TEXT_SPAN)

    def __content_str__(self):
        return "".join(span.__content_str__() for span in self.spans)

    @staticmethod
    def plain(text: str) -> "TextLine":
        return TextLine(type=TextLineType.PLAIN, spans=[TextSpan(content=text)])


@struct(StructType.TEXT_SPAN, inline=True)
class TextSpan(TextOptions):
    content: str | None = p_regular(33, default=None)
    node: Optional[Node] = p_regular(
        34, array=False, default=None, require=False, references=LINK_TARGET_NODE_TYPES
    )

    def __content_str__(self):
        if self.content:
            return repr(self.content)
        elif self.node:
            return f"@{self.node_reference!r}"
        else:
            return ""
