from bench.language import Block, BlockType, Choice, Package, Page, Session, TextLine


def test_cast_block_in_page(session: Session):
    """InlineSourceNodes should be added and moved as Blocks."""
    Page1 = Page.new("Page1")
    Page2 = Page.new("Page2")

    # plain Text Block
    Text1 = Block.new(BlockType.PARAGRAPH, line=TextLine.plain("Hello, world!"))
    Page1.append(Text1)
    assert len(Page1.blocks) == 1

    # Choice Block
    Choice1 = Choice.new("Choice1")
    ChoiceBlock1 = Page1.append(Choice1)
    assert len(Page1.blocks) == 2
    assert ChoiceBlock1.node == Choice1

    # move Text Block
    Text1.move(to=Page2)
    assert len(Page1.blocks) == 1
    assert len(Page2.blocks) == 1
    assert Text1.parent == Page2

    # move Choice Block (should sync with definition)
    ChoiceBlock1.move(to=Page2)
    assert len(Page1.blocks) == 0
    assert len(Page2.blocks) == 2
    assert ChoiceBlock1.parent == Page2
    assert Choice1.parent == Page2

    # move Choice back (should sync with Block)
    Choice1.move(to=Page1)
    assert len(Page1.blocks) == 1
    assert len(Page2.blocks) == 1
    assert ChoiceBlock1.parent == Page1
    assert Choice1.parent == Page1


def test_delete_restore_block_in_page(session: Session, package: Package):
    """Delete and restore a Block in a Page."""
    Page1 = Page.new("Page1")
    package.append(Page1)
    Choice1 = Choice.new("Choice1")
    ChoiceBlock1 = Page1.append(Choice1)
    assert len(Page1.blocks) == 1

    # deleting the Node should also delete the Block
    Choice1.delete()
    assert Choice1.is_deleted
    assert ChoiceBlock1.is_deleted
    Choice1.restore()
    assert not Choice1.is_deleted
    assert not ChoiceBlock1.is_deleted

    # and vice versa
    ChoiceBlock1.delete()
    assert Choice1.is_deleted
    assert ChoiceBlock1.is_deleted
    Choice1.restore()
    assert not Choice1.is_deleted
    assert not ChoiceBlock1.is_deleted
