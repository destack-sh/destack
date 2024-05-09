from subprocess import Popen
from uuid import UUID

import structlog

from bench.proto.services import MonitoredServiceBase
from bench.proto.wire import RuntimeBase

logger = structlog.get_logger(__name__)


class Runtime(RuntimeBase, MonitoredServiceBase):
    """
    nocheckin: ???
    """

    def __init__(
        self,
        *,
        client_id: UUID | None = None,
        user_id: UUID | None = None,
        server_id: UUID | None = None,
        bench_id: UUID | None = None,
    ):
        super().__init__()
        self.client_id = client_id
        self.server_id = server_id
        self.user_id = user_id
        self.bench_id = bench_id
        self.processes: dict[UUID, Popen] = {}
        self._stopped = False

    def __str__(self):
        return f"{self.bench_id} {self.server_id}"

    def __repr__(self):
        return f"<ServerHost {self}>"

    async def start(self):
        pass  # nocheckin: basic runtime

    def close(self):
        pass

    async def wait_closed(self):
        pass
