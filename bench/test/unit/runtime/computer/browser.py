from datetime import timedelta

import pytest

from bench.language import (
    Action,
    ActionType,
    Browser,
    BrowserType,
    Claim,
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
    browser = Browser.new(BrowserType.CHROME, "My Lil' Browser")
    runtime.main_package.append(browser)
    await browser.wait_until_ready()
    assert browser.status == ResourceStatus.UP


@pytest.mark.browser
@pytest.mark.slow
@pytest.mark.skip(reason="Flaky")
@simulated_runtime()
async def test_run_flow_browser_go_to_url(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Use a Browser as in a Flow to open a URL and get a screenshot."""
    from bench.builtin import ActionKit, BrowserKit, ChromeBrowserTemplate

    Flow1 = Flow.new("Flow", claims=[Claim.exclusive("Browser", ChromeBrowserTemplate)])
    Start = Action.new(ActionType.START)
    GoToUrl = Action.new(BrowserKit.actions.Go_To_Url, url="https://symbolx.com")
    Wait = Action.new(ActionKit.actions.Wait, delay=timedelta(seconds=3))
    Observe = Action.new(BrowserKit.actions.Screenshot)
    Complete = Action.new(ActionType.COMPLETE)
    Flow1.actions.extend(Start, GoToUrl, Wait, Observe, Complete)
    Start.connect(LinkType.REQUIRE, GoToUrl, is_manual=True)
    GoToUrl.connect(LinkType.REQUIRE, Wait, is_manual=True)
    Wait.connect(LinkType.REQUIRE, Observe, is_manual=True)
    Observe.connect(LinkType.REQUIRE, Complete, is_manual=True)
    runtime.page().append(Flow1)
    await runtime.commit()

    runner = await runtime.run_in_runtime(Flow1)
    assert runner.status == RunStatus.COMPLETED
