from base64 import b64encode
from typing import TYPE_CHECKING, Optional
from uuid import UUID

import regex

from bench.utils.func import IdEnum

from .const import EnumType, NodeType, StructType, enum_
from .node import (
    BenchNode,
    BuiltinObject,
    Node,
    NodeReference,
    Struct,
    object_,
    struct_,
)
from .property import p_regular

if TYPE_CHECKING:
    from bench.language import ColorType


@object_()
class TextOptionsBase(BuiltinObject):
    # color?
    color: Optional["ColorType"] = p_regular(50, default=None)
    # flags
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
class TextLineType(IdEnum):
    PLAIN = 1
    # heading
    HEADING_1 = 10
    HEADING_2 = 11
    HEADING_3 = 12
    HEADING_4 = 13
    # callout
    CALLOUT = 20
    QUOTE = 21
    # list
    LIST_BULLET = 30
    LIST_NUMBERED = 31
    LIST_UNCHECKED = 32
    LIST_CHECKED = 33
    # divider
    DIVIDER = 40
    # table
    # ...
    # equation
    # ...


@struct_(StructType.TEXT_SPAN)
class TextSpan(TextOptionsBase, Struct):
    """A span of text with optional formatting."""

    content: Optional[str] = p_regular(33, default=None)
    node: Optional[Node] = p_regular(34, array=False, default=None, require=False, references="any")
    if TYPE_CHECKING:
        node_ptr: Optional[NodeReference] = None

    def __content_str__(self):
        if self.content:
            return f"'{_md_wrap_text_options(self.content, self)}'"
        elif self.node:
            return f"@{self.node!r}"
        else:
            return ""

    def __len__(self):
        if self.content:
            return len(self.content)
        elif self.node_ptr:
            return 1
        else:
            return 0

    def __contains__(self, other: str) -> bool:
        if self.content is not None:
            return other in self.content
        else:
            return False

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


