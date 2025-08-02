import re
import textwrap
from collections.abc import Mapping, Sequence
from typing import TYPE_CHECKING, Any, Optional, assert_never

from destack.language.core import (
    Enum,
    EnumType,
    Node,
    StructFrozen,
    StructType,
    builtin_enum,
    builtin_property,
    builtin_struct,
)

if TYPE_CHECKING:
    from destack.language import NodeReference


@builtin_enum(EnumType.TEXT_SPAN_TYPE)
class TextSpanType(Enum):
    TEXT = 1, "Formatted text"
    HARD_BREAK = 2, "Hard break"
    MENTION = 10, "Reference to a Node"
    LINK = 11, "Hyperlink"
    CITATION = 12, "Citation"
    EQUATION = 20, "TeX equation"


@builtin_struct(StructType.TEXT_SPAN, frozen=True)
class TextSpan(StructFrozen):
    """A span of text with optional formatting"""

    type: TextSpanType = builtin_property(100, default=TextSpanType.TEXT)
    content: Optional[str] = builtin_property(101)
    node: Optional[Node] = builtin_property(102)
    if TYPE_CHECKING:
        node_ptr: Optional[NodeReference] = None
    url: Optional[str] = builtin_property(105)

    is_bold: Optional[bool] = builtin_property(150)
    is_italic: Optional[bool] = builtin_property(151)
    is_strikethrough: Optional[bool] = builtin_property(152)
    is_underline: Optional[bool] = builtin_property(153)
    is_code: Optional[bool] = builtin_property(154)

    def _to_option_kwargs(self):
        kwargs = {}
        for prop in self.__declaration__.properties:
            value = getattr(self, prop.name)
            if value is not None:
                kwargs[prop.name] = value
        return kwargs

    @staticmethod
    def hard_break() -> "TextSpan":
        return TextSpan(type=TextSpanType.HARD_BREAK)


@builtin_struct(StructType.TEXT, frozen=True)
class Text(StructFrozen):
    """
    Rich Text; a single paragraph composed of TextSpans with inline formatting.
    """

    spans: list[TextSpan] = builtin_property(103)

    is_bold: Optional[bool] = builtin_property(150)
    is_italic: Optional[bool] = builtin_property(151)
    is_strikethrough: Optional[bool] = builtin_property(152)
    is_underline: Optional[bool] = builtin_property(153)
    is_code: Optional[bool] = builtin_property(154)

    def _to_option_kwargs(self):
        kwargs = {}
        for prop in self.__declaration__.properties:
            value = getattr(self, prop.name)
            if value is not None:
                kwargs[prop.name] = value
        return kwargs

    def __contains__(self, item: str | Node) -> bool:
        if isinstance(item, str):
            for span in self.spans:
                if (content := span.content) and item in content:
                    return True
        elif isinstance(item, Node):
            for span in self.spans:
                if span.node_ptr is not None and span.node_ptr.id == item.id:
                    return True
        else:
            assert_never(item)
        return False

    def to_markdown(self) -> str:
        """Render the Text as markdown."""
        return text_to_markdown(self)

    def to_plain(self, max_characters: Optional[int] = None) -> str:
        """Render the Text as plain text without any formatting or markdown."""
        text_parts: list[str] = []
        if max_characters is not None:
            current_length = 0
            for span in self.spans:
                if span.type == TextSpanType.TEXT and span.content:
                    content = span.content
                    remaining = max_characters - current_length
                    if remaining <= 0:
                        break
                    elif len(content) > remaining:
                        text_parts.append(content[:remaining] + "...")
                        break
                    else:
                        text_parts.append(content)
                        current_length += len(content)
        else:
            for span in self.spans:
                if span.type == TextSpanType.TEXT:
                    text_parts.append(span.content or "")
        return "".join(text_parts)

    @property
    def is_empty(self) -> bool:
        for span in self.spans:  # noqa: SIM110
            if span.type != TextSpanType.TEXT or span.content:
                return False
        return True

    @staticmethod
    def paragraph(text: str) -> "Text":
        return Text(spans=_parse_inline(text))

    plain = paragraph

    @staticmethod
    def from_markdown(markdown: str) -> "Text":
        return markdown_to_text(markdown)

    to_string = to_markdown

    @staticmethod
    def plain_text(text: str) -> "Text":
        return Text(spans=[TextSpan(content=text)])

    from_string = plain_text

    @staticmethod
    def empty() -> "Text":
        return Text(spans=[])


