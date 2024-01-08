from subprocess import Popen
from uuid import UUID

import structlog

from bench.proto.wire import WorkerBase
from bench.utils.monitoring import Monitored

logger = structlog.get_logger(__name__)


class Worker(Monitored, WorkerBase):
    """
    Manages the lifecycle of the worker node's worker processes in a main sidecar process.
    During local development, this may also launch the worker node in the same process.
    """

    def __init__(
        self,
        worker_set_id: UUID | None,
        worker_id: UUID,
        bench_id: UUID | None,
        module_id: UUID | None,
    ):
        self.worker_set_id = worker_set_id
        self.worker_id = worker_id
        self.bench_id = bench_id
        self.module_id = module_id
        self.processes: dict[UUID, Popen] = {}
        self._stopped = False

    def __str__(self):
        return f"{self.bench_id} {self.worker_set_id} {self.worker_id}"

    def __repr__(self):
        return f"<WorkerHost {self}>"

    async def run(self):
        raise NotImplementedError("nocheckin: worker.run")
