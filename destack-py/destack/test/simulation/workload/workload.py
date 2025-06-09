import abc
from datetime import datetime
from typing import final

import structlog
from opentelemetry import trace

from destack.test.simulation.core import Simulation, to_value
from destack.utils.oracle import Oracle
from destack.utils.string import Casing, to_casing

from .spec import WorkloadSpec, WorkloadType

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


_workload_cls_by_type: dict[WorkloadType, type["Workload"]] = {}
_workload_spec_cls_by_type: dict[WorkloadType, type[WorkloadSpec]] = {}


def get_workload_cls(typ: WorkloadType) -> type["Workload"]:
    workload_cls = _workload_cls_by_type.get(typ)
    assert workload_cls is not None, f"unknown workload type {typ}"
    return workload_cls


def workload_(typ: WorkloadType, spec_cls: type[WorkloadSpec]):
    def _decorator(cls):
        assert typ not in _workload_cls_by_type, f"duplicate workload type {typ}"
        _workload_cls_by_type[typ] = cls
        _workload_spec_cls_by_type[typ] = spec_cls
        return cls

    return _decorator


class Workload[SpecT: WorkloadSpec](abc.ABC):
    """A simulated Workload."""

    def __init__(self, id: str, spec: SpecT, oracle: Oracle, simulation: "Simulation"):
        self.id = id
        self.spec = spec
        self.oracle = oracle
        self.simulation = simulation
        logger = structlog.get_logger(to_casing(self.__class__.__name__, Casing.SNAKE))
        self.log = logger.bind(workload=self)
        self.started_at: datetime | None = None
        self.terminated_at: datetime | None = None

    def __str__(self) -> str:
        return ""

    @final
    def __repr__(self):
        content_str = str(self)
        if self.started_at is not None:
            if self.terminated_at is not None:
                duration = (self.terminated_at - self.started_at).total_seconds()
                runtime_ns = f"runtime={(duration):.3f}s"
            else:
                duration = (self.oracle.utc() - self.started_at).total_seconds()
                runtime_ns = f"runtime={(duration):.3f}s"
        else:
            runtime_ns = "runtime=<not started>"
        if content_str:
            return f"<{self.__class__.__name__} {content_str}, {runtime_ns}>"
        else:
            return f"<{self.__class__.__name__} {runtime_ns}>"

    @property
    def name(self):
        return self.spec.name

    @property
    def network(self):
        return self.simulation.network

    @property
    def random(self):
        return self.simulation.random

    async def prepare(self):  # noqa: B027
        """Prepare the Workload before running it."""
        pass  # nothing to do by default

    @final
    async def run(self):
        """Runs the full Workload."""
        self.started_at = self.oracle.utc()
        try:
            n_runs = 1
            repeat = to_value(self.random, self.spec.repeat)
            repeat_interval = to_value(self.random, self.spec.repeat_interval)
            while n_runs <= repeat:
                with tracer.start_as_current_span(f"workload.{self.name}"):
                    await self.run_once()
                    self.log.info("workload.run", run=n_runs, span="current")
                    await self.oracle.sleep(repeat_interval)
                n_runs += 1
        finally:
            self.terminated_at = self.oracle.utc()

    @abc.abstractmethod
    async def run_once(self):
        """Runs one repetition of the Workload."""
        raise NotImplementedError

    @final
    @tracer.start_as_current_span("workload.check")
    async def check(self):
        """Validate any post-run conditions."""
        await self.check_self()
        if self.spec.group:
            group = self.simulation.get_workload_group(self.spec.group)
            # NOTE :Performance :Test: check in-group pairings only as needed
            await self.check_group(group)
        self.log.debug("workload.check", span="current")

    async def check_self(self):  # noqa: B027
        """Validates any post-run conditions."""
        pass

    async def check_group(self, group: list["Workload"]):  # noqa: B027
        """Validates any post-run conditions for a group of workloads. Called for every workload in the group."""
        pass
