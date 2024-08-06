import random
from pathlib import Path
from typing import TYPE_CHECKING, cast, override

import docker
import docker.models
import docker.models.containers
import structlog
from opentelemetry import trace

from bench.language import Bench, Client, Machine, ResourceStatus, Server
from bench.language.const import CLOUD, ClientType, NodeType, dockerify_domain
from bench.system.access import ACCESS_TOKEN_LENGTH
from bench.system.core import Commit, HostApi
from bench.system.provisioner import Provisioner
from bench.utils.analytics import SENTRY_DSN
from bench.utils.env import ENV
from bench.utils.func import bittuple, generate_access_token
from bench.utils.utils import get_from_env

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class ElasticServerProvisioner(Provisioner[Server, Server | Machine]):
    """Provision Servers by creating/deleting/scaling Machines (and their Clients) on-demand."""

    watch_types = bittuple(NodeType.SERVER, NodeType.MACHINE)
    provision_types = bittuple(NodeType.SERVER)

    async def _reconcile(self, server: Server):
        machines = server.machines.tolist()

        # "rescale server" (just ensure a single machine exists for now)
        # TODO :Incomplete!: scale ElasticServerProvisioner properly (up/down/sleep/...)
        if not machines:
            async with self.host.session(commit=True) as session:
                client = Client(
                    parent=server,
                    type=ClientType.BENCH_MACHINE,
                    name="Machine1",
                    access_token=generate_access_token(ACCESS_TOKEN_LENGTH),
                )
                session._create(client)
                await session.flush()
                machine = Machine(name="Machine1", cpu=0.25, ram=0.5, client=client)
                server.machines.append(machine)
                server.status = ResourceStatus.PROVISIONING
                client.machine = machine

        # update server status to reflect machines (if needed)
        if machines and all(m.status == ResourceStatus.HEALTHY for m in machines):
            actual_status = ResourceStatus.HEALTHY
        elif machines and any(m.status == ResourceStatus.UNHEALTHY for m in machines):
            actual_status = ResourceStatus.UNHEALTHY
        else:
            actual_status = ResourceStatus.HEALTHY  # not sure?
        if server.status != actual_status:
            async with self.host.session(commit=True):
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
        async with self.host.session(commit=True):
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
        async with self.host.session(commit=True):
            resource.connection_uri = self._local_machine_url
            resource.status = ResourceStatus.HEALTHY

    @override
    async def _do_decommission(self, resource: Machine):
        async with self.host.session(commit=True):
            resource.status = ResourceStatus.DECOMMISSIONED


def _get_machine_env_vars(machine: Machine, *, is_trusted: bool, is_docker: bool) -> dict[str, str]:
    """Gets the environment variables for a Machine."""
    client = machine.client
    assert client, f"{machine!r} has no client"
    supervisor_url = get_from_env("SUPERVISOR_URL", description="Supervisor URL")
    if is_docker:
        supervisor_url = dockerify_domain(supervisor_url)
    env_vars: dict[str, str | None] = {
        # hosting
        "SERVICE_NAME": "runtime",
        "ENVIRONMENT": ENV.value,
        "CLOUD": CLOUD.slug,
        "REGION": machine.region.slug,
        "SUPERVISOR_URL": supervisor_url,
        # bench
        "BENCH_ID": str(machine.bench.id),
        "SERVER_ID": str(machine.parent.id) if isinstance(machine.parent, Server) else None,
        "MACHINE_ID": str(machine.id),
        "CLIENT_ID": str(client.id),
        "CLIENT_TYPE": str(client.type.value),
        "CLIENT_ACCESS_TOKEN": client.access_token,
        # config
        "TRACING": "0",
        "LOG_LEVEL": "DEBUG",
        "LOG_MODE": "JSON",
    }
    if is_docker:
        env_vars["IS_IN_DOCKER"] = "1"
    if SENTRY_DSN:
        env_vars["SENTRY_DSN"] = SENTRY_DSN
    if is_trusted:
        # add secret api keys directly (for testing/development)
        env_vars["OPENAI_API_KEY"] = get_from_env("OPENAI_API_KEY", description="OpenAI API key")
        env_vars["ANTHROPIC_API_KEY"] = get_from_env(
            "ANTHROPIC_API_KEY", description="Anthropic API key"
        )
    return {k: v for k, v in env_vars.items() if v}


MACHINE_RUNTIME_IMAGE = get_from_env(
    "MACHINE_RUNTIME_IMAGE", description="Runtime container image for machine"
)


class DockerMachineProvisioner(Provisioner[Machine, Machine]):
    """Provision Machines as containers in a Docker installation."""

    # NOTE :DevX: use async docker api (instead of blocking sync)

    watch_types = bittuple(NodeType.MACHINE)
    provision_types = bittuple(NodeType.MACHINE)

    def __init__(self, host: HostApi, bench: Bench, docker_client: docker.DockerClient):
        super().__init__(host, bench)
        self._docker_client = docker_client

    @override
    async def _do_start(self) -> None:
        containers: list[docker.models.containers.Container] = self._docker_client.containers.list(
            all=True
        )
        containers_by_id = {cast(str, c.id): c for c in containers}
        servers = self.bench.servers.tolist()
        machines = [m for s in servers for m in s.machines]

        async with self.host.session(commit=True):
            for machine in machines:
                if machine.external_id is None:
                    continue
                container = containers_by_id.get(machine.external_id)
                if container is None:
                    machine.status = ResourceStatus.DECLARED

    @override
    async def _do_provision(self, resource: Machine):
        project_dir = Path(__file__).parent.parent.parent
        assert project_dir.exists() and project_dir.name == "bench", f"{project_dir!r}"
        bench_dir = (project_dir / "bench").absolute().as_posix()
        assigned_port = random.randint(60100, 65000)
        env_vars = _get_machine_env_vars(resource, is_trusted=True, is_docker=True)
        container = self._docker_client.containers.run(
            MACHINE_RUNTIME_IMAGE,
            environment=env_vars,
            detach=True,
            name=f"bench-{ENV.value}-{CLOUD.slug}-{resource.region.slug}-machine-{resource.id}",
            command=["python", "bench.py", "serve", "runtime", "0.0.0.0", str(assigned_port)],
            volumes=[f"{bench_dir}:/bench:ro"],  # mount our local code directly
            ports={f"{assigned_port}/tcp": ("0.0.0.0", assigned_port)},
        )
        async with self.host.session(commit=True):
            resource.external_id = container.id
            resource.status = ResourceStatus.HEALTHY
            resource.connection_uri = f"http://localhost:{assigned_port}"

    @override
    async def _do_update(self, resource: Machine):
        pass  # nothing to do?

    @override
    async def _do_decommission(self, resource: Machine):
        # remove container with same external_id if exists
        assert resource.external_id is not None, f"{resource!r} has no external id"
        container = self._docker_client.containers.get(resource.external_id)
        if container is not None:
            container.remove(force=True)
        async with self.host.session(commit=True):
            resource.status = ResourceStatus.DECOMMISSIONED


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
