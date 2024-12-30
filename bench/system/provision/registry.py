from bench.language.bench import Bench
from bench.system.host.core import Host
from bench.system.provision.browser import BrowserbaseBrowserProvisioner
from bench.system.provision.provisioner import Provisioner
from bench.utils.env import ENV, Env


def get_provisioners(host: Host, bench: Bench) -> list[Provisioner]:
    """Gets all available provisioners for that Bench in *this* environment"""
    from bench.system.provision.browser import LocalhostBrowserProvisioner
    from bench.system.provision.machine import (
        KubernetesMachineProvisioner,
        LocalhostMachineProvisioner,
    )
    from bench.system.provision.scaler import BrowserScalerProvisioner, MachineScalerProvisioner
    from bench.system.provision.store import LocalhostStoreProvisioner, NeonStoreProvisioner

    provisioners: list[type[Provisioner]]
    if ENV == Env.TEST or ENV == Env.DEV:
        provisioners = [
            BrowserScalerProvisioner,
            MachineScalerProvisioner,
            LocalhostStoreProvisioner,
            LocalhostMachineProvisioner,
            # LocalhostBrowserProvisioner,
            BrowserbaseBrowserProvisioner,
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
