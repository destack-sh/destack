import pytest

from bench.language.action import ActionMode
from bench.language.block import ActionBlock, Block
from bench.language.const import BlockType, RunStatus
from bench.language.field import Field
from bench.language.run import ModelProvider, RunErrorType
from bench.language.text import md
from bench.runtime.core import ModelIncapableError
from bench.test.unit.conftest import RuntimeHandle

TEST_MODEL_PROVIDERS = (ModelProvider.OPENAI, ModelProvider.ANTHROPIC)


def _for_every_provider():
    return pytest.mark.parametrize(
        "model_provider", TEST_MODEL_PROVIDERS, ids=lambda p: p.name if p else p
    )


async def test_run_action_empty_code(hosted_runtime: RuntimeHandle):
    """Running an empty action in strict mode should raise an error."""
    ActionBlock1 = Block.new(
        ActionBlock,
        "Action1",
        mode=ActionMode.CODE,
    )
    hosted_runtime.page().blocks.append(ActionBlock1)
    await hosted_runtime.commit()

    runner = await hosted_runtime.run(ActionBlock1, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.error and runner.error.type == RunErrorType.RUN_IMPOSSIBLE


async def test_run_action_empty_generate(hosted_runtime: RuntimeHandle):
    """Running an empty action in generate mode should raise an error."""
    ActionBlock1 = Block.new(ActionBlock, "Action1", mode=ActionMode.GENERATE)
    hosted_runtime.page().blocks.append(ActionBlock1)
    await hosted_runtime.commit()

    with pytest.raises(ModelIncapableError):
        _ = await hosted_runtime.run(ActionBlock1)


async def test_run_action_math(hosted_runtime: RuntimeHandle):
    """
    Running a generate action with a math implementation should work.
    Changing the action and re-running should change the output.
    """
    # Base: 'Add 1'
    ActionBlock1 = Block.new(
        ActionBlock,
        "Action1",
        text=md("Add 1"),
        mode=ActionMode.GENERATE,
        fields=[Field.input("x", int), Field.output("y", int)],
    )
    hosted_runtime.page().blocks.append(ActionBlock1)
    await hosted_runtime.commit()
    runner = await hosted_runtime.run(ActionBlock1, inputs={"x": 1})
    assert runner.outputs and runner.outputs.y == 2
    runner = await hosted_runtime.run(ActionBlock1, inputs={"x": 2})
    assert runner.outputs and runner.outputs.y == 3

    # New: 'Add 2'
    ActionBlock1.text = md("Add 2")
    runner = await hosted_runtime.run(ActionBlock1, inputs={"x": 1})
    assert runner.outputs and runner.outputs.y == 3
    runner = await hosted_runtime.run(ActionBlock1, inputs={"x": 2})
    assert runner.outputs and runner.outputs.y == 4


async def test_run_action_dynamic_text(hosted_runtime: RuntimeHandle):
    """
    Running a generate action with a desired dynamic behaviour should update it to dynamic,
     and then it should run as expected.
    Changing it back to desired generate behavior should return it to generate mode.
    """
    # Sentiment analysis
    Sentiment = Block.new(
        BlockType.CHOICE,
        "Sentiment",
        fields=[Field.option("Positive"), Field.option("Negative")],
    )
    ActionBlock1 = Block.new(
        ActionBlock,
        "Action1",
        text=md("Judge text sentiment"),
        mode=ActionMode.GENERATE,
        fields=(Field.input("Text", str), Field.output("Sentiment", Sentiment)),
    )
    hosted_runtime.page().blocks.append(ActionBlock1)
    await hosted_runtime.commit()

    # Run as generate, should become dynamic
    runner = await hosted_runtime.run(ActionBlock1, inputs={"Text": "That was great!"})
    assert runner.outputs and runner.outputs.Sentiment == Sentiment.fields.Positive
    assert ActionBlock1.is_dynamic  # should change to dynamic

    # Change so that it should run as generate again
    ActionBlock1.text = md("The text is positive if it contains the phrase 'good' (verbatim)")
    runner = await hosted_runtime.run(ActionBlock1, inputs={"Text": "That was good!"})
    assert runner.outputs and runner.outputs.Sentiment == Sentiment.fields.Positive
    assert not ActionBlock1.is_dynamic  # should change back to generate
    runner = await hosted_runtime.run(ActionBlock1, inputs={"Text": "That was bad!"})
    assert runner.outputs and runner.outputs.Sentiment == Sentiment.fields.Negative
    assert not ActionBlock1.is_dynamic  # should remain generate
