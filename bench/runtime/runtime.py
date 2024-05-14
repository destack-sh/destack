from urllib.parse import urlparse
from uuid import UUID

import structlog
from grpclib.client import Channel

from bench.proto.services import MonitoredServiceBase
from bench.proto.wire import RuntimeBase, SupervisorStub

logger = structlog.get_logger(__name__)


class Runtime(RuntimeBase, MonitoredServiceBase):
    """
    A Runtime processes selected Runs for a Bench/Package in Sessions.
    A Runtime process is started for each active Package in a Bench.
    """

    def __init__(
        self,
        *,
        supervisor_url: str,
        client_id: UUID | None = None,
        user_id: UUID | None = None,
        server_id: UUID | None = None,
        bench_id: UUID | None = None,
        package_id: UUID | None = None,
    ):
        super().__init__()
        # parse out supervisor host and port
        _supervisor_url = urlparse(supervisor_url)
        self.supervisor_host = _supervisor_url.hostname
        self.supervisor_port = _supervisor_url.port
        if self.supervisor_host is None or self.supervisor_port is None:
            raise ValueError(f"invalid supervisor URL: {supervisor_url}")
        self.supervisor = SupervisorStub(Channel(self.supervisor_host, self.supervisor_port))

        # context
        self.client_id = client_id
        self.server_id = server_id
        self.user_id = user_id
        self.bench_id = bench_id
        self.package_id = package_id

    def __str__(self):
        return f"{self.bench_id} {self.server_id}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    async def start(self):
        pass  # nocheckin: basic runtime

    def close(self):
        pass

    async def wait_closed(self):
        pass
