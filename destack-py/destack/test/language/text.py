import pytest

from destack.language import (
    Aliasing,
    Folder,
    Session,
    Space,
    Text,
    TextSpan,
    TextSpanType,
    markdown_to_text,
    text,
    text_to_markdown,
)


def test_text_mentions(session: Session, space: Space):
    Folder1 = Folder(name="Folder1")
    Folder2 = Folder(name="Folder2")
    aliasing = Aliasing.new(session.supergraph, {"Folder1": Folder1, "Folder2": Folder2})
    my_text = text(
        "Hello it's a [@Folder1] and [@Folder2]",
        aliasing,
    )
    assert my_text.spans[0] == TextSpan(type=TextSpanType.TEXT, content="Hello it's a ")
    assert my_text.spans[1] == TextSpan(type=TextSpanType.MENTION, content="Folder1", node=Folder1)
    assert my_text.spans[2] == TextSpan(type=TextSpanType.TEXT, content=" and ")
    assert my_text.spans[3] == TextSpan(type=TextSpanType.MENTION, content="Folder2", node=Folder2)
    assert text_to_markdown(my_text, aliasing) == "Hello it's a [@Folder1] and [@Folder2]"


def test_text_citation():
    md = "This is a deferred citation[^1] and an inline citation[^Symbol25](www.symbol.com)."
    text_obj = markdown_to_text(md)
    assert text_obj.spans[0] == TextSpan(
        type=TextSpanType.TEXT, content="This is a deferred citation"
    )
    assert text_obj.spans[1] == TextSpan(type=TextSpanType.CITATION, content="1")
    assert text_obj.spans[2] == TextSpan(type=TextSpanType.TEXT, content=" and an inline citation")
    assert text_obj.spans[3] == TextSpan(
        type=TextSpanType.CITATION, content="Symbol25", url="www.symbol.com"
    )


def test_text_code():
    md = "this is my `code`"
    text_obj = markdown_to_text(md)
    md_out = text_to_markdown(text_obj)
    assert md_out == md


def test_text_span_equation():
    md = "And then he said $$E = mc^2$$ and $$F = ma$$"
    text_obj = markdown_to_text(md)
    assert text_obj.spans[0] == TextSpan(type=TextSpanType.TEXT, content="And then he said ")
    assert text_obj.spans[1] == TextSpan(type=TextSpanType.EQUATION, content="E = mc^2")
    assert text_obj.spans[2] == TextSpan(type=TextSpanType.TEXT, content=" and ")
    assert text_obj.spans[3] == TextSpan(type=TextSpanType.EQUATION, content="F = ma")


def test_text_span_link():
    md = "[link](https://example.com) and [another](https://test.com/path?q=123#fragment) [Banana - Wikipedia](https://en.wikipedia.org/wiki/Banana) https://de.wikipedia.org/wiki/Switzerland like google.com"
    text_obj = markdown_to_text(md)
    assert text_obj.spans[0] == TextSpan(
        type=TextSpanType.LINK, content="link", url="https://example.com"
    )
    assert text_obj.spans[1] == TextSpan(type=TextSpanType.TEXT, content=" and ")
    assert text_obj.spans[2] == TextSpan(
        type=TextSpanType.LINK, content="another", url="https://test.com/path?q=123#fragment"
    )
    assert text_obj.spans[3] == TextSpan(type=TextSpanType.TEXT, content=" ")
    assert text_obj.spans[4] == TextSpan(
        type=TextSpanType.LINK,
        content="Banana - Wikipedia",
        url="https://en.wikipedia.org/wiki/Banana",
    )
    assert text_obj.spans[5] == TextSpan(type=TextSpanType.TEXT, content=" ")
    assert text_obj.spans[6] == TextSpan(
        type=TextSpanType.LINK,
        content="https://de.wikipedia.org/wiki/Switzerland",
        url="https://de.wikipedia.org/wiki/Switzerland",
    )
    assert text_obj.spans[7] == TextSpan(type=TextSpanType.TEXT, content=" like ")
    assert text_obj.spans[8] == TextSpan(
        type=TextSpanType.LINK, content="google.com", url="https://google.com"
    )


