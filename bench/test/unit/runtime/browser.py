import asyncio

import pytest

from bench.language.bench import ResourceStatus
from bench.language.browser import Browser
from bench.test.unit.conftest import RuntimeHandle


@pytest.mark.browser
async def test_acquire_browser_resource_directly(hosted_runtime: RuntimeHandle):
    browser = Browser.new(title="My Lil' Browser")
    hosted_runtime.bench.append(browser)
    await browser.wait_until_ready()
    assert browser.status == ResourceStatus.UP
    await asyncio.sleep(3)


@pytest.mark.browser
async def test_run_flow_browser(hosted_runtime: RuntimeHandle):
    pass  # nocheckin
