from typing import TYPE_CHECKING, Any, List, Optional
from uuid import UUID

import regex

from .const import BuiltinEnum, EnumType, StructType, enum_
from .node import Node, NodeReference
from .object import BuiltinObject, object_
from .property import p_regular
from .struct import Struct, struct_

if TYPE_CHECKING:
    from bench.language import Alignment, ColorType


@object_()
class TextOptionsBase(BuiltinObject):
    color: Optional["ColorType"] = p_regular(50, default=None)
    background_color: Optional["ColorType"] = p_regular(51, default=None)
    is_bold: Optional[bool] = p_regular(60, default=None)
    is_italic: Optional[bool] = p_regular(61, default=None)
    is_strikethrough: Optional[bool] = p_regular(62, default=None)
    is_underline: Optional[bool] = p_regular(63, default=None)
    is_code: Optional[bool] = p_regular(64, default=None)

    def _to_option_kwargs(self):
        kwargs = {}
        for prop in self.__declared_properties__.values():
            value = getattr(self, prop.name)
            if value is not None:
                kwargs[prop.name] = value
        return kwargs


@enum_(EnumType.TEXT_LINE_TYPE)
class TextLineType(BuiltinEnum):
    # basic
    PARAGRAPH = 1, "plain paragraph"
    # heading
    HEADING_1 = 10, "very big heading"
    HEADING_2 = 11, "big heading"
    HEADING_3 = 12, "medium heading"
    HEADING_4 = 13, "small heading"
    # highlight
    CALLOUT = 20, "callout"
    QUOTE = 21, "quote"
    # list
    LIST_BULLET = 30, "bullet list"
    LIST_NUMBERED = 31, "numbered list"
    LIST_UNCHECKED = 32, "unchecked list"
    LIST_CHECKED = 33, "checked list"
    # divider
    DIVIDER = 40, "horizontal line"
    # table
    TABLE = 50, "table"
    TABLE_ROW = 51, "table row"
    # code
    CODE = 60, "code block"
    EQUATION = 61, "equation (tex)"
    DIAGRAM = 62, "diagram (mermaid)"


@struct_(StructType.TEXT_SPAN)
class TextSpan(TextOptionsBase, Struct):
    """a span of text with optional formatting"""

    content: Optional[str] = p_regular(33, default=None)
    node: Optional[Node] = p_regular(34, array=False, default=None, require=False, references="any")
    if TYPE_CHECKING:
        node_id: Optional[UUID] = None
        node_ck: Optional[UUID] = None
        node_ptr: Optional[NodeReference] = None

    @staticmethod
    def new(
        content: str | Node | None,
        is_bold: bool | None = None,
        is_italic: bool | None = None,
        is_strikethrough: bool | None = None,
        is_underline: bool | None = None,
        is_code: bool | None = None,
        color: "ColorType | None" = None,
    ) -> "TextSpan":
        if isinstance(content, str):
            assert content, "content must be non-empty"
            node = None
        else:
            node = content
            content = None
        return TextSpan(
            content=content,
            node=node,
            is_bold=is_bold,
            is_italic=is_italic,
            is_strikethrough=is_strikethrough,
            is_underline=is_underline,
            is_code=is_code,
            color=color,
        )


@struct_(StructType.TEXT_TABLE)
class TextTable(Struct):
    """
    a table of text cells; header row defines keys; rows are stored as table rows
    """

    lines: List["TextLine"] = p_regular(33, array=True, struct=StructType.TEXT_LINE)
    has_row_header: bool = p_regular(40, default=False)
    has_column_header: bool = p_regular(41, default=False)


@struct_(StructType.TEXT_CELL)
class TextCell(TextOptionsBase, Struct):
    """a cell of text in a table"""

    spans: List[TextSpan] = p_regular(33, array=True, struct=StructType.TEXT_SPAN)
    colspan: int = p_regular(40, default=1)
    rowspan: int = p_regular(41, default=1)
    alignment: Optional["Alignment"] = p_regular(42, default=None)


