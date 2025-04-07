import textwrap
from typing import TYPE_CHECKING, Any, List, Mapping, Optional, Sequence, assert_never
from uuid import UUID

import regex

from bench.language.core.code import Code

from .const import BuiltinEnum, EnumType, StructType, enum_
from .node import Node, NodeReference
from .object import BuiltinObject, object_
from .property import p_regular
from .struct import Struct, struct_

if TYPE_CHECKING:
    from bench.language import ColorType
    from bench.language.source.render import Aliasing, AliasingIn


@object_()
class TextOptionsBase(BuiltinObject):
    color: Optional["ColorType"] = p_regular(50, default=None)
    background_color: Optional["ColorType"] = p_regular(51, default=None)
    is_bold: Optional[bool] = p_regular(60, default=None)
    is_italic: Optional[bool] = p_regular(61, default=None)
    is_strikethrough: Optional[bool] = p_regular(62, default=None)
    is_underline: Optional[bool] = p_regular(63, default=None)
    is_code: Optional[bool] = p_regular(64, default=None)
    language: Optional[str] = p_regular(70, default=None)

    def _to_option_kwargs(self):
        kwargs = {}
        for prop in self.__declared_properties__.values():
            value = getattr(self, prop.name)
            if value is not None:
                kwargs[prop.name] = value
        return kwargs


@enum_(EnumType.TEXT_LINE_TYPE)
class TextLineType(BuiltinEnum):  # :TextLineType
    # basic
    PARAGRAPH = 1, "Paragraph", "Plain paragraph", "fas fa-align-left"
    # heading
    HEADING_1 = 10, "Heading 1", "Very big heading", "fas fa-heading"
    HEADING_2 = 11, "Heading 2", "Big heading", "fas fa-heading"
    HEADING_3 = 12, "Heading 3", "Medium heading", "fas fa-heading"
    HEADING_4 = 13, "Heading 4", "Small heading", "fas fa-heading"
    # highlight
    CALLOUT = 20, "Callout", "Callout", "fas fa-circle-exclamation"
    QUOTE = 21, "Quote", "Quote", "fas fa-quote-left"
    # list
    LIST_UNORDERED = 30, "Unorderd list", "Unorderd list", "fas fa-list-ul"
    LIST_ORDERED = 31, "Numbered list", "Numbered list", "fas fa-list-ol"
    # divider
    DIVIDER = 40, "Horizontal line", "Horizontal line", "fas fa-horizontal-rule"
    # table
    # ...
    # code
    CODE = 60, "Code", "Code", "fas fa-code"
    # reference
    # LINK, EMBED, ...


@enum_(EnumType.TEXT_SPAN_TYPE)
class TextSpanType(BuiltinEnum):
    TEXT = 1, "Formatted text"
    HARD_BREAK = 2, "Hard break"
    MENTION = 10, "Reference to a Node"
    LINK = 11, "Hyperlink"
    CITATION = 12, "Citation"
    EQUATION = 20, "TeX equation"


@struct_(StructType.TEXT_SPAN)
class TextSpan(TextOptionsBase, Struct):
    """A span of text with optional formatting"""

    type: TextSpanType = p_regular(30, default=TextSpanType.TEXT)
    content: Optional[str] = p_regular(33, default=None)
    node: Optional[Node] = p_regular(34, array=False, default=None, require=False, references="any")
    if TYPE_CHECKING:
        node_id: Optional[UUID] = None
        node_ck: Optional[UUID] = None
        node_ptr: Optional[NodeReference] = None
    url: Optional[str] = p_regular(35, default=None)

    def __content_str__(self) -> str:
        return _render_inline((self,))

    @staticmethod
    def hard_break() -> "TextSpan":
        return TextSpan(type=TextSpanType.HARD_BREAK)

    @staticmethod
    def new(
        type: TextSpanType,
        content: str | None = None,
        node: Node | None = None,
        url: str | None = None,
        is_bold: bool | None = None,
        is_italic: bool | None = None,
        is_strikethrough: bool | None = None,
        is_underline: bool | None = None,
        color: "ColorType | None" = None,
        background_color: "ColorType | None" = None,
    ) -> "TextSpan":
        return TextSpan(
            type=type,
            content=content,
            node=node,
            url=url,
            is_bold=is_bold,
            is_italic=is_italic,
            is_strikethrough=is_strikethrough,
            is_underline=is_underline,
            color=color,
            background_color=background_color,
        )


