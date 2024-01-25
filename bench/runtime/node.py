from subprocess import Popen
from uuid import UUID

import structlog
from grpclib import GRPCError, Status as GRPCStatus

from bench.proto.services import MonitoredServiceBase
from bench.proto.wire import (
    WorkerBase,
    RestartWorkerRequest,
    RestartWorkerResponse,
    StartRunRequest,
    StartRunResponse,
    KillRunRequest,
    KillRunResponse,
)

logger = structlog.get_logger(__name__)


class Worker(WorkerBase, MonitoredServiceBase):
    """
    Manages the lifecycle of the worker node's worker processes in a main sidecar process.
    During local development, this may also launch the worker node in the same process.
    """

    def __init__(
        self,
        worker_set_id: UUID | None,
        worker_id: UUID,
        bench_id: UUID | None,
        package_id: UUID | None,
    ):
        super().__init__()
        self.worker_set_id = worker_set_id
        self.worker_id = worker_id
        self.bench_id = bench_id
        self.package_id = package_id
        self.processes: dict[UUID, Popen] = {}
        self._stopped = False

    def __str__(self):
        return f"{self.bench_id} {self.worker_set_id} {self.worker_id}"

    def __repr__(self):
        return f"<WorkerHost {self}>"

    async def start_quick(self):
        raise NotImplementedError("nocheckin: worker.start_quick")

    def close(self):
        pass

    async def wait_closed(self):
        pass

    async def restart_worker(
        self, restart_worker_request: "RestartWorkerRequest"
    ) -> "RestartWorkerResponse":
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)

    async def start_run(self, start_run_request: "StartRunRequest") -> "StartRunResponse":
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)

    async def kill_run(self, kill_run_request: "KillRunRequest") -> "KillRunResponse":
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)
