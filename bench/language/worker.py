from datetime import datetime
from typing import Optional

from bench.language.const import NodeType, ProjectRegion, StructType, WorkerProfile, WorkerSetStatus
from bench.language.module import (
    Bench,
    Node,
    ScopeNode,
    Struct,
    node,
    node_children,
    node_parent,
    struct,
    struct_internal,
)
from bench.utils.dt import utcnow_with_tz


@node(NodeType.WORKER_SET)
class WorkerSet(ScopeNode):
    """
    Set of workers to run a Bench's modules.
    """

    parent: "Bench" = node_parent(4, NodeType.BENCH)
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

    workers: list["Worker"] = node_children(NodeType.WORKER)


@node(NodeType.WORKER)
class Worker(Node):
    parent: "WorkerSet" = node_parent(4, NodeType.WORKER_SET)
    external_id: str = struct_internal(30)
    profile: WorkerProfile = struct_internal(31)
    image: Optional["WorkerImage"] = struct_internal(32, struct_t=StructType.WORKER_IMAGE)


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
