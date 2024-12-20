import pytest

from bench.language.bench import ResourceStatus
from bench.language.block import Block
from bench.language.browser import Browser
from bench.language.const import BlockType, RunStatus
from bench.language.field import Field
from bench.language.flow import PipeType, Step, StepType
from bench.test.unit.conftest import RuntimeHandle


@pytest.mark.browser
async def test_acquire_browser_resource_directly(hosted_runtime: RuntimeHandle):
    browser = Browser.new(title="My Lil' Browser")
    hosted_runtime.bench.append(browser)
    await browser.wait_until_ready()
    assert browser.status == ResourceStatus.UP


@pytest.mark.browser
async def test_run_flow_browser_go_to_url(hosted_runtime: RuntimeHandle):
    Flow = Block.new(BlockType.FLOW, name="Flow", fields=[Field.variable("Browser", Browser)])
    Start = Step.new(StepType.START, name="Start")
    GoToUrl = Step.new(StepType.GO_TO_URL, name="GoToUrl", url="https://google.com")
    Complete = Step.new(StepType.COMPLETE, name="Complete")
    Flow.steps.extend(Start, GoToUrl, Complete)
    Start.connect(PipeType.PASS, target=GoToUrl)
    GoToUrl.connect(PipeType.PASS, target=Complete)
    await hosted_runtime.commit()

    runner = await hosted_runtime.run(Flow)
    assert runner.status == RunStatus.COMPLETED
