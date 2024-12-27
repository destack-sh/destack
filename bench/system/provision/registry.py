import enum
from typing import assert_never

from bench.language.bench import Bench
from bench.system.host.core import Host
from bench.system.provision.browser import BrowserbaseBrowserProvisioner
from bench.system.provision.provisioner import Provisioner
from bench.utils.env import ENV, Env
from bench.utils.utils import get_from_env_maybe


class MachineProvisionerType(enum.StrEnum):
    LOCALHOST = "localhost"
    DOCKER = "docker"
    KUBERNETES = "kubernetes"


MACHINE_PROVISIONER_TYPE = get_from_env_maybe(
    "MACHINE_PROVISIONER_TYPE",
    typ=MachineProvisionerType,
    description="The machine provisioner to use (during development)",
)


def get_provisioners_for(host: Host, bench: Bench) -> list[Provisioner]:
    """Gets all available provisioners for that Bench in *this* environment"""
    from bench.system.provision.browser import LocalhostBrowserProvisioner
    from bench.system.provision.drive import S3DriveProvisioner
    from bench.system.provision.machine import (
        DockerMachineProvisioner,
        KubernetesMachineProvisioner,
        LocalhostMachineProvisioner,
    )
    from bench.system.provision.server import ElasticServerProvisioner
    from bench.system.provision.store import LocalhostStoreProvisioner, NeonStoreProvisioner

    provisioners: list[type[Provisioner]] = []
    if ENV == Env.TEST:
        provisioners = [
            LocalhostStoreProvisioner,
            ElasticServerProvisioner,
            LocalhostMachineProvisioner,
            S3DriveProvisioner,
            LocalhostBrowserProvisioner,
        ]
    elif ENV == Env.DEV:
        # dynamic machine provisioner
        assert MACHINE_PROVISIONER_TYPE is not None, "MACHINE_PROVISIONER_TYPE not set"
        if MACHINE_PROVISIONER_TYPE == MachineProvisionerType.LOCALHOST:
            machine_provisioner = LocalhostMachineProvisioner
        elif MACHINE_PROVISIONER_TYPE == MachineProvisionerType.DOCKER:
            machine_provisioner = DockerMachineProvisioner
        elif MACHINE_PROVISIONER_TYPE == MachineProvisionerType.KUBERNETES:
            machine_provisioner = KubernetesMachineProvisioner
        else:
            assert_never(MACHINE_PROVISIONER_TYPE)

        provisioners = [
            LocalhostStoreProvisioner,
            S3DriveProvisioner,
            machine_provisioner,
            S3DriveProvisioner,
            LocalhostBrowserProvisioner,
        ]
    elif ENV == Env.STAGE or ENV == Env.PROD:
        provisioners = [
            NeonStoreProvisioner,
            ElasticServerProvisioner,
            KubernetesMachineProvisioner,
            S3DriveProvisioner,
            BrowserbaseBrowserProvisioner,
        ]
    else:
        raise RuntimeError(f"unexpected environment: {ENV!r}")

    return [provisioner(host, bench) for provisioner in provisioners]
