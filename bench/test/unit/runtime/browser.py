from bench.language.browser import Browser
from bench.test.unit.conftest import RuntimeHandle


async def test_acquire_browser_resource_directly(hosted_runtime: RuntimeHandle):
    browser = Browser.new(title="My Browser")
    hosted_runtime.bench.append(browser)
    await hosted_runtime.commit()


async def test_run_flow_browser(hosted_runtime: RuntimeHandle):
    pass  # nocheckin
