from bench.language.block import Block
from bench.language.const import BlockType
from bench.language.field import Field
from bench.language.session import Session


async def test_add_detached_subtree(session: Session, page: Block):
    choice = Block.new(BlockType.CHOICE, "Letter")
    for i in range(0, 26):
        letter = chr(65 + i)
        choice.fields.append(Field.option(letter))
    page.blocks.append(choice)
    await session.commit()


async def test_clone_subtree(session: Session, page: Block):
    choice = Block.new(BlockType.CHOICE, "Letter")
    for i in range(0, 26):
        letter = chr(65 + i)
        choice.fields.append(Field.option(letter))
    page.blocks.append(choice)
    await session.commit()

    choice_clone = choice.clone()
    assert choice_clone._equals_content(choice)
    for field, field_clone in zip(choice.fields, choice_clone.fields):
        assert field_clone._equals_content(field)
    await session.commit()
