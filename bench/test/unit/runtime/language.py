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