#
# Parsing
#

MARKER_TO_FLAG = {
    "*": "is_italic",
    "_": "is_italic",
    "**": "is_bold",
    "__": "is_bold",
    "~~": "is_strikethrough",
    "<u>": "is_underline",
    "`": "is_code",
}


_MARKER_PATTERN = re.compile(
    r"(\$\$|\*\*|~~|`|<u>|<\/u>|<br>|\[\^[a-zA-Z0-9]+\]|\[[^[\]]+\]|\[@[a-zA-Z0-9_]+\])"
)
_LINK_PATTERN = re.compile(
    r"(?:https?://)?(?:www\.)?(?:[a-zA-Z0-9-]+\.)+[a-zA-Z]{2,}(?::\d{1,5})?(?:/\S*)?"
)


def _parse_inline_raw_urls(text: str, base: Mapping[str, Any]) -> list[TextSpan]:
    """Extract URLs from text as link spans."""
    spans = []
    pos = 0

    while pos < len(text):
        url_match = _LINK_PATTERN.search(text[pos:])
        if not url_match:
            # no more URLs, add remaining text
            if pos < len(text):
                spans.append(TextSpan(content=text[pos:], **base))
            break

        url_start = url_match.start() + pos
        url_end = url_match.end() + pos

        # add text before URL if any
        if url_start > pos:
            spans.append(TextSpan(content=text[pos:url_start], **base))

        # add the URL as a link
        url_text = text[url_start:url_end]
        url = url_text if url_text.startswith(("http://", "https://")) else f"https://{url_text}"
        spans.append(TextSpan(type=TextSpanType.LINK, content=url_text, url=url, **base))

        pos = url_end

    return spans