@struct_(StructType.TEXT_LINE)
class TextLine(TextOptionsBase, Struct):
    """
    a single line of text; may contain inline spans or hold a table or code block
    """

    type: TextLineType = p_regular(30, default=TextLineType.PARAGRAPH)
    spans: List[TextSpan] = p_regular(33, array=True, struct=StructType.TEXT_SPAN)
    content: Optional[str] = p_regular(34, default=None)
    table: Optional[TextTable] = p_regular(35, default=None, struct=StructType.TEXT_TABLE)
    cells: list[TextCell] = p_regular(36, array=True, struct=StructType.TEXT_CELL)

    def __contains__(self, item: str) -> bool:
        if (content := self.content) and item in content:
            return True
        for span in self.spans:
            if (content := span.content) and item in content:
                return True
        if self.table:
            for row in self.table.lines:
                for cell in row.cells:
                    for span in cell.spans:
                        if (content := span.content) and item in content:
                            return True
        return False

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
        return TextLine(type=TextLineType.LIST_BULLET, spans=_parse_inline(text))

    @staticmethod
    def list_numbered(text: str) -> "TextLine":
        return TextLine(type=TextLineType.LIST_NUMBERED, spans=_parse_inline(text))

    @staticmethod
    def list_unchecked(text: str) -> "TextLine":
        return TextLine(type=TextLineType.LIST_UNCHECKED, spans=_parse_inline(text))

    @staticmethod
    def list_checked(text: str) -> "TextLine":
        return TextLine(type=TextLineType.LIST_CHECKED, spans=_parse_inline(text))

    @staticmethod
    def divider() -> "TextLine":
        return TextLine(type=TextLineType.DIVIDER)

    @staticmethod
    def equation(text: str) -> "TextLine":
        return TextLine(type=TextLineType.EQUATION, content=text)

    @staticmethod
    def diagram(text: str) -> "TextLine":
        return TextLine(type=TextLineType.DIAGRAM, content=text)

    @staticmethod
    def new(
        type: TextLineType,
        spans: List[TextSpan] | str | None = None,
        is_bold: bool | None = None,
        is_italic: bool | None = None,
        is_strikethrough: bool | None = None,
        is_underline: bool | None = None,
        is_code: bool | None = None,
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
            is_code=is_code,
            color=color,
        )


@struct_(StructType.TEXT)
class Text(Struct):
    """
    rich text structure; composed of text lines
    """

    lines: List[TextLine] = p_regular(32, array=True, struct=StructType.TEXT_LINE)

    def __contains__(self, item: str) -> bool:
        return any(item in line for line in self.lines)

    def to_markdown(self) -> str:
        return text_to_markdown(self)

    @staticmethod
    def from_markdown(markdown: str) -> "Text":
        return markdown_to_text(markdown)

    to_string = to_markdown

    @staticmethod
    def plain(text: str) -> "Text":
        return Text(lines=[TextLine.paragraph(line) for line in text.splitlines()])

    from_string = plain

    @staticmethod
    def code(code: str) -> "Text":
        return Text(lines=[TextLine.code(code)])

    @staticmethod
    def empty() -> "Text":
        return Text(lines=[])


text = Text.plain
#
# Parsing
#

MARKER_TO_FLAG = {
    "*": "is_italic",
    "_": "is_italic",
    "**": "is_bold",
    "__": "is_bold",
    "~~": "is_strikethrough",
    "`": "is_code",
    "<u>": "is_underline",
}

_pat = regex.compile(r"(\*\*|~~|`|<u>|<\/u>|\*|\[[a-zA-Z]+\]|\[\/[a-zA-Z]+\])")


def _parse_color(color: str) -> "ColorType":
    from bench.language import ColorType

    try:
        return ColorType[color.upper()]
    except KeyError as e:
        raise ValueError(
            f"invalid color: {color} (must be one of {', '.join(ColorType.__members__.keys())})"
        ) from e


def _parse_inlines(
    text: str, pos: int = 0, end_marker: str | None = None, base: dict[str, Any] | None = None
) -> tuple[list[TextSpan], int]:
    if base is None:
        base = {}
    spans: list[TextSpan] = []
    while pos < len(text):
        m = _pat.search(text, pos)
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
        # closing color marker (if unexpected, treat as literal)
        if marker.startswith("[/") and marker.endswith("]"):
            if end_marker is not None and marker == end_marker:
                return spans, pos
            else:
                spans.append(TextSpan(content=marker, **base))
                continue
        # opening color marker, e.g. [red]
        if marker.startswith("[") and marker.endswith("]") and not marker.startswith("[/"):
            color = _parse_color(marker[1:-1])
            closing = f"[/{marker[1:-1]}]"
            inner, pos = _parse_inlines(text, pos, end_marker=closing, base=base.copy())
            for sp in inner:
                sp.color = color
            spans.extend(inner)
            continue
        # <u> marker with explicit closing </u>
        if marker == "<u>":
            inner, pos = _parse_inlines(text, pos, end_marker="</u>", base=base.copy())
            for sp in inner:
                sp.is_underline = True
            spans.extend(inner)
            continue
        # symmetric markers: *, **, ~~ or `
        if marker in {"*", "**", "~~", "`"}:
            flag = MARKER_TO_FLAG[marker]
            inner, pos = _parse_inlines(text, pos, end_marker=marker, base=base.copy())
            for sp in inner:
                setattr(sp, flag, True)
            spans.extend(inner)
            continue
        # unrecognized marker: treat as literal.
        spans.append(TextSpan(content=marker, **base))
    return spans, pos


