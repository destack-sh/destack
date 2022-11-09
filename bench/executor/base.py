from __future__ import annotations

import abc
import dataclasses
import enum
import threading
import uuid
from queue import Queue
from typing import Any, Dict, Union

import structlog

from bench.model.base import ModelHandler
from bench.models import Flow
from bench.models.execution import Execution
from bench.models.utils import UUIDT

logger = structlog.stdlib.get_logger()
Resource = str
ResourceRequirements = Dict[Resource, Union[int, float]]
PerNodeResourceRequirements = Dict[UUIDT, ResourceRequirements]


class FlowRuntimeValidation(enum.Enum):
    Off = "off"
    Lazy = "lazy"
    Full = "full"


@dataclasses.dataclass
class FlowExecutionOptions:
    blocking: bool
    validate: FlowRuntimeValidation


class Executor(abc.ABC):
    def __init__(self):
        self.executor_id = uuid.uuid4().hex
        self._loaded_models_by_iid: Dict[str, ModelHandler] = {}
        self._executions_queue: Queue[Any] = Queue()
        self._executions_thread = LocalExecutorThread(self, self._executions_queue, daemon=True)

    def start(self):
        self._executions_thread.start()
        self.mark_dead_executions_failed()

    def stop(self):
        if self._executions_thread.is_alive():
            self._executions_thread.stop()
            self._executions_thread.join()

    def mark_dead_executions_failed(self):
        dead_executions = Execution.objects.filter(
            status__in=[status.value for status in Execution.PENDING_STATUSES],
            metadata__queued__executor_type="local",
        )
        for execution in dead_executions:
            logger.warning("mark_dead_queued_execution_failed", execution=execution)
            execution.terminate(
                status=Execution.Status.Failed, transition_metadata={"message": "dead"}
            )

    async def run_flow(
        self,
        flow: Flow,
        options: FlowExecutionOptions,
    ) -> Any:
        raise NotImplementedError


class LocalExecutorThread(threading.Thread):
    def __init__(
        self,
        executor: Executor,
        executions_queue: Queue[Any],
        daemon: bool,
        **kwargs,
    ):
        super().__init__(**kwargs, daemon=daemon)
        self._executions_queue = executions_queue
        self._executor = executor
        self._should_stop = False

    def run(self):
        pass

    def stop(self):
        self._should_stop = True
