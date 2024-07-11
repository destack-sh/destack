from bench.language.block import Block
from bench.language.const import RunStatus
from bench.runtime.runner import RuntimeRunner


async def test_run_empty_text(runner: RuntimeRunner, page: Block):
    Text101 = Block.new_text(
        "Text1",
        """""",
    )
    page.blocks.append(Text101)
    await runner.session.commit()

    run = await runner.run(Text101, suppress_error=True)
    assert run.status == RunStatus.FAILED
