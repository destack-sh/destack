import pytest

from bench.language.block import Block
from bench.runtime.runner import RuntimeRunner


@pytest.mark.skip("nocheckin")
async def test_run_empty_text(runner: RuntimeRunner, page: Block):
    Text101 = Block.new_text(
        "Text1",
        """""",
    )
    page.blocks.append(Text101)
    await runner.session.commit()

    _ = await runner.run(Text101)
