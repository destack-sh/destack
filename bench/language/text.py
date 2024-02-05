import re
from dataclasses import dataclass
from typing import TYPE_CHECKING, NamedTuple, Optional, Union
from uuid import UUID

from bench.language.const import NodeType, StructType
from bench.language.node import LINK_TARGET_NODE_TYPES, Node, Struct, p_internal, p_regular, struct

if TYPE_CHECKING:
    from bench.language import FieldPath


@dataclass
class Text:
    spans: list["TextSpan"]
    _raw_text: str | None = None

    @property
    def mentions(self) -> list["TextMention"]:
        return [span for span in self.spans if isinstance(span, TextMention)]


@dataclass
class TextPlain:
    """A decoded non-overlapping span of a HasText text property."""

    text: str

    def __str__(self):
        return self.text

    def __repr__(self):
        return f"<TextPlain {self}>"

    @property
    def text_plain(self) -> Optional[str]:
        return self.text


TEXT_MENTION_REGEX = re.compile(
    r"<span data-ref-ck=\"(?P<ck>[a-f0-9-]+)\" data-ref-type=\"(?P<type>[a-zA-Z]+)\" data-ref-path=\"(?P<path>[^\"]*)\"></span>"
)
TEXT_MENTION_TEMPLATE = (
    '<span data-ref-ck="{ck}" data-ref-type="{type}" data-ref-path="{path}"></span>'
)
_TypedNodeReference = NamedTuple("TypedNodeReference", [("type", NodeType), ("ref", UUID)])


@dataclass
class TextMention:
    reference: Union[UUID, Node]
    reference_path: Optional[str] = None

    def __str__(self):
        return f"@{self.reference}"

    def __repr__(self):
        return f"<TextMention {self}>"

    def __copy__(self):
        return TextMention(reference=self.reference, reference_path=self.reference_path)

    @property
    def reference_ck(self) -> UUID:
        if isinstance(self.reference, Node):
            return self.reference.ck
        else:
            return self.reference.ref

    @property
    def text_plain(self) -> Optional[str]:
        return None

    @staticmethod
    def from_reference(reference: _TypedNodeReference, path: Optional[str] = None) -> "TextMention":
        return TextMention(reference=reference, reference_path=path)


TextSpan = Union[TextPlain, TextMention]


def parse_text_html(text_raw: str) -> list[TextSpan]:
    """
    Parse our subset of raw HTML with references as spans into TextSpans.
    User non-HTML characters are escaped (as in contenteditable).
    Spans are represented as span with data-reference attributes.
    :TextFormat

    e.g. "Hello <span data-reference-ck="02d1e2e0-7f6a-4b0e-3b0a-2b0a2b0a2b0a" data-reference-type="Block" data-reference-path="a.b.c"></span>!"
     -> [text("Hello "), Mention("02d1e2e0-7f6a-4b0e-3b0a-2b0a2b0a2b0a", "Block", "a.b.c"), TextSpan("!")]
    """
    spans = []
    last_end = 0

    for match in TEXT_MENTION_REGEX.finditer(text_raw):
        if match.start() > last_end:  # previous
            spans.append(TextPlain(text=text_raw[last_end : match.start()]))

        # mention
        ck = UUID(match.group("ck"))
        type = NodeType(match.group("type").upper())
        path = match.group("path") or None
        spans.append(TextMention(reference=_TypedNodeReference(type, ck), reference_path=path))

        last_end = match.end()

    if last_end < len(text_raw):  # remainder
        spans.append(TextPlain(text=text_raw[last_end:]))

    return spans


SIMPLE_MENTION_REGEX = re.compile(r"@(?P<ident>[a-zA-Z0-9_.]+)")


def parse_text_multi(text_raw: str) -> list[TextSpan]:
    """
    Parses text in the @<path> format (and in HTML format).
    """
    spans = []
    last_end = 0

    combined_regex = re.compile(
        r"|".join([SIMPLE_MENTION_REGEX.pattern, TEXT_MENTION_REGEX.pattern])
    )

    for match in combined_regex.finditer(text_raw):
        if match.start() > last_end:
            spans.append(TextPlain(text=text_raw[last_end : match.start()]))

        ident = match.group("ident") or UUID(match.group("ck"))
        node_type = match.group("type") or NodeType.BLOCK
        spans.append(
            TextMention(reference=_TypedNodeReference(node_type, ident), reference_path=None)
        )

        last_end = match.end()

    if last_end < len(text_raw):
        spans.append(TextPlain(text=text_raw[last_end:]))

    return spans


@struct(StructType.RICH_TEXT)
class RichText(Struct):
    spans: list["RichTextSpan"] = p_regular(30, default_factory=list, struct=StructType.RICH_TEXT)
    plain_text: str | None = p_internal(31, default=None)

    def __content_str__(self):
        return self.plain_text


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
        return self.text