def _parse_inline_raw(
    text: str,
    pos: int = 0,
    end_marker: str | None = None,
    base: dict[str, Any] | None = None,
) -> tuple[list[TextSpan], int]:
    """
    Parse a string of text into a list of TextSpans, with optional end marker.
    """
    if base is None:
        base = {}
    spans: list[TextSpan] = []
    while pos < len(text):
        m = _MARKER_PATTERN.search(text, pos)
        if not m:
            # Process the remaining text
            remaining_text = text[pos:]
            # Only check for URLs if there's a potential match
            if _LINK_PATTERN.search(remaining_text):
                spans.extend(_parse_inline_raw_urls(remaining_text, base))
            else:
                spans.append(TextSpan(content=remaining_text, **base))
            return spans, len(text)

        if m.start() > pos:
            # Handle text segment before the marker
            segment = text[pos : m.start()]
            # Only check for URLs if there's a potential match
            if _LINK_PATTERN.search(segment):
                spans.extend(_parse_inline_raw_urls(segment, base))
            else:
                spans.append(TextSpan(content=segment, **base))

        marker = m.group(0)
        pos = m.end()
        # if the marker matches the expected closing marker
        if end_marker is not None and marker == end_marker:
            return spans, pos
        # handle hard break marker
        if marker == "<br>":
            spans.append(TextSpan.hard_break())
            continue
        # inline equation marker: $$...$$
        if marker == "$$":
            inner, pos = _parse_inline_raw(text, pos, end_marker="$$", base=base.copy())
            merged = _merge_spans(inner)
            content = "".join(sp.content or "" for sp in merged)
            spans.append(TextSpan(type=TextSpanType.EQUATION, content=content, **base))
            continue
        # mention marker: [@identifier]
        if marker.startswith("[@") and marker.endswith("]"):
            identifier = marker[2:-1]
            node: Node | None = None
            node_ptr: NodeReference | None = None
            mention_span = TextSpan(
                type=TextSpanType.MENTION, content=identifier, node=node, node_ptr=node_ptr, **base
            )
            spans.append(mention_span)
            continue
        # opening marker: could be a citation or link
        if marker.startswith("[") and marker.endswith("]"):
            if marker.startswith("[^"):
                # citation markers
                citation_content = marker[2:-1]
                if pos < len(text) and text[pos] == "(":
                    end_paren = text.find(")", pos)
                    if end_paren == -1:
                        spans.append(TextSpan(content=marker, **base))
                        continue
                    url = text[pos + 1 : end_paren]
                    pos = end_paren + 1
                    spans.append(
                        TextSpan(
                            type=TextSpanType.CITATION, content=citation_content, url=url, **base
                        )
                    )
                    continue
                else:
                    spans.append(
                        TextSpan(type=TextSpanType.CITATION, content=citation_content, **base)
                    )
                    continue
            if pos < len(text) and text[pos] == "(":
                end_paren = text.find(")", pos)
                if end_paren == -1:
                    spans.append(TextSpan(content=marker, **base))
                    continue
                href = text[pos + 1 : end_paren]
                pos = end_paren + 1
                spans.append(
                    TextSpan(type=TextSpanType.LINK, content=marker[1:-1], url=href, **base)
                )
                continue
            else:
                # treat as literal text
                spans.append(TextSpan(content=marker, **base))
                continue
        # symmetric markers
        if marker in MARKER_TO_FLAG:
            flag = MARKER_TO_FLAG[marker]

            # check if this is likely a math operator or similar rather than a formatting marker
            # For single character markers, check if surrounded by whitespace or not followed by closing marker
            is_real_marker = False
            if len(marker) == 1:
                # check if followed by whitespace (or end of text)
                if pos >= len(text) or text[pos].isspace():
                    is_real_marker = True
                else:
                    # check if there's a matching closing marker in the remaining text
                    remaining = text[pos:]
                    if marker not in remaining:
                        is_real_marker = True

            if is_real_marker:
                spans.append(TextSpan(content=marker, **base))
                continue

            inner, pos = _parse_inline_raw(text, pos, end_marker=marker, base=base.copy())
            for sp in inner:
                setattr(sp, flag, True)
            spans.extend(inner)
            continue
        # unrecognized marker: treat as literal.
        spans.append(TextSpan(content=marker, **base))
    return spans, pos


def _parse_inline(text: str) -> list[TextSpan]:
    """
    Parse a string of text into a list of TextSpans.
    """
    spans, _ = _parse_inline_raw(text, 0, None, {})
    return _merge_spans(spans)


def _merge_spans(spans: list[TextSpan]) -> list[TextSpan]:
    """
    Merge adjacent spans with identical formatting.
    """
    if not spans:
        return spans
    merged = [spans[0]]
    for sp in spans[1:]:
        if (
            sp.type == TextSpanType.TEXT
            and merged[-1].type == TextSpanType.TEXT
            and _span_format_key(merged[-1]) == _span_format_key(sp)
        ):
            content = (merged[-1].content or "") + (sp.content or "")
            new_span = TextSpan(
                type=TextSpanType.TEXT,
                content=content,
                is_bold=merged[-1].is_bold,
                is_italic=merged[-1].is_italic,
                is_strikethrough=merged[-1].is_strikethrough,
                is_underline=merged[-1].is_underline,
                is_code=merged[-1].is_code,
            )
            merged[-1] = new_span
        else:
            merged.append(sp)
    return merged


def _span_format_key(span: TextSpan) -> tuple:
    """
    A key that uniquely identifies the formatting of a TextSpan.
    """
    return (
        span.is_bold,
        span.is_italic,
        span.is_strikethrough,
        span.is_underline,
        span.is_code,
    )