@struct_(StructType.TEXT_LINE)
class TextLine(TextOptionsBase, Struct):
    """
    A single line of Text with formatting, composed of spans.
    A line may contain hard breaks, so it's effectively a paragraph.
    """

    type: TextLineType = p_regular(30, default=TextLineType.PLAIN)
    spans: list["TextSpan"] = p_regular(33, array=True, struct=StructType.TEXT_SPAN)

    def __content_str__(self):
        if self.type == TextLineType.DIVIDER:
            return "---"
        md_line = "".join(span.__content_str__() for span in self.spans)
        md_line = md_line.replace("\n", "<br>")  # hard breaks
        return f"{_MD_PREFIX_BY_LINE_TYPE.get(self.type, '')}{_md_wrap_text_options(md_line, self)}"

    def __len__(self):
        return sum(len(span) for span in self.spans)

    def __contains__(self, other: str) -> bool:
        return any(other in span for span in self.spans)

    @staticmethod
    def plain(text: str) -> "TextLine":
        if text:
            return TextLine(type=TextLineType.PLAIN, spans=[TextSpan(content=text)])
        else:
            return TextLine(type=TextLineType.PLAIN, spans=[])

    @staticmethod
    def code(text: str) -> "TextLine":
        # NOTE :CrummyMarkdown: we don't support proper code blocks yet
        if text:
            return TextLine(type=TextLineType.PLAIN, spans=[TextSpan(content=text, is_code=True)])
        else:
            return TextLine(type=TextLineType.PLAIN, spans=[])

    @staticmethod
    def new(
        type: TextLineType,
        spans: list["TextSpan"] | str | None = None,
        is_bold: bool | None = None,
        is_italic: bool | None = None,
        is_strikethrough: bool | None = None,
        is_underline: bool | None = None,
        is_code: bool | None = None,
        color: "ColorType | None" = None,
    ) -> "TextLine":
        if spans is None:
            spans = []
        elif isinstance(spans, str):
            spans = [TextSpan(content=spans)]
        return TextLine(
            type=type,
            spans=spans,
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
    Rich, markdown-inspired Text with mentions, lists & other extensions.
    Text is structured into lines, which contain spans.
    Formatting can be applied at per Text, line and span.
    """

    lines: list["TextLine"] = p_regular(32, array=True, struct=StructType.TEXT_LINE)

    def __content_str__(self):
        return "\n".join(line.__content_str__() for line in self.lines)

    def to_markdown(self):
        return text_to_markdown(self)

    to_string = to_markdown

    def __len__(self):
        return sum(len(line) for line in self.lines)

    def __contains__(self, other: str) -> bool:
        return any(other in line for line in self.lines)

    @staticmethod
    def plain(text: str) -> "Text":
        return Text(lines=[TextLine.plain(line) for line in text.splitlines(keepends=False)])

    from_string = plain

    @staticmethod
    def from_markdown(markdown: str) -> "Text":
        return markdown_to_text(markdown)

    @staticmethod
    def code(code: str) -> "Text":
        return Text(lines=[TextLine.code(line) for line in code.splitlines(keepends=False)])

    @staticmethod
    def empty() -> "Text":
        return Text(lines=[])


text = Text.plain
md = Text.from_markdown


def to_text(text: Text | str) -> Text:
    if isinstance(text, str):
        return Text.from_markdown(text)
    else:
        return text


_MD_PREFIX_BY_LINE_TYPE: dict[TextLineType, str] = {
    # in order of precedence
    TextLineType.HEADING_1: "# ",
    TextLineType.HEADING_2: "## ",
    TextLineType.HEADING_3: "### ",
    TextLineType.HEADING_4: "#### ",
    TextLineType.CALLOUT: "! ",
    TextLineType.QUOTE: "> ",
    TextLineType.LIST_NUMBERED: "1. ",
    TextLineType.LIST_UNCHECKED: "- [ ] ",
    TextLineType.LIST_CHECKED: "- [x] ",
    TextLineType.LIST_BULLET: "- ",  # least precedence
}
_LINE_TYPE_BY_MD_PREFIX: dict[str, TextLineType] = {
    v: k for k, v in _MD_PREFIX_BY_LINE_TYPE.items()
}


def _mention_to_url(node: Node) -> str:
    node_ref = node.to_ref()
    url_parts = []
    for key in NodeReference.__declared_properties__:
        value = getattr(node_ref, key)
        if value is not None:
            if type(value) is NodeType:
                value = value.value
            elif type(value) is UUID:
                value = b64encode(value.bytes).decode("ascii")
            else:
                raise ValueError(f"unexpected value type: {value!r} ({type(value).__name__})")
            url_parts.append(f"{key}={value}")
    if isinstance(node, BenchNode) and node.bench:
        return f"bench://{node.bench.slug}/node?{'&'.join(url_parts)}"
    else:
        return f"bench://node?{'&'.join(url_parts)}"


def _md_wrap_text_options(content: str, options: TextOptionsBase) -> str:
    if options.is_bold:
        content = f"**{content}**"
    if options.is_italic:
        content = f"*{content}*"
    if options.is_strikethrough:
        content = f"~~{content}~~"
    if options.is_underline:
        content = f"<u>{content}</u>"
    if options.is_code:
        content = f"`{content}`"
    return content


def text_to_markdown(text: Text) -> str:
    """
    Convert our rich Text into markdown (or GH-flavored markdown), as losslessly as possible.
    Mentions are converted into links with the absolute path as label an pointers info in the link.
    """
    md_lines = []
    for line in text.lines:
        # special case: divider is just one thing
        if line.type == TextLineType.DIVIDER:
            md_lines.append("---")
            continue

        # map spans
        md_line_parts = []
        for span in line.spans:
            if span.node:
                md_line_parts.append(f"[{span.node.absolute_path}]({_mention_to_url(span.node)}")
            elif span.content:
                md_line_parts.append(_md_wrap_text_options(span.content, span))

        # map line
        md_line = "".join(md_line_parts)
        md_line = md_line.replace("\n", "<br>")  # hard breaks
        md_line = (
            f"{_MD_PREFIX_BY_LINE_TYPE.get(line.type, '')}{_md_wrap_text_options(md_line, line)}"
        )
        md_lines.append(md_line)
    return "\n".join(md_lines)


# in order of precedence
_MARK_TOKENS = {
    "is_bold": ("**", "__"),
    "is_italic": ("*", "_"),
    "is_strikethrough": ("~~",),
    "is_underline": (
        "<u>",
        "</u>",
    ),  # NOTE: we assume valid HTML (treating <u> and </u> as one token)
    "is_code": ("`",),
}
_MENTION_REGEX = r"\[([^\]]+)\]\((bench://[^)]+)\)"
_SPLIT_SPAN_REGEX = regex.compile(
    "|".join(f"(?P<{k}>{'|'.join(map(regex.escape, v))})" for k, v in _MARK_TOKENS.items())
    + f"|(?P<mention>{_MENTION_REGEX})"
)


def _split_parse_spans(md_line: str) -> list[TextSpan]:
    """
    'Recursively' parse and split markdown line into spans.
    This is a single line, so we only need to parse out marks & mentions.
    TODO :Robustness: improve markdown parsing :CrummyMarkdown
    """
    current_options = {}
    cur_pos = 0
    spans = []

    # we accumulate marks until we find content (or mention)
    for match in _SPLIT_SPAN_REGEX.finditer(md_line):
        # add previous span
        if match.start() > cur_pos:
            content = md_line[cur_pos : match.start()]
            spans.append(TextSpan(content=content, **current_options))

        # check match (mark or mention)
        for k, v in match.groupdict().items():
            if v is not None:
                if k == "mention":
                    # parse mention
                    raise NotImplementedError(":Incomplete parse mentions")
                else:
                    # toggle option
                    if current_options.get(k, False):
                        current_options.pop(k)
                    else:
                        current_options[k] = True

        # next
        cur_pos = match.end()

    # add last span
    if cur_pos < len(md_line):
        content = md_line[cur_pos:]
        spans.append(TextSpan(content=content, **current_options))

    return spans


def markdown_to_text(markdown: str) -> Text:
    """
    Convert markdown (or GH-flavored markdown) to our rich Text.
    """
    if not markdown:
        return Text.empty()
    md_lines = markdown.splitlines()
    text_lines: list[TextLine] = []
    for md_line in md_lines:
        md_line = md_line.strip()
        # figure out line type
        if md_line == "---":
            text_lines.append(TextLine(type=TextLineType.DIVIDER))
            continue
        for prefix, line_type in _LINE_TYPE_BY_MD_PREFIX.items():
            if md_line.startswith(prefix):
                line_type = line_type
                md_line = md_line[len(prefix) :]  # remove prefix
                break
        else:
            line_type = TextLineType.PLAIN
        # restore hard breaks
        md_line = md_line.replace("<br>", "\n")
        # TODO :Incomplete: eat outermost marks in markdown parse
        line = TextLine(type=line_type)
        line.spans = _split_parse_spans(md_line)
        text_lines.append(line)
    return Text(lines=text_lines)
