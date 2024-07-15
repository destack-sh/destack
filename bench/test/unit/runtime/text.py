import pytest

from bench.language.block import Block
from bench.language.const import RunStatus
from bench.language.field import Field
from bench.language.run import ModelOptions, ModelProvider, RunErrorType, RunOptions
from bench.runtime.runner import RuntimeRunner

TEST_MODEL_PROVIDERS = (
    None,  # = automatic
    ModelProvider.OPENAI,
    ModelProvider.ANTHROPIC,
)


async def test_run_empty_text_no_io(runner: RuntimeRunner, page: Block):
    """Empty Text without any fields should fail."""
    Text1 = Block.new_text("Text1", "")
    page.blocks.append(Text1)
    await runner.session.commit()

    run = await runner.run(Text1, suppress_error=True)
    assert run.status == RunStatus.FAILED
    assert run.error and run.error.type == RunErrorType.NOT_RUNNABLE


@pytest.mark.model()
@pytest.mark.parametrize("model_provider", TEST_MODEL_PROVIDERS, ids=lambda p: p.name if p else p)
async def test_run_empty_text_with_io(
    runner: RuntimeRunner, page: Block, model_provider: ModelProvider
):
    """Empty Text with fields should work."""
    Text1 = Block.new_text(
        "Text1",
        "",
        fields=[Field.input("Text", str), Field.output("IsHappy", bool)],
        run_options=RunOptions(model_options=ModelOptions(provider=model_provider)),
    )
    page.blocks.append(Text1)
    await runner.session.commit()

    run = await runner.run(Text1, inputs={"Text": "Hello, I'm very happy!"}, suppress_error=True)
    assert run.status == RunStatus.COMPLETED
    assert run.outputs and run.outputs.IsHappy is True
