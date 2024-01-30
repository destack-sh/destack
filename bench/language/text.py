import re
from dataclasses import dataclass
from typing import TYPE_CHECKING, NamedTuple, Optional, Union
from uuid import UUID

from bench.language.const import NodeType, StructType
from bench.language.node import (
    LINK_TARGET_NODE_TYPES,
    Node,
    Struct,
    struct,
    struct_internal,
    struct_property,
)

if TYPE_CHECKING:
    pass


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


def render_text_html(text_spans: list[TextSpan]) -> str:
    """
    Render text spans back into HTML-style raw text with references.
    See above for details.
    :TextFormat
    """
    spans_str = []
    for span in text_spans:
        if isinstance(span, TextMention):
            if isinstance(span.reference, Node):
                ref = TEXT_MENTION_TEMPLATE.format(
                    type=span.reference._type, ck=span.reference.ck, path=""
                )
            else:
                ref = TEXT_MENTION_TEMPLATE.format(
                    type=span.reference.type, ck=span.reference.ref, path=span.reference_path or ""
                )
            spans_str.append(ref)
        else:
            spans_str.append(span.text)
    return "".join(spans_str)


def patch_text_html(text_raw: str | None, target_cks: dict[UUID, UUID]) -> str | None:
    """
    Replaces references in text with new references.
    """
    if text_raw is None:
        return None
    spans = parse_text_html(text_raw)
    for span in spans:
        if isinstance(span, TextMention) and span.reference_ck in target_cks:
            span.reference = _TypedNodeReference(
                type=span.reference.type, ref=target_cks[span.reference_ck]
            )
    return render_text_html(spans)


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


def render_text_simple(text_spans: list[TextSpan]) -> str:
    """
    Renders text spans back into @<path> format.
    """
    spans_str = []
    for span in text_spans:
        if isinstance(span, TextMention):
            # should be smarter about qualifying/scoping paths here
            path = span.reference.py_ident if isinstance(span.reference, Node) else None
            spans_str.append("@" + (path or "???"))
        else:
            spans_str.append(span.text)
    return "".join(spans_str)


@struct(StructType.RICH_TEXT)
class RichText(Struct):
    spans: list["RichTextSpan"] = struct_property(
        30, default_factory=list, struct=StructType.RICH_TEXT
    )
    plain_text: str | None = struct_internal(31, default=None)

    def __content_str__(self):
        return self.plain_text


@struct(StructType.RICH_TEXT_SPAN)
class RichTextSpan(Struct):
    text: str = struct_property(30, default="")
    reference: Node | None = struct_property(
        32, array=False, default=None, require=False, references=LINK_TARGET_NODE_TYPES
    )
    is_bold: bool = struct_property(33, default=False)
    is_italic: bool = struct_property(34, default=False)
    is_underline: bool = struct_property(35, default=False)
    is_strikethrough: bool = struct_property(36, default=False)

    def __content_str__(self):
        return self.text
