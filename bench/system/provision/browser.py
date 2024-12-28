from typing import override

from bench.language.bench import ResourceStatus
from bench.language.browser import Browser, BrowserType
from bench.language.const import NodeType
from bench.runtime.playwright import playwright_api
from bench.system.provision.provisioner import Provisioner
from bench.utils.func import bittuple

# NOTE :Architecture: Resource subtypes may need different Provisioners :SubtypeProvisioning
#  (how should we assign Provisioners? Just run them all and have some tied to subtypes?
#   how should we determine which Resource subtype to use? How to know which BrowserType is good?)


class BrowserbaseBrowserProvisioner(Provisioner[Browser, Browser]):
    """Provision Browsers on localhost."""

    watch_types = bittuple(NodeType.BROWSER)
    resource_type = NodeType.BROWSER

    @override
    async def _do_provision(self, resource: Browser):
        resource.type = BrowserType.CHROMIUM  # :SubtypeProvisioning
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
            resource.type = BrowserType.LOCAL  # :SubtypeProvisioning
            await playwright_api.provision_local(resource)
            resource.status = ResourceStatus.UP

    @override
    async def _do_decommission(self, resource: Browser):
        async with self.host.session(commit=True):
            await playwright_api.decommission_local(resource)
            resource.status = ResourceStatus.DECOMMISSIONED
