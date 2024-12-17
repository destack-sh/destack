from bench.language.browser import Browser
from bench.language.const import NodeType
from bench.system.provision.provisioner import Provisioner
from bench.utils.func import bittuple


class LocalhostBrowserProvisioner(Provisioner[Browser, Browser]):
    """Provision Browsers on localhost."""

    watch_types = bittuple(NodeType.BROWSER)
    resource_type = NodeType.BROWSER
