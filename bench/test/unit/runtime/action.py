import pytest

from bench.language.action import ActionMode
from bench.language.block import ActionBlock, Block
from bench.language.const import RunStatus
from bench.language.field import Field
from bench.language.run import ModelProvider, RunErrorType
from bench.language.text import md
from bench.test.unit.conftest import RuntimeHandle

TEST_MODEL_PROVIDERS = (ModelProvider.OPENAI, ModelProvider.ANTHROPIC)


def _for_every_provider():
    return pytest.mark.parametrize(
        "model_provider", TEST_MODEL_PROVIDERS, ids=lambda p: p.name if p else p
    )


async def test_run_action_empty_strict(local_runtime: RuntimeHandle):
    """Running an empty action in strict mode should raise an error."""
    ActionBlock1 = Block.new(
        ActionBlock,
        "Action1",
        mode=ActionMode.STRICT,
    )
    local_runtime.page().blocks.append(ActionBlock1)
    await local_runtime.commit()

    runner = await local_runtime.run(ActionBlock1, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.error and runner.error.type == RunErrorType.RUN_IMPOSSIBLE


async def test_run_action_empty_adaptive(local_runtime: RuntimeHandle):
    """Running an empty action in adaptive mode should do nothing."""
    ActionBlock1 = Block.new(ActionBlock, "Action1", mode=ActionMode.ADAPTIVE)
    local_runtime.page().blocks.append(ActionBlock1)
    await local_runtime.commit()

    _ = await local_runtime.run(ActionBlock1)


async def test_run_action_math(local_runtime: RuntimeHandle):
    """
    Running an adaptive action with a math implementation should work.
    Changing the action and re-running should change the output.
    """
    # Base: 'Add 1'
    ActionBlock1 = Block.new(
        ActionBlock,
        "Action1",
        text=md("Add 1"),
        mode=ActionMode.ADAPTIVE,
        fields=[Field.input("x", int), Field.output("y", int)],
    )
    local_runtime.page().blocks.append(ActionBlock1)
    await local_runtime.commit()
    runner = await local_runtime.run(ActionBlock1, inputs={"x": 1})
    assert runner.outputs and runner.outputs.y == 2

    # New: 'Add 2'
    ActionBlock1.text = md("Add 2")
    runner = await local_runtime.run(ActionBlock1, inputs={"x": 1})
    assert runner.outputs and runner.outputs.y == 3
