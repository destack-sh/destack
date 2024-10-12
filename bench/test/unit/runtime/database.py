import pytest
from grpclib import GRPCError, Status

from bench.language.block import Block
from bench.language.const import BlockType
from bench.test.unit.conftest import RuntimeHandle


async def test_crud_database_block(hosted_runtime: RuntimeHandle):
    runtime = hosted_runtime
    Database1 = Block.new(BlockType.DATABASE, "Database1")
    runtime.page().blocks.append(Database1)

    # cannot access database before committing it
    with pytest.raises(GRPCError) as e:  # :BadRemoteErrors
        _ = await Database1.records.search()
        assert e.value.status == Status.FAILED_PRECONDITION

    # commit to create database
    await runtime.session.commit()

    # access, should be empty
    records = await Database1.records.search()
    assert records == []