@struct_(StructType.TEXT_LINE)
class TextLine(TextOptionsBase, Struct):
    """
    A single line of text; may contain inline TextSpans, or hold a TextTable or such.
    """

    type: TextLineType = p_regular(30, default=TextLineType.PARAGRAPH)
    spans: List[TextSpan] = p_regular(33, array=True, struct=StructType.TEXT_SPAN)
    content: Optional[str] = p_regular(34, default=None)

    def __content_str__(self) -> str:
        return text_line_to_markdown(self)

    def __contains__(self, item: str | Node) -> bool:
        if isinstance(item, str):
            if (content := self.content) and item in content:
                return True
            for span in self.spans:
                if (content := span.content) and item in content:
                    return True
        elif isinstance(item, Node):
            for span in self.spans:
                if span.node_ck is not None and span.node_ck == item.ck:
                    return True
        else:
            assert_never(item)
        return False

    def to_markdown(self) -> str:
        """Render the TextLine as markdown."""
        return text_line_to_markdown(self)

    def to_plain(self) -> str:
        """Render the TextLine as plain text without any formatting or markdown."""
        text_parts: list[str] = []
        for span in self.spans:
            if span.type == TextSpanType.TEXT:
                text_parts.append(span.content or "")
        return "".join(text_parts)

    @staticmethod
    def paragraph(text: str) -> "TextLine":
        return TextLine(type=TextLineType.PARAGRAPH, spans=_parse_inline(text))

    plain = paragraph

    @staticmethod
    def code(text: str) -> "TextLine":
        return TextLine(type=TextLineType.CODE, content=text)

    @staticmethod
    def heading(level: int, text: str) -> "TextLine":
        if level == 1:
            return TextLine(type=TextLineType.HEADING_1, spans=_parse_inline(text))
        elif level == 2:
            return TextLine(type=TextLineType.HEADING_2, spans=_parse_inline(text))
        elif level == 3:
            return TextLine(type=TextLineType.HEADING_3, spans=_parse_inline(text))
        elif level == 4:
            return TextLine(type=TextLineType.HEADING_4, spans=_parse_inline(text))
        else:
            raise ValueError(f"invalid heading level: {level} (must be 1-4)")

    @staticmethod
    def callout(text: str) -> "TextLine":
        return TextLine(type=TextLineType.CALLOUT, spans=_parse_inline(text))

    @staticmethod
    def quote(text: str) -> "TextLine":
        return TextLine(type=TextLineType.QUOTE, spans=_parse_inline(text))

    @staticmethod
    def list_bullet(text: str) -> "TextLine":
        return TextLine(type=TextLineType.LIST_UNORDERED, spans=_parse_inline(text))

    @staticmethod
    def list_numbered(text: str) -> "TextLine":
        return TextLine(type=TextLineType.LIST_ORDERED, spans=_parse_inline(text))

    @staticmethod
    def divider() -> "TextLine":
        return TextLine(type=TextLineType.DIVIDER)

    @staticmethod
    def new(
        type: TextLineType,
        spans: List[TextSpan] | str | None = None,
        is_bold: bool | None = None,
        is_italic: bool | None = None,
        is_strikethrough: bool | None = None,
        is_underline: bool | None = None,
        color: "ColorType | None" = None,
    ) -> "TextLine":
        if spans is None:
            spans_list = []
        elif isinstance(spans, str):
            spans_list = _parse_inline(spans)
        else:
            spans_list = spans
        return TextLine(
            type=type,
            spans=spans_list,
            is_bold=is_bold,
            is_italic=is_italic,
            is_strikethrough=is_strikethrough,
            is_underline=is_underline,
            color=color,
        )