def markdown_to_text(markdown: str) -> Text:
    """
    Parse markdown as a single paragraph Text.
    """
    # bail if nothing to parse
    if not markdown:
        return Text.empty()

    # parse as single line, ignoring line prefixes and multi-line constructs
    markdown = textwrap.dedent(markdown).strip()
    # Remove line breaks to treat as single paragraph
    markdown = " ".join(line.strip() for line in markdown.splitlines() if line.strip())

    spans = _parse_inline(markdown)
    return Text(spans=spans)


#
# Rendering
#

MARKER_ORDER = ["is_italic", "is_bold", "is_strikethrough", "is_underline", "is_code"]
MARKER_OPEN = {
    "is_italic": "*",
    "is_bold": "**",
    "is_strikethrough": "~~",
    "is_underline": "<u>",
    "is_code": "`",
}
MARKER_CLOSE = {
    "is_italic": "*",
    "is_bold": "**",
    "is_strikethrough": "~~",
    "is_underline": "</u>",
    "is_code": "`",
}


def _get_span_options(span: TextSpan) -> Sequence[str]:
    """
    Get the options for a span.
    """
    if span.type in (
        TextSpanType.EQUATION,
        TextSpanType.LINK,
        TextSpanType.CITATION,
        TextSpanType.HARD_BREAK,
    ):
        return ()
    return tuple(flag for flag in MARKER_ORDER if getattr(span, flag))


def _render_inline_raw(spans: Sequence[TextSpan]) -> str:
    """
    Render a list of TextSpan objects with inline markdown formatting.
    This function computes formatting state transitions between spans so that
    nested formatting markers (e.g. *…~~…~~…*) are rendered correctly.
    """

    result = []
    current_state: Sequence[str] = ()
    for span in spans:
        if span.type == TextSpanType.CITATION:
            for flag in reversed(current_state):
                result.append(MARKER_CLOSE[flag])
            if span.url:
                result.append(f"[^{span.content}]({span.url})")
            else:
                result.append(f"[^{span.content}]")
            current_state = ()
        elif span.type == TextSpanType.LINK:
            for flag in reversed(current_state):
                result.append(MARKER_CLOSE[flag])
            result.append(f"[{span.content}]({span.url})")
            current_state = ()
        elif span.type == TextSpanType.EQUATION:
            for flag in reversed(current_state):
                result.append(MARKER_CLOSE[flag])
            result.append(f"$${span.content}$$")
            current_state = ()
        elif span.type == TextSpanType.HARD_BREAK:
            for flag in reversed(current_state):
                result.append(MARKER_CLOSE[flag])
            result.append("<br>")
            current_state = ()
        elif span.type == TextSpanType.MENTION:
            for flag in reversed(current_state):
                result.append(MARKER_CLOSE[flag])
            if (node := span.node) is not None:
                result.append(f"[@{node.id}]")
            elif span.content:
                result.append(f"[@{span.content}]")
            else:
                result.append("[@???]")
            current_state = ()
        else:
            new_state = _get_span_options(span)
            min_len = min(len(new_state), len(current_state))
            common = 0
            while common < min_len and current_state[common] == new_state[common]:
                common += 1
            for flag in reversed(current_state[common:]):
                result.append(MARKER_CLOSE[flag])
            for flag in new_state[common:]:
                result.append(MARKER_OPEN[flag])
            result.append(span.content or "")
            current_state = new_state

    for flag in reversed(current_state):
        result.append(MARKER_CLOSE[flag])
    return "".join(result)


def text_to_markdown(text: Text) -> str:
    """
    Render a Text object as markdown.
    """
    return _render_inline_raw(text.spans)


TextIn = Text | str


def text(text_input: TextIn) -> Text:
    """Parse markdown as Text."""
    if isinstance(text_input, str):
        return markdown_to_text(text_input)
    else:
        assert isinstance(text_input, Text), f"expected Text, got {text_input!r}"
        return text_input


to_text = text
title = text
