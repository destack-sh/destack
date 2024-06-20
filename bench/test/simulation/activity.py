import abc
from typing import TYPE_CHECKING

import structlog
from opentelemetry import trace

from bench.language.const import EDIT_TYPES, BlockType, EditType
from bench.test.simulation.spec import ActivitySpec, ActivityType

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

if TYPE_CHECKING:
    from bench.test.simulation.simulation import Simulation


_activity_cls_by_type: dict[ActivityType, type["ActivityBase"]] = {}
_activity_spec_cls_by_type: dict[ActivityType, type[ActivitySpec]] = {}


def get_activity_cls(typ: ActivityType) -> type["ActivityBase"]:
    activity_cls = _activity_cls_by_type.get(typ)
    assert activity_cls is not None, f"unknown activity type {typ}"
    return activity_cls


def workload(typ: ActivityType, spec_cls: type[ActivitySpec]):
    def _decorator(cls):
        assert typ not in _activity_cls_by_type, f"duplicate workload type {typ}"
        _activity_cls_by_type[typ] = cls
        _activity_spec_cls_by_type[typ] = spec_cls
        return cls

    return _decorator


class ActivityBase[SpecT: ActivitySpec](abc.ABC):
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


class WriteBlockTreeSpec(ActivitySpec):
    block_types: tuple[BlockType, ...] = (BlockType.PAGE, BlockType.TEXT)
    edit_types: tuple[EditType, ...] = EDIT_TYPES.tuple


@workload(ActivityType.WRITE_BLOCK_TREE, WriteBlockTreeSpec)
class WriteBlockTreeActivity(ActivityBase[WriteBlockTreeSpec]):
    async def run(self):
        raise NotImplementedError


class ReadBenchSpec(ActivitySpec):
    live: bool = True


@workload(ActivityType.READ_BENCH, ReadBenchSpec)
class ReadBenchActivity(ActivityBase[ReadBenchSpec]):
    async def run(self):
        raise NotImplementedError


class ReadPackageSpec(ActivitySpec):
    live: bool = True
