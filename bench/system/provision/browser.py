from typing import override

from bench.language.bench import ResourceStatus
from bench.language.browser import Browser
from bench.language.const import NodeType
from bench.runtime.playwright import playwright_api
from bench.system.provision.provisioner import Provisioner
from bench.utils.func import bittuple


class BrowserbaseBrowserProvisioner(Provisioner[Browser, Browser]):
    """Provision Browsers on localhost."""

    watch_types = bittuple(NodeType.BROWSER)
    provision_type = NodeType.BROWSER

    @override
    async def _do_provision(self, resource: Browser):
        raise NotImplementedError(f"nocheckin: provision {resource!r}")

    @override
    async def _do_decommission(self, resource: Browser):
        raise NotImplementedError(f"nocheckin: decommission {resource!r}")


class LocalhostBrowserProvisioner(Provisioner[Browser, Browser]):
    """Provision Browsers on localhost."""

    watch_types = bittuple(NodeType.BROWSER)
    provision_type = NodeType.BROWSER

    @override
    async def _do_start(self) -> None:
        # local 'Browsers' are always just 'declared' on start
        browsers = await Browser.where(
            Browser.get_property("bench").eq(self.bench)
            & Browser.get_property("status").neq(ResourceStatus.DECOMMISSIONED)
        ).tolist()
        async with self.host.session(commit=True):
            for browser in browsers:
                await playwright_api.provision_local_browser(browser)
                browser.status = ResourceStatus.UP

    @override
    async def _do_provision(self, resource: Browser):
        async with self.host.session(commit=True):
            await playwright_api.provision_local_browser(resource)
            resource.status = ResourceStatus.UP

    @override
    async def _do_decommission(self, resource: Browser):
        async with self.host.session(commit=True):
            await playwright_api.decommission_local_browser(resource)
            resource.status = ResourceStatus.DECOMMISSIONED