@struct_(StructType.TEXT)
class Text(Struct):
    """
    Rich Text; composed of TextLines with many markdown+ goodies.
    """

    lines: List[TextLine] = p_regular(32, array=True, struct=StructType.TEXT_LINE)

    def __contains__(self, item: str | Node) -> bool:
        return any(item in line for line in self.lines)

    def to_markdown(self) -> str:
        """Render the Text as markdown."""
        return text_to_markdown(self)

    def to_plain(self) -> str:
        """Render the Text as plain text without any formatting or markdown."""
        text_parts: list[str] = []
        for line in self.lines:
            text_parts.append(line.to_plain())
        return "\n".join(text_parts)

    @staticmethod
    def from_markdown(markdown: str) -> "Text":
        return markdown_to_text(markdown)

    to_string = to_markdown

    @staticmethod
    def plain(text: str) -> "Text":
        return Text(lines=[TextLine.paragraph(line) for line in text.splitlines()])

    from_string = plain

    @staticmethod
    def code(code: str | Code) -> "Text":
        if not isinstance(code, str):
            code = code.to_string()
        return Text(lines=[TextLine.code(code)])

    @staticmethod
    def empty() -> "Text":
        return Text(lines=[])


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


def _parse_color(color: str) -> "ColorType":
    """
    Parse a color from a string.
    """
    from bench.language import ColorType

    try:
        return ColorType[color.upper()]
    except KeyError as e:
        raise ValueError(
            f"invalid color: {color} (must be one of {', '.join(ColorType.__members__.keys())})"
        ) from e


_marker_pattern = regex.compile(
    r"(\$\$|\*\*|~~|`|<u>|<\/u>|<br>|\*|\[\^[a-zA-Z0-9]+\]|\[[a-zA-Z]+\]|\[\/[a-zA-Z]+\]|\[@[a-zA-Z0-9_]+\])"
)


def _parse_inline_raw(
    text: str,
    pos: int = 0,
    end_marker: str | None = None,
    base: dict[str, Any] | None = None,
    aliasing: "Aliasing | None" = None,
) -> tuple[list[TextSpan], int]:
    """
    Parse a string of text into a list of TextSpans, with optional end marker.
    """
    if base is None:
        base = {}
    spans: list[TextSpan] = []
    while pos < len(text):
        m = _marker_pattern.search(text, pos)
        if not m:
            spans.append(TextSpan(content=text[pos:], **base))
            return spans, len(text)
        if m.start() > pos:
            spans.append(TextSpan(content=text[pos : m.start()], **base))
        marker = m.group(0)
        pos = m.end()
        # if the marker matches the expected closing marker
        if end_marker is not None and marker == end_marker:
            return spans, pos
        # handle hard break marker
        if marker == "<br>":
            spans.append(TextSpan.hard_break())
            continue
        # closing color marker (if unexpected, treat as literal)
        if marker.startswith("[/") and marker.endswith("]"):
            if end_marker is not None and marker == end_marker:
                return spans, pos
            else:
                spans.append(TextSpan(content=marker, **base))
                continue
        # inline equation marker: $$...$$
        if marker == "$$":
            inner, pos = _parse_inline_raw(
                text, pos, end_marker="$$", base=base.copy(), aliasing=aliasing
            )
            merged = _merge_spans(inner)
            content = "".join(sp.content or "" for sp in merged)
            spans.append(TextSpan(type=TextSpanType.EQUATION, content=content, **base))
            continue
        # mention marker: [@identifier]
        if marker.startswith("[@") and marker.endswith("]"):
            identifier = marker[2:-1]
            mention_span = TextSpan(type=TextSpanType.MENTION, content=identifier, **base)
            if aliasing is not None:
                node = aliasing.resolve(identifier)
                if isinstance(node, Node):
                    mention_span.node = node
            spans.append(mention_span)
            continue
        # opening marker: could be a citation, link, or a color marker
        if marker.startswith("[") and marker.endswith("]") and not marker.startswith("[/"):
            if marker.startswith("[^"):
                # Handle citation markers
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
                color = _parse_color(marker[1:-1])
                closing = f"[/{marker[1:-1]}]"
                inner, pos = _parse_inline_raw(
                    text, pos, end_marker=closing, base=base.copy(), aliasing=aliasing
                )
                for sp in inner:
                    sp.color = color
                spans.extend(inner)
                continue
        # <u> marker with explicit closing </u>
        if marker == "<u>":
            inner, pos = _parse_inline_raw(
                text, pos, end_marker="</u>", base=base.copy(), aliasing=aliasing
            )
            for sp in inner:
                sp.is_underline = True
            spans.extend(inner)
            continue
        # symmetric markers
        if marker in {"*", "**", "~~", "`"}:
            flag = MARKER_TO_FLAG[marker]
            inner, pos = _parse_inline_raw(
                text, pos, end_marker=marker, base=base.copy(), aliasing=aliasing
            )
            for sp in inner:
                setattr(sp, flag, True)
            spans.extend(inner)
            continue
        # unrecognized marker: treat as literal.
        spans.append(TextSpan(content=marker, **base))
    return spans, pos


