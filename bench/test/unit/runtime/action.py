import pytest

from bench.language.action import ActionMode
from bench.language.block import Block
from bench.language.const import BlockType, RunStatus
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
    ActionBlock = Block.new(
        BlockType.ACTION,
        "Action1",
        mode=ActionMode.STRICT,
    )
    local_runtime.page().blocks.append(ActionBlock)
    await local_runtime.commit()

    runner = await local_runtime.run(ActionBlock, return_error=True)
    assert runner.status == RunStatus.FAILED
    assert runner.error and runner.error.type == RunErrorType.RUN_IMPOSSIBLE


async def test_run_action_empty_adaptive(local_runtime: RuntimeHandle):
    """Running an empty action in adaptive mode should do nothing."""
    ActionBlock = Block.new(
        BlockType.ACTION,
        "Action1",
        mode=ActionMode.ADAPTIVE,
    )
    local_runtime.page().blocks.append(ActionBlock)
    await local_runtime.commit()

    _ = await local_runtime.run(ActionBlock)


async def test_run_action_math(local_runtime: RuntimeHandle):
    ActionBlock = Block.new(
        BlockType.ACTION,
        "Action1",
        text=md("Add 1"),
        mode=ActionMode.ADAPTIVE,
        fields=[Field.input("x", int), Field.output("y", int)],
    )
    local_runtime.page().blocks.append(ActionBlock)
    await local_runtime.commit()

    runner = await local_runtime.run(ActionBlock, inputs={"x": 1})
    assert runner.outputs and runner.outputs.y == 2
