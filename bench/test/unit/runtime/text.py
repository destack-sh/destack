import pytest

from bench.language.block import Block
from bench.language.const import BlockType, RunStatus
from bench.language.field import Field
from bench.language.run import ModelOptions, ModelProvider, RunErrorType, RunOptions
from bench.language.validation import constrain
from bench.runtime.runner import RuntimeRunner

TEST_MODEL_PROVIDERS = (
    ModelProvider.OPENAI,
    ModelProvider.ANTHROPIC,
)


def _for_every_provider():
    return pytest.mark.parametrize(
        "model_provider", TEST_MODEL_PROVIDERS, ids=lambda p: p.name if p else p
    )


async def test_run_text_empty(runner: RuntimeRunner, page: Block):
    """Empty Text without any fields should fail."""
    Text1 = Block.new_text("Text1", "")
    page.blocks.append(Text1)
    await runner.session.commit()

    run = await runner.run(Text1, return_error=True)
    assert run.status == RunStatus.FAILED
    assert run.error and run.error.type == RunErrorType.NOT_RUNNABLE


@pytest.mark.model()
@_for_every_provider()
async def test_run_text_output_scalar(
    runner: RuntimeRunner, page: Block, model_provider: ModelProvider
):
    AnalyzeSentiment = Block.new_text(
        "AnalyzeSentiment",
        "",
        fields=[Field.input("Text", str), Field.output("IsHappy", bool)],
        run_options=RunOptions(model_options=ModelOptions(provider=model_provider)),
    )
    page.blocks.append(AnalyzeSentiment)
    await runner.session.commit()

    run = await runner.run(
        AnalyzeSentiment, inputs={"Text": "Today was a great day."}, return_error=True
    )
    assert run.status == RunStatus.COMPLETED
    assert run.outputs and run.outputs.IsHappy is True


@pytest.mark.model()
@_for_every_provider()
async def test_run_text_output_dict(
    runner: RuntimeRunner, page: Block, model_provider: ModelProvider
):
    Mood = Block.new(
        BlockType.CHOICE,
        "Mood",
        fields=[Field.option("Positive"), Field.option("Neutral"), Field.option("Negative")],
    )
    WritingStyle = Block.new(
        BlockType.CLASS,
        "WritingStyle",
        fields=[Field.member("formality", int, constrain(min_value=0, max_value=10))],
    )
    AnalyzeSentiment = Block.new_text(
        "AnalyzeSentiment",
        "",
        fields=[
            Field.input("Text", str),
            Field.output("Style", WritingStyle),
            Field.output("Mood", Mood),
        ],
        run_options=RunOptions(model_options=ModelOptions(provider=model_provider)),
    )
    page.blocks.append(AnalyzeSentiment)
    await runner.session.commit()

    run = await runner.run(
        AnalyzeSentiment, inputs={"Text": "today's a great day"}, return_error=True
    )
    assert run.status == RunStatus.COMPLETED
    assert run.outputs and run.outputs.Mood == Mood.fields.Positive
