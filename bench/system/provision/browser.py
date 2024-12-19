import asyncio
from typing import override

from bench.language.bench import ResourceStatus
from bench.language.browser import Browser
from bench.language.const import NodeType
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
        # nocheckin: provision
        async with self.host.session(commit=True):
            await asyncio.sleep(2)
            resource.status = ResourceStatus.UP

    @override
    async def _do_decommission(self, resource: Browser):
        # nocheckin: decommission
        async with self.host.session(commit=True):
            resource.status = ResourceStatus.DECOMMISSIONED