def _parse_inline(text: str) -> list[TextSpan]:
    spans, _ = _parse_inlines(text, 0, None, {})
    return _merge_spans(spans)


def _merge_spans(spans: list[TextSpan]) -> list[TextSpan]:
    """
    Merge adjacent spans with identical formatting.
    """
    if not spans:
        return spans
    merged = [spans[0]]
    for sp in spans[1:]:
        if _span_format_key(merged[-1]) == _span_format_key(sp):
            merged[-1].content = (merged[-1].content or "") + (sp.content or "")
        else:
            merged.append(sp)
    return merged


def _span_format_key(span: TextSpan) -> tuple:
    return (
        span.is_bold,
        span.is_italic,
        span.is_strikethrough,
        span.is_underline,
        span.is_code,
        span.color,
        span.background_color,
    )


def is_table_start(lines: list[str], idx: int) -> bool:
    if "|" not in lines[idx]:
        return False
    if idx + 1 >= len(lines):
        return False
    sep = lines[idx + 1].strip()
    return regex.search(r"^\s*\|?( *:?-+:? *\|)+ *:?-*:?\|?\s*$", sep) is not None


def _parse_line(line: str) -> TextLine:
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
    elif content_stripped.startswith("- [ ] "):
        ttype = TextLineType.LIST_UNCHECKED
        content = content_stripped[6:]
    elif content_stripped.startswith("- [x] "):
        ttype = TextLineType.LIST_CHECKED
        content = content_stripped[6:]
    elif content_stripped.startswith("- "):
        ttype = TextLineType.LIST_BULLET
        content = content_stripped[2:]
    elif regex.match(r"^\d+\.\s", content_stripped):
        ttype = TextLineType.LIST_NUMBERED
        content = regex.sub(r"^\d+\.\s", "", content_stripped)
    elif content_stripped == "---":
        ttype = TextLineType.DIVIDER
        content = ""
    spans = _parse_inline(content)
    line_obj = TextLine(type=ttype, spans=spans)
    if line_color:
        line_obj.color = ColorType[line_color]
    return line_obj


def _parse_code(lines: list[str], start: int) -> tuple[TextLine, int]:
    first = lines[start].strip()
    lang = first[3:].strip().lower()
    if lang == "tex":
        ttype = TextLineType.EQUATION
    elif lang == "mermaid":
        ttype = TextLineType.DIAGRAM
    else:
        ttype = TextLineType.CODE
    i = start + 1
    code_lines = []
    while i < len(lines) and not lines[i].startswith("```"):
        code_lines.append(lines[i])
        i += 1
    i += 1  # skip closing ```
    content = "\n".join(code_lines)
    return TextLine(type=ttype, content=content), i


def _parse_table(lines: list[str], start: int) -> tuple[TextLine, int]:
    """
    Parse a markdown table.
    """
    from bench.language import Alignment

    header_line = lines[start].strip()
    sep_line = lines[start + 1].strip()
    sep_cells = [cell.strip() for cell in sep_line.strip("|").split("|")]
    alignments: list[Alignment | None] = []
    for cell in sep_cells:
        if cell.startswith(":") and cell.endswith(":"):
            alignments.append(Alignment.MIDDLE)
        elif cell.startswith(":"):
            alignments.append(Alignment.START)
        elif cell.endswith(":"):
            alignments.append(Alignment.END)
        else:
            alignments.append(None)
    header_cells = [cell.strip() for cell in header_line.strip("|").split("|")]
    header_objs: list[TextCell] = []
    for j, txt in enumerate(header_cells):
        spans = _parse_inline(txt)
        alignment = alignments[j] if j < len(alignments) else None
        header_objs.append(TextCell(spans=spans, alignment=alignment))
    body_rows: list[TextLine] = []
    i = start + 2
    while i < len(lines) and "|" in lines[i]:
        row_text = lines[i].strip()
        row_cells_text = [cell.strip() for cell in row_text.strip("|").split("|")]
        if len(row_cells_text) < len(alignments):
            row_cells_text += [""] * (len(alignments) - len(row_cells_text))
        row_cells: list[TextCell] = []
        for j, txt in enumerate(row_cells_text):
            spans = _parse_inline(txt)
            alignment = alignments[j] if j < len(alignments) else None
            row_cells.append(TextCell(spans=spans, alignment=alignment))
        body_rows.append(TextLine(type=TextLineType.TABLE_ROW, cells=row_cells))
        i += 1
    table = TextTable(
        lines=[TextLine(type=TextLineType.TABLE_ROW, cells=header_objs), *body_rows],
        has_row_header=True,
        has_column_header=False,
    )
    return TextLine(type=TextLineType.TABLE, table=table), i