def _parse_inline(text: str, aliasing: "Aliasing | None" = None) -> list[TextSpan]:
    """
    Parse a string of text into a list of TextSpans.
    """
    spans, _ = _parse_inline_raw(text, 0, None, {}, aliasing)
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
            merged[-1].content = (merged[-1].content or "") + (sp.content or "")
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
        span.color,
        span.background_color,
    )


def markdown_line_to_line(line: str, aliasing: "Aliasing | None" = None) -> TextLine:
    """
    Parse a markdown line into a TextLine.
    """
    from bench.language import ColorType

    line_color = None
    stripped = line.strip()
    m = regex.match(r"^\[([a-zA-Z]+)\](.*)\[\/\1\]\s*$", stripped)
    if m:
        line_color = m.group(1)
        content = m.group(2).strip()
    else:
        content = line
    ttype = TextLineType.PARAGRAPH
    content_stripped = content.lstrip()
    if content_stripped.startswith("#"):
        count = 0
        for ch in content_stripped:
            if ch == "#":
                count += 1
            else:
                break
        if 1 <= count <= 4 and content_stripped[count : count + 1] == " ":
            if count == 1:
                ttype = TextLineType.HEADING_1
            elif count == 2:
                ttype = TextLineType.HEADING_2
            elif count == 3:
                ttype = TextLineType.HEADING_3
            elif count == 4:
                ttype = TextLineType.HEADING_4
            content = content_stripped[count + 1 :]
    elif content_stripped.startswith("! "):
        ttype = TextLineType.CALLOUT
        content = content_stripped[2:]
    elif content_stripped.startswith("> "):
        ttype = TextLineType.QUOTE
        content = content_stripped[2:]
    elif content_stripped.startswith("- "):
        ttype = TextLineType.LIST_UNORDERED
        content = content_stripped[2:]
    elif regex.match(r"^\d+\.\s", content_stripped):
        ttype = TextLineType.LIST_ORDERED
        content = regex.sub(r"^\d+\.\s", "", content_stripped)
    elif content_stripped == "---" or content_stripped == "--":
        ttype = TextLineType.DIVIDER
        content = ""
    spans = _parse_inline(content, aliasing)
    line_obj = TextLine(type=ttype, spans=spans)
    if line_color:
        line_obj.color = ColorType[line_color]
    return line_obj


def _parse_code(lines: list[str], start: int) -> tuple[TextLine, int]:
    """
    Parse a code line.
    """
    i = start + 1

    # language
    language = None
    opening_line = lines[start].strip()
    language_match = regex.match(r"^```([a-zA-Z0-9+#.]+)?", opening_line)
    if language_match and language_match.group(1):
        language = language_match.group(1).strip()

    # code
    code_lines = []
    while i < len(lines) and not lines[i].startswith("```"):
        code_lines.append(lines[i])
        i += 1
    content = "\n".join(code_lines)
    i += 1  # skip closing ```

    return TextLine(type=TextLineType.CODE, content=content, language=language), i


def markdown_to_text(markdown: str, aliasing: "AliasingIn | None" = None) -> Text:
    """
    Parse markdown as Text.
    """
    from bench.language import Aliasing

    if not markdown:
        return Text.empty()
    if isinstance(aliasing, Mapping):
        aliasing = Aliasing.new(aliasing)

    markdown = textwrap.dedent(markdown)
    lines_str = markdown.splitlines()
    lines: list[TextLine] = []
    i = 0
    while i < len(lines_str):
        line = lines_str[i]
        if line.strip() == "":
            i += 1
            continue
        if line.startswith("```"):
            tl, i = _parse_code(lines_str, i)
            lines.append(tl)
        elif line.lstrip().startswith("> "):
            # join multi-line quote blocks
            quote_content = []
            while i < len(lines_str) and lines_str[i].lstrip().startswith("> "):
                # extract content after the '> ' prefix
                content = lines_str[i].lstrip()[2:]
                quote_content.append(content)
                i += 1
            combined_content = "\n".join(quote_content)
            spans = _parse_inline(combined_content, aliasing)
            lines.append(TextLine(type=TextLineType.QUOTE, spans=spans))
        else:
            tl = markdown_line_to_line(line, aliasing)
            lines.append(tl)
            i += 1
    return Text(lines=lines)


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


