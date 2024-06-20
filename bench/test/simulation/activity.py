import abc

from bench.language.const import BlockType
from bench.test.simulation.spec import ActivitySpec, ActivityType

_activity_cls_by_type: dict[ActivityType, abc.ABC] = {}
_activity_spec_cls_by_type: dict[ActivityType, type[ActivitySpec]] = {}


def workload(typ: ActivityType, spec_cls: type[ActivitySpec]):
    def _decorator(cls):
        assert typ not in _activity_cls_by_type, f"duplicate workload type {typ}"
        _activity_cls_by_type[typ] = cls
        _activity_spec_cls_by_type[typ] = spec_cls
        return cls

    return _decorator


class ActivityBase[SpecT: ActivitySpec](abc.ABC):
    def __init__(self, spec: SpecT):
        self.spec = spec

    async def prepare(self):
        pass

    async def run(self):
        raise NotImplementedError


class WriteBlockTreeSpec(ActivitySpec):
    block_types: tuple[BlockType, ...] = (BlockType.PAGE, BlockType.TEXT)


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
