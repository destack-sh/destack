from datetime import datetime
from typing import Optional

from bench.language.const import NodeType, StructType, WorkerProfile, WorkerSetStatus
from bench.language.node import (
    Bench,
    Node,
    ScopeNode,
    Struct,
    node,
    node_children,
    node_parent,
    struct,
    struct_internal,
    struct_property,
)
from bench.utils.dt import utcnow_with_tz


@node(NodeType.WORKER_SET)
class WorkerSet(ScopeNode):
    """
    Set of workers to run a Bench's packages.
    """

    parent: "Bench" = node_parent(4, NodeType.BENCH)
    profile: WorkerProfile = struct_property(31)
    sleeping: bool = struct_internal(32, system=True)
    status: WorkerSetStatus = struct_internal(33, system=True)
    desired_replicas: int = struct_property(34)
    target_replicas: int = struct_internal(35, system=True)
    available_replicas: int = struct_internal(36, system=True)
    ready_replicas: int = struct_internal(37, system=True)
    last_active_at: Optional[datetime] = struct_internal(
        38, system=True, default_factory=utcnow_with_tz
    )
    last_bumped_at: Optional[datetime] = struct_internal(
        39, system=True, default_factory=utcnow_with_tz
    )

    workers: list["Worker"] = node_children(NodeType.WORKER)


@node(NodeType.WORKER)
class Worker(Node):
    parent: "WorkerSet" = node_parent(4, NodeType.WORKER_SET)
    external_id: str = struct_internal(30, unique=True, system=True)
    profile: WorkerProfile = struct_internal(31)
    image: Optional["WorkerImage"] = struct_internal(32, struct=StructType.WORKER_IMAGE)
    version: Optional[str] = struct_internal(33, index_in_pg=True)
    access_token: Optional[str] = struct_internal(
        34, default=None, system=True, encrypt=True, defer=True, sensitive=True
    )


@struct(StructType.WORKER_IMAGE)
class WorkerImage(Struct):
    language: str = struct_internal(30)
    version: str = struct_internal(31)
    platform: str = struct_internal(32)
    dependencies: list["Dependency"] = struct_internal(33, struct=StructType.DEPENDENCY)


@struct(StructType.DEPENDENCY)
class Dependency(Struct):
    name: str = struct_internal(30)
    version: str = struct_internal(31)
