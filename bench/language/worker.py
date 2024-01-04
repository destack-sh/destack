from datetime import datetime

from bench.language import Node, Struct
from bench.language.const import NodeType, ProjectRegion, StructType, WorkerProfile, WorkerSetStatus
from bench.language.module import node, struct, struct_internal
from bench.utils.dt import utcnow_with_tz


@node(NodeType.WORKER_SET)
class WorkerSet(Node):
    """
    Set of workers to run a Bench's modules.
    """

    region: ProjectRegion = struct_internal(31)
    profile: WorkerProfile = struct_internal(32)
    sleeping: bool = struct_internal(33)
    status: WorkerSetStatus = struct_internal(34)
    desired_replicas: int = struct_internal(35)
    target_replicas: int = struct_internal(36)
    available_replicas: int = struct_internal(37)
    ready_replicas: int = struct_internal(38)
    last_active_at: datetime = struct_internal(39, default_factory=utcnow_with_tz)
    last_bumped_at: datetime = struct_internal(40, default_factory=utcnow_with_tz)


@struct(StructType.WORKER_IMAGE)
class WorkerImage(Struct):
    language: str = struct_internal(30)
    version: str = struct_internal(31)
    platform: str = struct_internal(32)
    dependencies: list["Dependency"] = struct_internal(33, struct_t=StructType.DEPENDENCY)


@struct(StructType.DEPENDENCY)
class Dependency(Struct):
    name: str = struct_internal(30)
    version: str = struct_internal(31)
