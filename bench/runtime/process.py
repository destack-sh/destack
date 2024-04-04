from subprocess import Popen
from uuid import UUID

import structlog

from bench.proto.services import MonitoredServiceBase
from bench.proto.wire import RuntimeBase

logger = structlog.get_logger(__name__)


class Runtime(RuntimeBase, MonitoredServiceBase):
    """
    Manages the lifecycle of the server node's server processes in a main sidecar process.
    During local development, this may also launch the server node in the same process.
    """

    def __init__(self, server_id: UUID, bench_id: UUID | None):
        super().__init__()
        self.server_id = server_id
        self.bench_id = bench_id
        self.processes: dict[UUID, Popen] = {}
        self._stopped = False

    def __str__(self):
        return f"{self.bench_id} {self.server_id}"

    def __repr__(self):
        return f"<ServerHost {self}>"

    async def start(self):
        raise NotImplementedError("TODO :Broken: basic runtime")

    def close(self):
        pass

    async def wait_closed(self):
        pass
