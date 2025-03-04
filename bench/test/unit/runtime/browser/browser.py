from datetime import timedelta

import pytest

from bench.language import (
    Action,
    ActionType,
    Browser,
    Field,
    Flow,
    LinkType,
    ResourceStatus,
    RunStatus,
)
from bench.test.simulation.core import Simulation
from bench.test.simulation.workload import RuntimeLambdaWorkload
from bench.test.unit.conftest import simulated_runtime


@pytest.mark.browser
@simulated_runtime()
async def test_acquire_browser_resource_directly(
    simulation: Simulation, runtime: RuntimeLambdaWorkload
):
    """Acquire a Browser directly and wait for it to be ready."""
    browser = Browser.new(name="My Lil' Browser")
    runtime.bench.append(browser)
    await browser.wait_until_ready()
    assert browser.status == ResourceStatus.UP


@pytest.mark.browser
@pytest.mark.slow
@pytest.mark.skip(reason="Flaky")
@simulated_runtime()
async def test_run_flow_browser_go_to_url(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Use a Browser as a 'variable' in a Flow to open a URL and observe the state."""
    Flow1 = Flow.new("Flow", fields=[Field.resource("Browser", Browser)])
    Start = Action.new(ActionType.START, name="Start")
    GoToUrl = Action.new(ActionType.GO_TO_URL, name="GoToUrl", url="https://symbolx.com")
    Wait = Action.new(ActionType.WAIT, name="Wait", delay=timedelta(seconds=3))
    Observe = Action.new(ActionType.LOOK, name="Observe")
    Complete = Action.new(ActionType.COMPLETE, name="Complete")
    Flow1.actions.extend(Start, GoToUrl, Wait, Observe, Complete)
    Start.connect(LinkType.REQUIRE, target=GoToUrl, is_manual=True)
    GoToUrl.connect(LinkType.REQUIRE, target=Wait, is_manual=True)
    Wait.connect(LinkType.REQUIRE, target=Observe, is_manual=True)
    Observe.connect(LinkType.REQUIRE, target=Complete, is_manual=True)
    runtime.page().append(Flow1)
    await runtime.commit()

    runner = await runtime.run_in_runtime(Flow1)
    assert runner.status == RunStatus.COMPLETED
