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
from bench.test.unit.conftest import RuntimeHandle


@pytest.mark.browser
async def test_acquire_browser_resource_directly(hosted_runtime: RuntimeHandle):
    """Acquire a Browser directly and wait for it to be ready."""
    browser = Browser.new(name="My Lil' Browser")
    hosted_runtime.bench.append(browser)
    await browser.wait_until_ready()
    assert browser.status == ResourceStatus.UP


@pytest.mark.browser
@pytest.mark.slow
async def test_run_flow_browser_go_to_url(hosted_runtime: RuntimeHandle):
    """Use a Browser as a 'variable' in a Flow to open a URL and observe the state."""
    Flow = Block.new(BlockType.FLOW, name="Flow", fields=[Field.variable("Browser", Browser)])
    Start = Action.new(ActionType.START, name="Start")
    GoToUrl = Action.new(ActionType.GO_TO_URL, name="GoToUrl", url="https://symbolx.com")
    Wait = Action.new(ActionType.WAIT, name="Wait", delay=timedelta(seconds=3))
    Observe = Action.new(ActionType.OBSERVE, name="Observe")
    Complete = Action.new(ActionType.COMPLETE, name="Complete")
    Flow.actions.extend(Start, GoToUrl, Wait, Observe, Complete)
    Start.connect(PipeType.FORWARD, target=GoToUrl)
    GoToUrl.connect(PipeType.FORWARD, target=Wait)
    Wait.connect(PipeType.FORWARD, target=Observe)
    Observe.connect(PipeType.FORWARD, target=Complete)
    hosted_runtime.page().blocks.append(Flow)
    await hosted_runtime.commit()

    runner = await hosted_runtime.run(Flow)
    assert runner.status == RunStatus.COMPLETED