def _render_color(color: "ColorType") -> str:
    """
    Render a ColorType as a string.
    """
    return color.name.lower()


def _render_inline_raw(spans: Sequence[TextSpan], aliasing: "Aliasing | None" = None) -> str:
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
                if aliasing is not None:
                    alias = aliasing.get_or_add(node)
                else:
                    alias = node.code_name or "???"
                result.append(f"[@{alias}]")
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


def _render_inline(spans: Sequence[TextSpan], aliasing: "Aliasing | None" = None) -> str:
    """
    Group consecutive spans with the same color so that the outer color marker is rendered only once.
    """
    grouped: list[tuple[ColorType | None, list[TextSpan]]] = []
    current_color = None
    current_group: list[TextSpan] = []
    for span in spans:
        if span.color == current_color:
            current_group.append(span)
        else:
            if current_group:
                grouped.append((current_color, current_group))
            current_color = span.color
            current_group = [span]
    if current_group:
        grouped.append((current_color, current_group))
    parts = []
    for color, group in grouped:
        inner = _render_inline_raw(group, aliasing)
        if color:
            parts.append(f"[{_render_color(color)}]{inner}[/{_render_color(color)}]")
        else:
            parts.append(inner)
    return "".join(parts)


def text_line_to_markdown(line: TextLine, aliasing: "AliasingIn | None" = None) -> str:
    """
    Render a single TextLine as markdown.
    """
    if aliasing is not None and isinstance(aliasing, Mapping):
        from bench.language.source import Aliasing

        aliasing = Aliasing.new(aliasing)
    if line.type == TextLineType.DIVIDER:
        return "---"
    elif line.type == TextLineType.CODE:
        if line.language:
            return f"```{line.language}\n" + (line.content or "") + "\n```"
        else:
            return "```\n" + (line.content or "") + "\n```"
    else:
        prefix = ""
        if line.type == TextLineType.HEADING_1:
            prefix = "# "
        elif line.type == TextLineType.HEADING_2:
            prefix = "## "
        elif line.type == TextLineType.HEADING_3:
            prefix = "### "
        elif line.type == TextLineType.HEADING_4:
            prefix = "#### "
        elif line.type == TextLineType.CALLOUT:
            prefix = "! "
        elif line.type == TextLineType.QUOTE:
            prefix = "> "
        elif line.type == TextLineType.LIST_UNORDERED:
            prefix = "- "
        elif line.type == TextLineType.LIST_ORDERED:
            prefix = "1. "
        content = _render_inline(line.spans, aliasing)

        # emit multi-line quotes by adding "> " prefix to each line
        if line.type == TextLineType.QUOTE and "\n" in content:
            lines = content.split("\n")
            content = "\n".join(f"{prefix}{line}" for line in lines)
            prefix = ""

        if line.color:
            content = f"[{_render_color(line.color)}]{content}[/{_render_color(line.color)}]"
        return prefix + content


def text_to_markdown(text: Text, aliasing: "AliasingIn | None" = None) -> str:
    """
    Render a Text object as markdown.
    """
    return "\n".join(text_line_to_markdown(line, aliasing) for line in text.lines)


TextIn = Text | str
TextLineIn = TextLine | str


def text(text: TextIn, aliasing: "AliasingIn | None" = None) -> Text:
    if isinstance(text, str):
        return markdown_to_text(text, aliasing=aliasing)
    else:
        return text


def text_line(text: TextLineIn, aliasing: "AliasingIn | None" = None) -> TextLine:
    if isinstance(text, str):
        if isinstance(aliasing, Mapping):
            aliasing = Aliasing.new(aliasing)
        return markdown_line_to_line(text, aliasing=aliasing)
    else:
        return text


to_text = text
to_text_line = text_line
