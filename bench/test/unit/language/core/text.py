import pytest

from bench.language import (
    Page,
    Session,
    TextLine,
    TextLineType,
    TextSpan,
    TextSpanType,
    markdown_to_text,
    text,
    text_to_markdown,
)


def test_text_mentions(session: Session):
    Page1 = Page.new("Page1")
    Page2 = Page.new("Page2")
    my_text = text(
        "Hello it's a [@Page1] and [@Page2]",
        {"Page1": Page1, "Page2": Page2},
    )
    assert my_text.lines[0].spans[0] == TextSpan.new(TextSpanType.TEXT, "Hello it's a ")
    assert my_text.lines[0].spans[1] == TextSpan.new(TextSpanType.MENTION, "Page1", node=Page1)
    assert my_text.lines[0].spans[2] == TextSpan.new(TextSpanType.TEXT, " and ")
    assert my_text.lines[0].spans[3] == TextSpan.new(TextSpanType.MENTION, "Page2", node=Page2)
    assert (
        text_to_markdown(my_text, {"Page1": Page1, "Page2": Page2})
        == "Hello it's a [@Page1] and [@Page2]"
    )


def test_text_prefix():
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


def test_text_citation():
    md = """\
This is a deferred citation[^1] and an inline citation[^Symbol25](www.symbol.com).
"""
    text = markdown_to_text(md)
    assert text.lines[0].spans[0] == TextSpan.new(TextSpanType.TEXT, "This is a deferred citation")
    assert text.lines[0].spans[1] == TextSpan.new(TextSpanType.CITATION, "1")
    assert text.lines[0].spans[2] == TextSpan.new(TextSpanType.TEXT, " and an inline citation")
    assert text.lines[0].spans[3] == TextSpan.new(
        TextSpanType.CITATION, "Symbol25", url="www.symbol.com"
    )


def test_text_code():
    md = "this is my `code`"
    text = markdown_to_text(md)
    md_out = text_to_markdown(text)
    assert md_out == md


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
        TextSpanType.LINK, content="link", url="https://example.com"
    )
    assert text.lines[0].spans[1] == TextSpan.new(TextSpanType.TEXT, " and ")
    assert text.lines[0].spans[2] == TextSpan.new(
        TextSpanType.LINK, content="another", url="https://test.com/path?q=123#fragment"
    )


def test_text_hard_break():
    md = """\
This is a hard break<br>This is another line"""
    text = markdown_to_text(md)
    assert text.lines[0].spans[0] == TextSpan.new(TextSpanType.TEXT, "This is a hard break")
    assert text.lines[0].spans[1] == TextSpan.hard_break()
    assert text.lines[0].spans[2] == TextSpan.new(TextSpanType.TEXT, "This is another line")


def test_text_inline_nested():
    md = """\
this is **bold** and *italic* and ~~strike~~ and `code` and <u>underline</u>.
also we have [red]red[/red] and [blue]blue[/blue] and [green]green[/green].
they [yellow]can be *nested, like ~~deeply~~ nested* _and_ ~~combined~~[/yellow]."""
    text = markdown_to_text(md)
    md_out = text_to_markdown(text)
    assert md_out == md


def test_text_code_block():
    md = """\
```
print('hello')
line2
```
"""
    text = markdown_to_text(md)
    line = text.lines[0]
    assert line.language is None
    assert line.type == TextLineType.CODE
    assert "print('hello')" in (line.content or "")
    md_out = text_to_markdown(text)
    assert "```" in md_out
    assert "print('hello')" in md_out


def test_text_code_block_language():
    md = """\
A FizzBuzz implementation in Rust:
```rust
fn main() {
    for i in 1..=100 {
        if i % 15 == 0 {
            println!("FizzBuzz");
        } else if i % 3 == 0 {
            println!("Fizz");
        } else if i % 5 == 0 {
            println!("Buzz");
        } else {
            println!("{}", i);
        }
    }
}
```"""
    text = markdown_to_text(md)
    line = text.lines[1]
    assert line.language == "rust"
    assert line.type == TextLineType.CODE
    assert text_to_markdown(text) == md


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
> This is a blockquote with <u>underlined text</u>[^2]
! Important callout message""",
        """\
```
def complex_function():
    # This is a comment
    result = [x for x in range(10) if x % 2 == 0]
    print(f"Even numbers: {result}")
```""",
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
