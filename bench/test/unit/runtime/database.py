from bench.language.block import Block
from bench.language.const import BlockType
from bench.test.unit.conftest import RuntimeHandle


async def test_create_database(hosted_runtime: RuntimeHandle):
    runtime = hosted_runtime
    Database1 = Block.new(BlockType.DATABASE, "Database1")
    runtime.page().blocks.append(Database1)
    await runtime.session.commit()

    records = await Database1.records.search()
    assert records == []
