from datetime import timedelta

import pytest

from bench.language import (
    Action,
    ActionType,
    Block,
    BlockType,
    Browser,
    Field,
    PipeType,
    ResourceStatus,
    RunStatus,
)
from bench.test.simulation.workload import RuntimeLambdaWorkload
from bench.test.unit.conftest import simulated_runtime


@pytest.mark.browser
@simulated_runtime()
async def test_acquire_browser_resource_directly(runtime: RuntimeLambdaWorkload):
    """Acquire a Browser directly and wait for it to be ready."""
    browser = Browser.new(name="My Lil' Browser")
    runtime.bench.append(browser)
    await browser.wait_until_ready()
    assert browser.status == ResourceStatus.UP


@pytest.mark.browser
@pytest.mark.slow
@simulated_runtime()
async def test_run_flow_browser_go_to_url(runtime: RuntimeLambdaWorkload):
    """Use a Browser as a 'variable' in a Flow to open a URL and observe the state."""
    Flow = Block.new(BlockType.FLOW, name="Flow", fields=[Field.variable("Browser", Browser)])
    Start = Action.new(ActionType.START, name="Start")
    GoToUrl = Action.new(ActionType.GO_TO_URL, name="GoToUrl", url="https://symbolx.com")
    Wait = Action.new(ActionType.WAIT, name="Wait", delay=timedelta(seconds=3))
    Observe = Action.new(ActionType.LOOK, name="Observe")
    Complete = Action.new(ActionType.COMPLETE, name="Complete")
    Flow.actions.extend(Start, GoToUrl, Wait, Observe, Complete)
    Start.connect(PipeType.CALL, target=GoToUrl)
    GoToUrl.connect(PipeType.CALL, target=Wait)
    Wait.connect(PipeType.CALL, target=Observe)
    Observe.connect(PipeType.CALL, target=Complete)
    runtime.page().blocks.append(Flow)
    await runtime.commit()

    runner = await runtime.run_in_runtime(Flow)
    assert runner.status == RunStatus.COMPLETED
