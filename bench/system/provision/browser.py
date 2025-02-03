from typing import override

from bench.language import Browser, NodeType, ResourceStatus
from bench.language.connection.connection import connection_capture
from bench.language.core import bittuple

from .browserbase import browserbase_api
from .playwright import playwright_server
from .provisioner import Provisioner


class BrowserbaseBrowserProvisioner(Provisioner[Browser, Browser]):
    """Provision Browsers on localhost."""

    watch_types = bittuple(NodeType.BROWSER)
    provision_type = NodeType.BROWSER

    @override
    @connection_capture("close_and_release")
    async def _do_start(self) -> None:
        browsers = await Browser.where(
            Browser.get_property("bench").eq(self.bench)
            & Browser.get_property("status").neq(ResourceStatus.DECOMMISSIONED)
            & Browser.get_property("external_id").exists()
        ).tolist()
        running_browsers = await browserbase_api.get_running_browsers()
        running_browsers_by_external_id = {
            browser.external_id: browser for browser in running_browsers if browser.external_id
        }
        # synchronize Browsers with Browserbase status
        async with self.host.session(commit=True):
            for browser in browsers:
                if (
                    browser.external_id not in running_browsers_by_external_id
                    and browser.status.is_extant
                ):
                    browser.status = ResourceStatus.DECOMMISSIONED

    @override
    async def _do_provision(self, resource: Browser):
        connection = await browserbase_api.start_browser(resource)
        async with self.host.session(commit=True):
            resource.connection_uri = connection.connection_uri
            resource.debugger_uri = connection.debugger_uri
            resource.view_uri = connection.view_uri
            resource.external_id = connection.external_id
            resource.status = ResourceStatus.UP

    @override
    async def _do_decommission(self, resource: Browser):
        if resource.external_id:
            await browserbase_api.stop_browser(resource)
        async with self.host.session(commit=True):
            resource.status = ResourceStatus.DECOMMISSIONED


class LocalhostBrowserProvisioner(Provisioner[Browser, Browser]):
    """Provision Browsers on localhost."""

    watch_types = bittuple(NodeType.BROWSER)
    provision_type = NodeType.BROWSER

    @override
    @connection_capture("close_and_release")
    async def _do_start(self) -> None:
        # local Browsers have to be re-provisioned on start (since playwright is a subprocess)
        browsers = await Browser.where(
            Browser.get_property("bench").eq(self.bench)
            & Browser.get_property("status").neq(ResourceStatus.DECOMMISSIONED)
        ).tolist()
        async with self.host.session(commit=True):
            for browser in browsers:
                connection = await playwright_server.start_browser(browser)
                browser.connection_uri = connection.connection_uri
                browser.debugger_uri = connection.debugger_uri
                browser.view_uri = connection.view_uri
                browser.external_id = connection.external_id
                browser.status = ResourceStatus.UP

    @override
    async def _do_provision(self, resource: Browser):
        connection = await playwright_server.start_browser(resource)
        async with self.host.session(commit=True):
            resource.connection_uri = connection.connection_uri
            resource.debugger_uri = connection.debugger_uri
            resource.view_uri = connection.view_uri
            resource.status = ResourceStatus.UP

    @override
    async def _do_decommission(self, resource: Browser):
        await playwright_server.stop_browser(resource)
        async with self.host.session(commit=True):
            resource.status = ResourceStatus.DECOMMISSIONED
