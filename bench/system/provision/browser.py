from typing import override

from bench.language.bench import ResourceStatus
from bench.language.browser import Browser
from bench.language.const import NodeType
from bench.system.provision.playwright import playwright_server
from bench.system.provision.provisioner import Provisioner
from bench.utils.func import bittuple


class BrowserbaseBrowserProvisioner(Provisioner[Browser, Browser]):
    """Provision Browsers on localhost."""

    # nocheckin: BrowserbaseBrowserProvisioner

    watch_types = bittuple(NodeType.BROWSER)
    provision_type = NodeType.BROWSER

    @override
    async def _do_start(self) -> None:
        pass

    @override
    async def _do_provision(self, resource: Browser):
        raise NotImplementedError

    @override
    async def _do_decommission(self, resource: Browser):
        raise NotImplementedError


class LocalhostBrowserProvisioner(Provisioner[Browser, Browser]):
    """Provision Browsers on localhost."""

    watch_types = bittuple(NodeType.BROWSER)
    provision_type = NodeType.BROWSER

    @override
    async def _do_start(self) -> None:
        # local Browsers have to be re-provisioned on start (since playwright is a subprocess)
        browsers = await Browser.where(
            Browser.get_property("bench").eq(self.bench)
            & Browser.get_property("status").neq(ResourceStatus.DECOMMISSIONED)
        ).tolist()
        async with self.host.session(commit=True):
            for browser in browsers:
                browser.connection_uri = await playwright_server.start_browser(browser)
                browser.status = ResourceStatus.UP

    @override
    async def _do_provision(self, resource: Browser):
        async with self.host.session(commit=True):
            resource.connection_uri = await playwright_server.start_browser(resource)
            resource.status = ResourceStatus.UP

    @override
    async def _do_decommission(self, resource: Browser):
        async with self.host.session(commit=True):
            await playwright_server.stop_browser(resource)
            resource.status = ResourceStatus.DECOMMISSIONED
