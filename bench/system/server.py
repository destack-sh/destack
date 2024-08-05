from pathlib import Path
from typing import TYPE_CHECKING, cast, override

import docker
import structlog
from opentelemetry import trace

from bench.language import Bench, Machine, ResourceStatus, Server
from bench.language.const import CLOUD, NodeType
from bench.system.core import Commit, HostApi
from bench.system.provisioner import Provisioner
from bench.utils.analytics import SENTRY_DSN
from bench.utils.env import ENV
from bench.utils.func import bittuple
from bench.utils.utils import get_from_env

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class ElasticServerProvisioner(Provisioner[Server, Server | Machine]):
    """Provision Servers by creating/deleting/scaling Machines 'on-demand'."""

    watch_types = bittuple(NodeType.SERVER, NodeType.MACHINE)
    provision_types = bittuple(NodeType.SERVER)

    async def _reconcile(self, server: Server):
        machines = server.machines.tolist()

        # "rescale server" (just ensure a single machine exists for now)
        # nocheckin? :Incomplete: scale ElasticServerProvisioner properly (up/down/sleep/...)
        if not machines:
            async with self.host.session(autocommit=True):
                machine = Machine(name="Machine1", cpu=0.25, ram=0.5)
                server.machines.append(machine)
                server.status = ResourceStatus.PROVISIONING

        # update server status to reflect machines (if needed)
        if machines and all(m.status == ResourceStatus.HEALTHY for m in machines):
            actual_status = ResourceStatus.HEALTHY
        elif machines and any(m.status == ResourceStatus.UNHEALTHY for m in machines):
            actual_status = ResourceStatus.UNHEALTHY
        else:
            actual_status = ResourceStatus.HEALTHY  # not sure?
        if server.status != actual_status:
            async with self.host.session(autocommit=True):
                server.status = actual_status

    @override
    async def _do_on_commit_deferred(self, commit: Commit[Server | Machine]) -> None:
        # find any changed servers (directly or indirectly via machines)
        servers: set[Server] = set()
        for node in commit.edited:
            if isinstance(node, Server):
                servers.add(node)
            elif isinstance(node, Machine):
                servers.add(cast(Server, node.parent))
            else:
                raise TypeError(f"unexpected node type {type(node)}")

        # and check/update them
        for server in servers:
            if server.status != ResourceStatus.DECOMMISSIONED:
                await self._reconcile(server)

    @override
    async def _do_provision(self, resource: Server):
        await self._reconcile(resource)

    @override
    async def _do_update(self, resource: Server):
        await self._reconcile(resource)

    @override
    async def _do_decommission(self, resource: Server):
        # nothing special, child machines are automatically removed too
        async with self.host.session(autocommit=True):
            resource.status = ResourceStatus.DECOMMISSIONED


class LocalhostMachineProvisioner(Provisioner[Machine, Machine]):
    """Provision Machines by short-circuiting to localhost."""

    watch_types = bittuple(NodeType.MACHINE)
    provision_types = bittuple(NodeType.MACHINE)

    def __init__(self, host: HostApi, bench: Bench, local_machine_url: str):
        super().__init__(host, bench)
        self._local_machine_url = local_machine_url

    @override
    async def _do_provision(self, resource: Machine):
        async with self.host.session(autocommit=True):
            resource.connection_uri = self._local_machine_url
            resource.status = ResourceStatus.HEALTHY

    @override
    async def _do_decommission(self, resource: Machine):
        async with self.host.session(autocommit=True):
            resource.status = ResourceStatus.DECOMMISSIONED


def _get_machine_env_vars(machine: Machine, *, is_trusted: bool) -> dict[str, str]:
    """Gets the environment variables for a Machine."""
    env_vars: dict[str, str] = {
        "SERVICE_NAME": "runtime",
        "ENVIRONMENT": ENV.value,
        "CLOUD": CLOUD.slug,
        "REGION": machine.region.slug,
        "TRACING": "0",
        "LOG_LEVEL": "DEBUG",
        "LOG_MODE": "JSON",
    }
    if SENTRY_DSN:
        env_vars["SENTRY_DSN"] = SENTRY_DSN
    if is_trusted:
        ...  # nocheckin: add trusted env vars
    return env_vars


MACHINE_RUNTIME_IMAGE = get_from_env(
    "MACHINE_RUNTIME_IMAGE", description="Runtime container image for machine"
)


class DockerMachineProvisioner(Provisioner[Machine, Machine]):
    """Provision Machines as containers in a Docker installation."""

    # NOTE :DevX: use async docker api (instead of blocking sync)

    watch_types = bittuple(NodeType.MACHINE)
    provision_types = bittuple(NodeType.MACHINE)

    def __init__(self, host: HostApi, bench: Bench, docker_api: docker.DockerClient):
        super().__init__(host, bench)
        self._docker_api = docker_api

    @override
    async def _do_start(self) -> None:
        # nocheckin: start watching docker containers
        ...

    @override
    async def _do_provision(self, resource: Machine):
        project_dir = Path(__file__).parent.parent.parent
        assert project_dir.exists() and project_dir.name == "bench", f"{project_dir!r}"
        container = self._docker_api.containers.run(
            MACHINE_RUNTIME_IMAGE,
            environment=_get_machine_env_vars(resource, is_trusted=True),
            detach=True,
            name=f"bench-{ENV.value}-{CLOUD.slug}-{resource.region.slug}-machine-{resource.id.hex}",
            # nocheckin: mount local bench code
            # volumes={"/bench", "/bench:ro"},
        )
        async with self.host.session(autocommit=True):
            resource.external_id = container.id
            resource.status = ResourceStatus.HEALTHY

    @override
    async def _do_update(self, resource: Machine):
        pass  # nothing to do?

    @override
    async def _do_decommission(self, resource: Machine):
        # remove container with same external_id if exists
        assert resource.external_id is not None, f"{resource!r} has no external id"
        container = self._docker_api.containers.get(resource.external_id)
        if container is not None:
            container.remove(force=True)
        async with self.host.session(autocommit=True):
            resource.status = ResourceStatus.DECOMMISSIONED

    # nocheckin: DockerMachineProvisioner


class KubernetesMachineProvisioner(Provisioner[Machine, Machine]):
    """Provision Machines as Pods on Kubernetes."""

    watch_types = bittuple(NodeType.MACHINE)
    provision_types = bittuple(NodeType.MACHINE)

    @override
    async def _do_provision(self, resource: Machine):
        pass

    @override
    async def _do_update(self, resource: Machine):
        pass

    @override
    async def _do_decommission(self, resource: Machine):
        pass

    # nocheckin: KubernetesMachineProvisioner
