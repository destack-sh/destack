import pytest

from bench.language.text import (
    Text,
    TextLine,
    TextLineType,
    TextSpan,
    markdown_to_text,
)


@pytest.mark.parametrize(
    "markdown,lines",
    [
        ("", []),
        ("---", [TextLine.new(TextLineType.DIVIDER)]),
        (
            "## Hello",
            [TextLine.new(TextLineType.HEADING_MEDIUM, "Hello")],
        ),
        (
            """\
---
# Heading Large
## Heading Medium
### Heading Small

! Callout
> Quote

- List Bullet
1. List Numbered
- [ ] List Unchecked
- [x] List Checked""",
            [
                TextLine.new(TextLineType.DIVIDER),
                TextLine.new(TextLineType.HEADING_LARGE, "Heading Large"),
                TextLine.new(TextLineType.HEADING_MEDIUM, "Heading Medium"),
                TextLine.new(TextLineType.HEADING_SMALL, "Heading Small"),
                TextLine.new(TextLineType.PLAIN, []),
                TextLine.new(TextLineType.CALLOUT, "Callout"),
                TextLine.new(TextLineType.QUOTE, "Quote"),
                TextLine.new(TextLineType.PLAIN, []),
                TextLine.new(TextLineType.LIST_BULLET, "List Bullet"),
                TextLine.new(TextLineType.LIST_NUMBERED, "List Numbered"),
                TextLine.new(TextLineType.LIST_UNCHECKED, "List Unchecked"),
                TextLine.new(TextLineType.LIST_CHECKED, "List Checked"),
            ],
        ),
        (
            """\
- Hey!: **Bold** and *Italic* and ~~Strikethrough~~
 ! **Bold and *Italic*** and *Italic and **Bold***
## <u>Underline</u> and `Code` too""",
            [
                TextLine.new(
                    TextLineType.LIST_BULLET,
                    [
                        TextSpan.new("Hey!: "),
                        TextSpan.new("Bold", is_bold=True),
                        TextSpan.new(" and "),
                        TextSpan.new("Italic", is_italic=True),
                        TextSpan.new(" and "),
                        TextSpan.new("Strikethrough", is_strikethrough=True),
                    ],
                ),
                TextLine.new(
                    TextLineType.CALLOUT,
                    [
                        TextSpan.new("Bold and ", is_bold=True),
                        TextSpan.new("Italic", is_bold=True, is_italic=True),
                        TextSpan.new(" and "),
                        TextSpan.new("Italic and ", is_italic=True),
                        TextSpan.new("Bold", is_bold=True, is_italic=True),
                    ],
                ),
                TextLine.new(
                    TextLineType.HEADING_MEDIUM,
                    [
                        TextSpan.new("Underline", is_underline=True),
                        TextSpan.new(" and "),
                        TextSpan.new("Code", is_code=True),
                        TextSpan.new(" too"),
                    ],
                ),
            ],
        ),
    ],
)
def test_text_to_markdown_roundtrip(markdown: str, lines: list[TextLine]):
    text = Text(lines=lines)

    # markdown -> text
    text_from_markdown = markdown_to_text(markdown)
    # easier to debug than text_from_markdown == text (because its nested)
    for i, expected_line in enumerate(lines):
        parsed_line = text_from_markdown.lines[i]
        assert parsed_line == expected_line
    assert text_from_markdown == text  # for sanity

    # text -> markdown
    # TODO :Incomplete :Test: implement more optimal text to markdown (and test it in roundtrip)
    #  Currently, we naively wrap each span individually (ignoring successive spans), resulting in bloated MD.
    # markdown_from_text = text_to_markdown(text)
    # assert markdown_from_text == markdown
