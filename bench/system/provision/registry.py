from bench.language import Bench
from bench.system.host import Host
from bench.utils.env import ENV, Env

from .browser import BrowserbaseBrowserProvisioner
from .provisioner import Provisioner


def get_provisioners(host: Host, bench: Bench) -> list[Provisioner]:
    """Gets all available provisioners for that Bench in *this* environment"""
    from .browser import LocalhostBrowserProvisioner
    from .machine import (
        KubernetesMachineProvisioner,
        LocalhostMachineProvisioner,
    )
    from .scaler import BrowserScalerProvisioner, MachineScalerProvisioner
    from .store import LocalhostStoreProvisioner, NeonStoreProvisioner

    provisioners: list[type[Provisioner]]
    if ENV == Env.TEST or ENV == Env.DEV:
        provisioners = [
            BrowserScalerProvisioner,
            MachineScalerProvisioner,
            LocalhostStoreProvisioner,
            LocalhostMachineProvisioner,
            LocalhostBrowserProvisioner,
        ]
    elif ENV == Env.STAGE or ENV == Env.PROD:
        provisioners = [
            BrowserScalerProvisioner,
            MachineScalerProvisioner,
            NeonStoreProvisioner,
            KubernetesMachineProvisioner,
            BrowserbaseBrowserProvisioner,
        ]
    else:
        raise RuntimeError(f"unexpected environment: {ENV!r}")

    return [provisioner(host, bench) for provisioner in provisioners]
