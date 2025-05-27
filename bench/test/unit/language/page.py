from bench.language import Block, BlockType, Page, Schema, Session, TextLine, title


def test_cast_block_in_page(session: Session):
    """InlineSourceNodes should be added and moved as Blocks."""
    Page1 = Page(title=title("Page1"))
    Page2 = Page(title=title("Page2"))

    # plain Text Block
    Text1 = Block(type=BlockType.PARAGRAPH, line=TextLine.plain("Hello, world!"))
    Page1.add_child(Text1)
    assert len(Page1.get_children(Block)) == 1

    # Schema Block
    Schema1 = Schema(name="Schema1")
    SchemaBlock1 = Page1.add_child(Schema1)
    assert len(Page1.get_children(Block)) == 2
    assert SchemaBlock1.node == Schema1

    # move Text Block
    Text1.move(to=Page2)
    assert len(Page1.get_children(Block)) == 1
    assert len(Page2.get_children(Block)) == 1
    assert Text1.parent == Page2

    # move Schema Block (should sync with definition)
    SchemaBlock1.move(to=Page2)
    assert len(Page1.get_children(Block)) == 0
    assert len(Page2.get_children(Block)) == 2
    assert SchemaBlock1.parent == Page2
    assert Schema1.parent == Page2

    # move Schema back (should sync with Block)
    Schema1.move(to=Page1)
    assert len(Page1.get_children(Block)) == 1
    assert len(Page2.get_children(Block)) == 1
    assert SchemaBlock1.parent == Page1
    assert Schema1.parent == Page1
