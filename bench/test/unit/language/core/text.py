import pytest

from bench.language import TextLineType, markdown_to_text, text_to_markdown
from bench.language.core.text import TextLine, TextSpan, TextSpanType


def test_text_heading_and_list():
    md = (
        "# Heading 1\n"
        "## Heading 2\n"
        "! callout text\n"
        "> quoted text\n"
        "- list item\n"
        "1. numbered item\n"
        "---"
    )
    text = markdown_to_text(md)
    assert text.lines[0] == TextLine.heading(1, "Heading 1")
    assert text.lines[1] == TextLine.heading(2, "Heading 2")
    assert text.lines[2] == TextLine.callout("callout text")
    assert text.lines[3] == TextLine.quote("quoted text")
    assert text.lines[4] == TextLine.list_bullet("list item")
    assert text.lines[5] == TextLine.list_numbered("numbered item")
    assert text.lines[6] == TextLine.divider()


def test_text_span_equation():
    md = """\
And then he said $$E = mc^2$$ and $$F = ma$$"""
    text = markdown_to_text(md)
    assert text.lines[0].spans[0] == TextSpan.new(TextSpanType.TEXT, "And then he said ")
    assert text.lines[0].spans[1] == TextSpan.new(TextSpanType.EQUATION, "E = mc^2")
    assert text.lines[0].spans[2] == TextSpan.new(TextSpanType.TEXT, " and ")
    assert text.lines[0].spans[3] == TextSpan.new(TextSpanType.EQUATION, "F = ma")


def test_text_span_link():
    md = """\
[link](https://example.com) and [another](https://test.com/path?q=123#fragment)"""
    text = markdown_to_text(md)
    assert text.lines[0].spans[0] == TextSpan.new(
        TextSpanType.LINK, content="link", href="https://example.com"
    )
    assert text.lines[0].spans[1] == TextSpan.new(TextSpanType.TEXT, " and ")
    assert text.lines[0].spans[2] == TextSpan.new(
        TextSpanType.LINK, content="another", href="https://test.com/path?q=123#fragment"
    )


def test_text_hard_break():
    md = """\
This is a hard break<br>This is another line"""
    text = markdown_to_text(md)
    assert text.lines[0].spans[0] == TextSpan.new(TextSpanType.TEXT, "This is a hard break")
    assert text.lines[0].spans[1] == TextSpan.hard_break()
    assert text.lines[0].spans[2] == TextSpan.new(TextSpanType.TEXT, "This is another line")


def test_text_inline_formatting():
    md = """\
this is **bold** and *italic* and ~~strike~~ and `code` and <u>underline</u>.
also we have [red]red[/red] and [blue]blue[/blue] and [green]green[/green].
they [yellow]can be *nested, like ~~deeply~~ nested* _and_ ~~combined~~[/yellow]."""
    text = markdown_to_text(md)
    md_out = text_to_markdown(text)
    assert md_out == md


def test_text_code_block():
    md = """\
```python
print('hello')
line2
```
"""
    text = markdown_to_text(md)
    line = text.lines[0]
    assert line.type == TextLineType.CODE
    assert "print('hello')" in (line.content or "")
    md_out = text_to_markdown(text)
    assert "```" in md_out
    assert "print('hello')" in md_out


def test_text_equation_block():
    md = """\
```tex
E = mc^2
```
"""
    text = markdown_to_text(md)
    assert text.lines[0].type == TextLineType.EQUATION
    assert text.lines[0].content == "E = mc^2"
    md_out = text_to_markdown(text)
    assert "```tex" in md_out
    assert "E = mc^2" in md_out


def test_text_diagram_block():
    md = """\
```mermaid
graph TD;
A-->B;
```
"""
    text = markdown_to_text(md)
    assert text.lines[0].type == TextLineType.DIAGRAM
    assert text.lines[0].content == "graph TD;\nA-->B;"
    md_out = text_to_markdown(text)
    assert "```mermaid" in md_out
    assert "graph TD;" in md_out


def test_text_table():
    md = """\
| Product | Price | Stock | Description |
| --- | ---: | :---: | :--- |
| iPhone 13 | $999.99 | 50 | Latest model with A15 chip |
| AirPods Pro | $249.99 | 100 | Active *noise* cancellation |
| MacBook Air | $1299.99 | 25 | M1 chip, 13" display |
    """.strip()
    text = markdown_to_text(md)
    table_line = text.lines[0]
    assert table_line.type == TextLineType.TABLE
    table = table_line.table
    assert table is not None
    md_out = text_to_markdown(text)
    assert md_out == md


@pytest.mark.parametrize(
    "md",
    [
        """\
Here is my $$E = mc^2$$ equation, check out [this link](https://example.com) and [another one](https://test.com/path?q=123#fragment)""",
        """\
# Project Overview
## Key Features
This is a paragraph with **bold** and *italic* formatting""",
        """\
Normal text with `inline code` and ~~strikethrough~~.
> This is a blockquote with <u>underlined text</u>
! Important callout message""",
        """\
```
def complex_function():
    # This is a comment
    result = [x for x in range(10) if x % 2 == 0]
    print(f"Even numbers: {result}")
```""",
        """\
| Language | Paradigm | Year | Creator | Color |
| --- | ---: | :---: | :--- | --- |
| Python | Multi-paradigm | 1991 | Guido `van` *Rossum* | [blue]blue[/blue] |
| Rust | Systems | 2010 | Graydon Hoare | [red]red[/red] |""",
        """\
This text has **multiple** *different* ~~formatting~~ <u>styles</u>
And **spans multiple** lines with *consistent* formatting""",
    ],
)
def test_text_roundtrip(md):
    text = markdown_to_text(md)
    text_rendered = text_to_markdown(text)
    # should equal original markdown
    assert text_rendered == md
    # should roundtrip
    text_reparsed = markdown_to_text(text_rendered)
    assert text_to_markdown(text_reparsed) == text_to_markdown(text)
