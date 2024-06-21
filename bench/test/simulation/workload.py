import abc
from dataclasses import dataclass
from typing import TYPE_CHECKING

import structlog
from opentelemetry import trace

from bench.language.const import EDIT_TYPES, BlockType, EditType
from bench.test.simulation.spec import WorkloadSpec, WorkloadType

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

if TYPE_CHECKING:
    from bench.test.simulation.simulation import Simulation


_workload_cls_by_type: dict[WorkloadType, type["WorkloadBase"]] = {}
_workload_spec_cls_by_type: dict[WorkloadType, type[WorkloadSpec]] = {}


def get_workload_cls(typ: WorkloadType) -> type["WorkloadBase"]:
    workload_cls = _workload_cls_by_type.get(typ)
    assert workload_cls is not None, f"unknown workload type {typ}"
    return workload_cls


def workload(typ: WorkloadType, spec_cls: type[WorkloadSpec]):
    def _decorator(cls):
        assert typ not in _workload_cls_by_type, f"duplicate workload type {typ}"
        _workload_cls_by_type[typ] = cls
        _workload_spec_cls_by_type[typ] = spec_cls
        return cls

    return _decorator


class WorkloadBase[SpecT: WorkloadSpec](abc.ABC):
    def __init__(self, spec: SpecT, simulation: "Simulation"):
        self.spec = spec
        self.simulation = simulation

    @property
    def network(self):
        return self.simulation.network

    @property
    def random(self):
        return self.simulation.random

    async def prepare(self):  # noqa: B027
        pass

    @abc.abstractmethod
    async def run(self):
        raise NotImplementedError


#
# Write
#


@dataclass
class WriteBlockTreeSpec(WorkloadSpec):
    type: WorkloadType = WorkloadType.WRITE_BLOCK_TREE
    client: str | None = None
    block_types: tuple[BlockType, ...] = (BlockType.PAGE, BlockType.TEXT)
    edit_types: tuple[EditType, ...] = EDIT_TYPES.tuple


@workload(WorkloadType.WRITE_BLOCK_TREE, WriteBlockTreeSpec)
class WriteBlockTreeWorkload(WorkloadBase[WriteBlockTreeSpec]):
    async def run(self):
        raise NotImplementedError


#
# Read
#


@dataclass
class ReadBenchSpec(WorkloadSpec):
    type: WorkloadType = WorkloadType.READ_BENCH
    client: str | None = None
    live: bool = True


@workload(WorkloadType.READ_BENCH, ReadBenchSpec)
class ReadBenchWorkload(WorkloadBase[ReadBenchSpec]):
    async def run(self):
        raise NotImplementedError


@dataclass
class ReadPackageSpec(WorkloadSpec):
    type: WorkloadType = WorkloadType.READ_PACKAGE
    client: str | None = None
    live: bool = True
