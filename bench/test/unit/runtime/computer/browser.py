import pytest

from bench.language import Application, ApplicationType, ResourceStatus
from bench.test.simulation.core import Simulation
from bench.test.simulation.workload import RuntimeLambdaWorkload
from bench.test.unit.conftest import simulated_runtime


@pytest.mark.browser
@simulated_runtime()
async def test_acquire_browser_resource_directly(
    simulation: Simulation, runtime: RuntimeLambdaWorkload
):
    """Acquire a Browser directly and wait for it to be ready."""
    browser = Application.new(ApplicationType.CHROME_BROWSER, "My Lil' Browser")
    runtime.main_package.append(browser)
    await browser.wait_until_ready()
    assert browser.status == ResourceStatus.UP