def markdown_to_text(markdown: str) -> Text:
    """
    Parse markdown into a Text object.
    """
    if not markdown:
        return Text.empty()
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
        elif is_table_start(lines_str, i):
            tl, i = _parse_table(lines_str, i)
            lines.append(tl)
        else:
            tl = _parse_line(line)
            lines.append(tl)
            i += 1
    return Text(lines=lines)


#
# Rendering
#


def _render_color(color: "ColorType") -> str:
    return color.name.lower()


def _render_span_no_color(span: TextSpan) -> str:
    txt = span.content or ""
    if span.is_code:
        txt = f"`{txt}`"
    if span.is_underline:
        txt = f"<u>{txt}</u>"
    if span.is_bold:
        txt = f"**{txt}**"
    if span.is_italic:
        txt = f"*{txt}*"
    if span.is_strikethrough:
        txt = f"~~{txt}~~"
    return txt


def _render_inline(spans: list[TextSpan]) -> str:
    """
    Group consecutive spans with the same color so that the outer color marker is rendered once.
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
        inner = _render_formatted_spans(group)
        if color:
            parts.append(f"[{_render_color(color)}]{inner}[/{_render_color(color)}]")
        else:
            parts.append(inner)
    return "".join(parts)


MARKER_ORDER = ["is_italic", "is_bold", "is_strikethrough", "is_underline"]
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


def _render_formatted_spans(spans: list[TextSpan]) -> str:
    """
    Render a list of TextSpan objects with inline markdown formatting.
    This function computes formatting state transitions between spans so that
    nested formatting markers (e.g. *…~~…~~…*) are rendered correctly.
    """
    # fixed order for non-code markers

    def _get_options(span: TextSpan) -> list[str]:
        # code spans ignore other formatting
        if span.is_code:
            return ["is_code"]
        # using list comprehension is efficient enough here
        return [flag for flag in MARKER_ORDER if getattr(span, flag)]

    result = []
    current_state: list[str] = []
    for span in spans:
        new_state = _get_options(span)
        # compute common prefix length
        min_len = min(len(new_state), len(current_state))
        common = 0
        while common < min_len and current_state[common] == new_state[common]:
            common += 1
        # close markers that are no longer active
        for flag in reversed(current_state[common:]):
            result.append(MARKER_CLOSE[flag])
        # open new markers
        for flag in new_state[common:]:
            result.append(MARKER_OPEN[flag])
        result.append(span.content or "")
        current_state = new_state
    # close any markers still open
    for flag in reversed(current_state):
        result.append(MARKER_CLOSE[flag])
    return "".join(result)


def _render_table(table: TextTable) -> str:
    from bench.language import Alignment

    if not table.lines:
        return ""
    header_line = table.lines[0]
    if not header_line.cells:
        return ""
    header_md = "| " + " | ".join(_render_inline(cell.spans) for cell in header_line.cells) + " |"
    sep_parts = []
    for cell in header_line.cells:
        if cell.alignment == Alignment.START:
            sep_parts.append(":---")
        elif cell.alignment == Alignment.END:
            sep_parts.append("---:")
        elif cell.alignment == Alignment.MIDDLE:
            sep_parts.append(":---:")
        else:
            sep_parts.append("---")
    sep_md = "| " + " | ".join(sep_parts) + " |"
    rows_md = [header_md, sep_md]
    for row in table.lines[1:]:
        if not row.cells:
            continue
        row_md = "| " + " | ".join(_render_inline(cell.spans) for cell in row.cells) + " |"
        rows_md.append(row_md)
    return "\n".join(rows_md)


def text_to_markdown(text_obj: Text) -> str:
    """
    Render a Text object as markdown.
    """
    md_lines = []
    for line in text_obj.lines:
        if line.type == TextLineType.DIVIDER:
            md_lines.append("---")
        elif line.type in (TextLineType.CODE, TextLineType.EQUATION, TextLineType.DIAGRAM):
            lang = ""
            if line.type == TextLineType.EQUATION:
                lang = "tex"
            elif line.type == TextLineType.DIAGRAM:
                lang = "mermaid"
            md_lines.append(f"```{lang}".rstrip())  # noqa: FURB113
            md_lines.append(line.content or "")
            md_lines.append("```")
        elif line.type == TextLineType.TABLE and line.table:
            md_lines.append(_render_table(line.table))
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
            elif line.type == TextLineType.LIST_UNCHECKED:
                prefix = "- [ ] "
            elif line.type == TextLineType.LIST_CHECKED:
                prefix = "- [x] "
            elif line.type == TextLineType.LIST_BULLET:
                prefix = "- "
            elif line.type == TextLineType.LIST_NUMBERED:
                prefix = "1. "
            content = _render_inline(line.spans)
            if line.color:
                content = f"[{_render_color(line.color)}]{content}[/{_render_color(line.color)}]"
            md_lines.append(prefix + content)
    return "\n".join(md_lines)