def test_text_hard_break():
    md = "This is a hard break<br>This is another line"
    text_obj = markdown_to_text(md)
    assert text_obj.spans[0] == TextSpan(type=TextSpanType.TEXT, content="This is a hard break")
    assert text_obj.spans[1] == TextSpan(type=TextSpanType.HARD_BREAK)
    assert text_obj.spans[2] == TextSpan(type=TextSpanType.TEXT, content="This is another line")


def test_text_inline_nested():
    md = "this is **bold** and *italic* and ~~strike~~ and `code`."
    text_obj = markdown_to_text(md)
    md_out = text_to_markdown(text_obj)
    assert md_out == md


@pytest.mark.parametrize(
    "md",
    [
        "Here is my $$E = mc^2$$ equation, check out [this link](https://example.com) and [another one](https://test.com/path?q=123#fragment)",
        "Normal text with `inline code` and ~~strikethrough~~.",
        "This text has **multiple** *different* ~~formatting~~.",
        "Basic math: 4 * 7 = 28 and 10 * 3 = 30",
        "This is *italic*, but 2 * 3 is multiplication, and this is also *italic*.",
    ],
)
def test_text_roundtrip(md):
    text_obj = markdown_to_text(md)
    text_rendered = text_to_markdown(text_obj)
    # should equal original markdown
    assert text_rendered == md
    # should roundtrip
    text_reparsed = markdown_to_text(text_rendered)
    assert text_to_markdown(text_reparsed) == text_to_markdown(text_obj)


def test_invalid_bracket_content():
    """Test that invalid content in brackets like [PDF] is treated as literal text."""
    md = "This document is available as [PDF] and [EPUB] formats."
    text_obj = markdown_to_text(md)

    # Check that [PDF] and [EPUB] are preserved as literal text
    combined_content = "".join(span.content or "" for span in text_obj.spans)
    assert "[PDF]" in combined_content
    assert "[EPUB]" in combined_content

    # Verify roundtrip
    md_out = text_to_markdown(text_obj)
    assert "This document is available as [PDF] and [EPUB] formats." in md_out


def test_math_operators():
    """Test that math operators like * in '4 * 7 = 24' are not treated as formatting markers."""
    md = "Basic math: 4 * 7 = 28 and 10 * 3 = 30"
    text_obj = markdown_to_text(md)

    # Verify that asterisks are preserved as literal text, not formatting
    combined_content = "".join(span.content or "" for span in text_obj.spans)
    assert "4 * 7 = 28" in combined_content
    assert "10 * 3 = 30" in combined_content

    # Check none of the spans have italic formatting
    assert all(not span.is_italic for span in text_obj.spans)

    # Verify roundtrip
    md_out = text_to_markdown(text_obj)
    assert md_out == md


def test_multiline_becomes_single_paragraph():
    """Test that multi-line input becomes a single paragraph."""
    md = """This is line one
This is line two
This is line three"""
    text_obj = markdown_to_text(md)

    # Should be collapsed into single paragraph
    combined_content = "".join(span.content or "" for span in text_obj.spans)
    assert combined_content == "This is line one This is line two This is line three"


def test_empty_text():
    """Test empty text handling."""
    text_obj = Text.empty()
    assert text_obj.is_empty
    assert text_obj.to_plain() == ""
    assert text_obj.to_markdown() == ""


def test_plain_text_creation():
    """Test creating plain text without markdown."""
    text_obj = Text.plain_text("Hello world")
    assert len(text_obj.spans) == 1
    assert text_obj.spans[0].content == "Hello world"
    assert text_obj.spans[0].type == TextSpanType.TEXT
