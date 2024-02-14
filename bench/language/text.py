from typing import TYPE_CHECKING, Optional

from bench.language.const import StructType
from bench.language.node import LINK_TARGET_NODE_TYPES, Node, Struct, struct
from bench.language.property import p_regular

if TYPE_CHECKING:
    from bench.language import FieldPath


@struct(StructType.RICH_TEXT)
class RichText(Struct):
    spans: list["RichTextSpan"] = p_regular(30, default_factory=list, struct=StructType.RICH_TEXT)

    def __content_str__(self):
        spans_strs: list[str] = [span.__content_str__() for span in self.spans]
        return "".join(spans_strs)

    @staticmethod
    def plain(text: str) -> "RichText":
        return RichText(spans=[RichTextSpan(text=text)])


@struct(StructType.RICH_TEXT_SPAN)
class RichTextSpan(Struct):
    # plain text
    text: str | None = p_regular(30, default="")
    # mentions
    reference: Node | None = p_regular(
        31, array=False, default=None, require=False, references=LINK_TARGET_NODE_TYPES
    )
    path: Optional["FieldPath"] = p_regular(
        32, array=False, default=None, require=False, struct=StructType.FIELD_PATH
    )

    # flags
    is_bold: bool = p_regular(40, default=False)
    is_italic: bool = p_regular(41, default=False)
    is_strikethrough: bool = p_regular(42, default=False)
    is_underline: bool = p_regular(43, default=False)
    is_code: bool = p_regular(44, default=False)

    def __content_str__(self):
        if self.text:
            return self.text
        elif self.reference:
            return f"@{self.reference.name}"
        else:
            return ""
