import re
from copy import deepcopy
from dataclasses import dataclass
from typing import Optional, Union
from uuid import UUID

from bench.language import IssueType
from bench.language.const import ModuleNodeType, NodeReference, TypedNodeReference
from bench.language.module import Node, NodeVisitor, ScopeNode, node_component, nproperty, nruntime


@node_component
class HasText(Node):
    """Some instruction text with optional references."""

    text: str | None = nproperty(default=None)
    _text_spans: list["TextSpan"] | None = nruntime(default=None, copy=lambda v: deepcopy(v))

    @property
    def text_plain(self) -> Optional[str]:
        if self._text_spans is None:
            return None
        return "".join(str(s) for s in self._text_spans)

    @property
    def text_spans(self) -> list["TextSpan"]:
        if self.text is None:
            return []
        elif self._text_spans is None:
            raise RuntimeError(f"{self!r} is not interpreted")
        else:
            return self._text_spans

    @property
    def mentions(self) -> list["TextMention"]:
        return [span for span in self.text_spans if isinstance(span, TextMention)]

    def _init_inner(self) -> None:
        if self._session is not None and self.text:
            # parse @<path> format on init during session
            self._set_untracked("text", render_text_html(parse_text_simple(self.text)))

    def _clear_inner(self) -> None:
        self._text_spans = None

    def _interp_inner(self, scope: ScopeNode) -> None:
        if self.text is None:
            return
        self._text_spans = parse_text_html(self.text)

        # resolve references
        for span in self._text_spans:
            if not isinstance(span, TextMention):
                continue
            if isinstance(span.reference, Node):
                continue
            resolved = None
            if span.reference is not None:
                if isinstance(span.reference, TypedNodeReference):
                    resolved = scope.lookup(span.reference.ref, node_t=span.reference.type)
                else:
                    resolved = scope.lookup(span.reference)
            if resolved is None:
                self._on_issue(
                    type=IssueType.MISSING_REFERENCE, subject=self, path=span.reference_path
                )
                continue
            span.reference = resolved  # success

    def _visit_inner(self, visitor: NodeVisitor) -> None:
        if self._text_spans is None:
            return
        for span in self._text_spans:
            if isinstance(span, TextMention) and isinstance(span.reference, Node):
                visitor.visit_reference(span.reference)


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


@dataclass
class TextMention:
    reference: Union[NodeReference | TypedNodeReference, Node]
    reference_path: Optional[str]

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
    def from_reference(reference: TypedNodeReference, path: Optional[str] = None) -> "TextMention":
        return TextMention(reference=reference, reference_path=path)


TextSpan = Union[TextPlain, TextMention]


def parse_text_html(text_raw: str) -> list[TextSpan]:
    """
    Parse our subset of raw HTML with references as spans into TextSpans.
    User non-HTML characters are escaped (as in contenteditable).
    Spans are represented as span with data-reference attributes.
    :TextFormat

    e.g. "Hello <span data-reference-ck="02d1e2e0-7f6a-4b0e-3b0a-2b0a2b0a2b0a" data-reference-type="Statement" data-reference-path="a.b.c"></span>!"
     -> [text("Hello "), mMention("02d1e2e0-7f6a-4b0e-3b0a-2b0a2b0a2b0a", "Statement", "a.b.c"), TextSpan("!")]
    """
    spans = []
    last_end = 0

    for match in TEXT_MENTION_REGEX.finditer(text_raw):
        if match.start() > last_end:  # previous
            spans.append(TextPlain(text=text_raw[last_end : match.start()]))

        # mention
        ck = UUID(match.group("ck"))
        type = ModuleNodeType(match.group("type"))
        path = match.group("path") or None
        spans.append(TextMention(reference=TypedNodeReference(type, ck), reference_path=path))

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
            spans_str.append(
                TEXT_MENTION_TEMPLATE.format(
                    type=span.reference.type, ck=span.reference.ref, path=span.reference_path or ""
                )
            )
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
            span.reference = TypedNodeReference(
                type=span.reference.type, ref=target_cks[span.reference_ck]
            )
    return render_text_html(spans)


def parse_text_simple(text_raw: str) -> list[TextSpan]:
    """
    Parses text in the @<path> format.
    """
    SIMPLE_MENTION_REGEX = re.compile(r"@(?P<path>[a-zA-Z0-9_.]+)")
    spans = []
    last_end = 0

    for match in SIMPLE_MENTION_REGEX.finditer(text_raw):
        if match.start() > last_end:
            spans.append(TextPlain(text=text_raw[last_end : match.start()]))

        path = match.group("path")
        spans.append(
            TextMention(
                reference=TypedNodeReference(ModuleNodeType.Statement, path), reference_path=None
            )
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
            path = span.reference.py_ident if isinstance(span.reference, Node) else "???"
            spans_str.append("@" + path)
        else:
            spans_str.append(span.text)
    return "".join(spans_str)
