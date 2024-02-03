from datetime import datetime
from typing import Optional

from bench.language.const import NodeType, StructType, WorkerProfile, WorkerSetStatus
from bench.language.node import (
    Bench,
    Node,
    ScopeNode,
    Struct,
    node,
    p_child,
    p_parent,
    struct,
    p_internal,
    p_tracked,
    p_system,
)
from bench.utils.dt import utcnow_with_tz


@node(NodeType.WORKER_SET)
class WorkerSet(ScopeNode):
    """
    Set of workers to run a Bench's packages.
    """

    parent: "Bench" = p_parent(4, NodeType.BENCH)
    profile: WorkerProfile = p_tracked(31)
    sleeping: bool = p_system(32)
    status: WorkerSetStatus = p_system(33)
    desired_replicas: int = p_tracked(34)
    target_replicas: int = p_system(35)
    available_replicas: int = p_system(36)
    ready_replicas: int = p_system(37)
    last_active_at: Optional[datetime] = p_internal(38, default_factory=utcnow_with_tz)
    last_bumped_at: Optional[datetime] = p_internal(39, default_factory=utcnow_with_tz)

    workers: list["Worker"] = p_child(NodeType.WORKER)


@node(NodeType.WORKER)
class Worker(Node):
    parent: "WorkerSet" = p_parent(4, NodeType.WORKER_SET)
    external_id: str = p_system(30, unique=True)
    profile: WorkerProfile = p_system(31)
    image: Optional["WorkerImage"] = p_system(32, struct=StructType.WORKER_IMAGE)
    version: Optional[str] = p_system(33, index_in_pg=True)
    access_token: Optional[str] = p_system(
        34, default=None, encrypt=True, defer=True, sensitive=True
    )


@struct(StructType.WORKER_IMAGE)
class WorkerImage(Struct):
    language: str = p_internal(30)
    version: str = p_internal(31)
    platform: str = p_internal(32)
    dependencies: list["Dependency"] = p_internal(33, struct=StructType.DEPENDENCY)


@struct(StructType.DEPENDENCY)
class Dependency(Struct):
    name: str = p_internal(30)
    version: str = p_internal(31)
