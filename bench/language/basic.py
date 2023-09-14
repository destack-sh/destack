import enum
import itertools
import re
from dataclasses import dataclass
from typing import Optional, Union
from uuid import UUID

from bench.language.core import (
    HasIssues,
    ModuleNode,
    ModuleNodeType,
    ModuleVisitor,
    Scope,
    Statement,
    StatementType,
    TypedNodeReference,
    node,
)
from bench.language.issue import IssueType

# common statements


@node(tracked=[])
class Blank(Statement):
    """A blank statement."""

    type: StatementType = StatementType.BLANK

    def _visit(self, visitor: ModuleVisitor) -> None:
        for child in self.children:
            visitor.visit_child(child)


@node
class HasText(HasIssues):
    """Some instruction text with optional references."""

    _text_spans: list["TextSpan"] | None = None

    @property
    def text_plain(self) -> Optional[str]:
        if self._text_spans is None:
            return None
        return "".join(str(s) for s in self._text_spans)

    @property
    def text_spans(self) -> list["TextSpan"]:
        if self._text_spans is None:
            raise ValueError(f"{self} is not interpreted")
        return self._text_spans

    @property
    def mentions(self) -> list["TextMention"]:
        return [span for span in self.text_spans if isinstance(span, TextMention)]

    def _clear(self) -> None:
        self._text_spans = None

    def _interp(self, scope: Scope) -> None:
        if self.text is None:
            return
        self._text_spans = parse_text_html(self.text)

        # resolve references
        for span in self._text_spans:
            if not isinstance(span, TextMention):
                continue
            if isinstance(span.reference, ModuleNode):
                continue
            resolved = None
            if span.reference is not None:
                resolved = scope.lookup(span.reference.ref, node_t=span.reference.type)
            if resolved is None:
                self._on_issue(
                    type=IssueType.MISSING_REFERENCE, subject=self, path=span.reference_path
                )
                continue
            span.reference = resolved  # success

    def _visit(self, visitor: ModuleVisitor) -> None:
        if self._text_spans is None:
            return
        for span in self._text_spans:
            if isinstance(span, TextMention) and isinstance(span.reference, ModuleNode):
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
    reference: Union[TypedNodeReference, ModuleNode]
    reference_path: Optional[str]

    def __str__(self):
        return f"@{self.reference}"

    def __repr__(self):
        return f"<TextMention {self}>"

    @property
    def reference_ck(self) -> UUID:
        if isinstance(self.reference, ModuleNode):
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


class TextHeadingLevel(enum.IntEnum):
    """Classic headings big to small."""

    H1 = 1
    H2 = 2
    H3 = 3


@node(tracked=["text", "heading_level"])
class Text(HasText, Statement):
    heading_level: Optional[TextHeadingLevel] = None
    type: StatementType = StatementType.TEXT

    def _clear(self) -> None:
        Statement._clear(self)
        HasText._clear(self)

    def _interp(self, scope: Scope) -> None:
        HasText._interp(self, scope)

    def _visit(self, visitor: ModuleVisitor) -> None:
        for child in self.children:
            visitor.visit_child(child)
        HasText._visit(self, visitor)


# avoid circular import because Reference IsFlowNode
from bench.language.flow import IsFlowNode  # noqa: E402
from bench.language.tag import HasTags  # noqa: E402


@node(tracked=["reference"])
class Reference(Statement, HasTags, HasText, IsFlowNode):
    """A reference to another statement."""

    type: StatementType = StatementType.REFERENCE

    def _clear(self) -> None:
        HasTags._clear(self)
        IsFlowNode._clear(self)

    def _interp(self, scope: Scope) -> None:
        HasTags._interp(self, scope)
        IsFlowNode._interp(self, scope)
        resolved = None
        if not isinstance(self.reference, Statement):
            resolved = scope.lookup(self.reference)
        if resolved is None:
            self._on_issue(type=IssueType.MISSING_REFERENCE, subject=self, path="<root>")
        else:
            self.reference = resolved

    def _visit(self, visitor: "ModuleVisitor") -> None:
        for n in itertools.chain(self.children, self.tags, self.triggers):
            visitor.visit_child(n)
        if isinstance(self.reference, Statement):
            visitor.visit_reference(self.reference)
        HasText._visit(self, visitor)


# hard-coded, do not change ever :BenchUuidNamespace
BENCH_UUID_NAMESPACE = UUID("d822dab7-41ad-4706-a9c8-4379e15b2ed0")
