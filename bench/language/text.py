from typing import TYPE_CHECKING

from bench.language.const import StructType
from bench.language.node import LINK_TARGET_NODE_TYPES, Node, Struct, struct
from bench.language.property import p_regular

if TYPE_CHECKING:
    from bench.language import ValueReference


@struct(StructType.TEXT)
class Text(Struct):
    spans: list["TextSpan"] = p_regular(32, default_factory=list, struct=StructType.TEXT)

    def __content_str__(self):
        spans_strs: list[str] = [span.__content_str__() for span in self.spans]
        return "".join(spans_strs)

    @staticmethod
    def plain(text: str) -> "Text":
        return Text(spans=[TextSpan(text=text)])


@struct(StructType.TEXT_SPAN)
class TextSpan(Struct):
    # plain text
    content: str | None = p_regular(30, default=None)
    # mentions
    node_reference: Node | None = p_regular(
        31, array=False, default=None, require=False, references=LINK_TARGET_NODE_TYPES
    )
    value_reference: ValueReference | None = p_regular(
        32, array=False, default=None, require=False, struct=StructType.VALUE_REFERENCE
    )

    # flags
    is_bold: bool = p_regular(40, default=False)
    is_italic: bool = p_regular(41, default=False)
    is_strikethrough: bool = p_regular(42, default=False)
    is_underline: bool = p_regular(43, default=False)
    is_code: bool = p_regular(44, default=False)

    def __content_str__(self):
        if self.content:
            return self.content
        elif self.node_reference:
            return f"@{self.node_reference!r}"
        else:
            return ""
