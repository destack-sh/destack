from typing import TYPE_CHECKING, cast, override

import structlog
from opentelemetry import trace

from bench.language import Client, Machine, ResourceStatus, Server
from bench.language.const import (
    ClientType,
    NodeType,
)
from bench.system.host.core import Commit
from bench.system.provision.provisioner import Provisioner
from bench.system.utils.access import ACCESS_TOKEN_LENGTH
from bench.utils.func import bittuple, generate_access_token

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
                client.machine = machine

        # update server status to reflect machines (if needed)
        if machines and all(m.status == ResourceStatus.READY for m in machines):
            current_status = ResourceStatus.READY
        else:
            current_status = ResourceStatus.NOT_READY
        if server.status != current_status:
            async with self.host.session(commit=True):
                server.current_status = current_status

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
            if server.status != ResourceStatus.GONE:
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
            resource.status = ResourceStatus.GONE
