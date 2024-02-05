from datetime import datetime
from typing import Optional

from bench.language.const import NodeType, StructType, ServerProfile, ServerStatus
from bench.language.node import (
    Bench,
    Node,
    ScopeNode,
    Struct,
    node,
    p_parent,
    struct,
    p_internal,
    p_regular,
    p_system,
)
from bench.utils.dt import utcnow_with_tz


@struct(StructType.SERVER_ALLOCATION)
class ServerAllocation(Struct):
    default_profile: ServerProfile = p_regular(30)
    default_image: Optional["ServerImage"] = p_regular(31, struct=StructType.SERVER_IMAGE)


@node(NodeType.SERVER)
class Server(Node):
    parent: "Bench" = p_parent(4, NodeType.BENCH)

    target_profile: ServerProfile = p_regular(30)
    target_image: Optional["ServerImage"] = p_regular(31, struct=StructType.SERVER_IMAGE)
    target_version: Optional[str] = p_system(32, index_in_pg=True)
    current_profile: Optional[ServerProfile] = p_system(33)
    current_image: Optional["ServerImage"] = p_system(34, struct=StructType.SERVER_IMAGE)
    current_version: Optional[str] = p_system(35, index_in_pg=True)

    sleep: bool = p_system(36, default=True)
    status: ServerStatus = p_system(37)
    last_active_at: Optional[datetime] = p_internal(38, default_factory=utcnow_with_tz)
    last_bumped_at: Optional[datetime] = p_internal(39, default_factory=utcnow_with_tz)

    external_id: Optional[str] = p_system(50, unique=True)
    access_token: Optional[str] = p_system(
        51, default=None, encrypt=True, defer=True, sensitive=True
    )


@struct(StructType.SERVER_IMAGE)
class ServerImage(Struct):
    language: str = p_regular(30)
    version: str = p_regular(31)
    platform: str = p_regular(32)
    dependencies: list["ServerImageDependency"] = p_regular(
        33, struct=StructType.SERVER_IMAGE_DEPENDENCY
    )


@struct(StructType.SERVER_IMAGE_DEPENDENCY)
class ServerImageDependency(Struct):
    name: str = p_regular(30)
    version: str = p_regular(31)
