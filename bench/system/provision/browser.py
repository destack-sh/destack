import asyncio
from typing import override

from bench.language.bench import ResourceStatus
from bench.language.browser import Browser
from bench.language.const import NodeType
from bench.system.provision.playwright import playwright_api
from bench.system.provision.provisioner import Provisioner
from bench.utils.func import bittuple


class BrowserbaseBrowserProvisioner(Provisioner[Browser, Browser]):
    """Provision Browsers on localhost."""

    watch_types = bittuple(NodeType.BROWSER)
    resource_type = NodeType.BROWSER

    @override
    async def _do_provision(self, resource: Browser):
        raise NotImplementedError(f"nocheckin: provision {resource!r}")

    @override
    async def _do_decommission(self, resource: Browser):
        raise NotImplementedError(f"nocheckin: decommission {resource!r}")


class LocalhostBrowserProvisioner(Provisioner[Browser, Browser]):
    """Provision Browsers on localhost."""

    watch_types = bittuple(NodeType.BROWSER)
    resource_type = NodeType.BROWSER

    @override
    async def _do_provision(self, resource: Browser):
        async with self.host.session(commit=True):
            await asyncio.sleep(2)
            window_size = (int(resource.size.x), int(resource.size.y))
            connection_uri = await playwright_api.provision(
                is_insecure=resource.is_insecure, window_size=window_size
            )
            resource.connection_uri = connection_uri
            resource.status = ResourceStatus.UP

    @override
    async def _do_decommission(self, resource: Browser):
        async with self.host.session(commit=True):
            if resource.external_id is not None:
                await playwright_api.decommission(resource.external_id)
            resource.status = ResourceStatus.DECOMMISSIONED
