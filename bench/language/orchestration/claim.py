from datetime import datetime, timedelta
from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.language.core import (
    RESOURCE_NODE_TYPES,
    BuiltinEnum,
    ColorType,
    EnumType,
    IsInstantiable,
    IsOwnable,
    IsRuntime,
    IsTraceable,
    NodeReference,
    NodeType,
    PackageNode,
    Selection,
    enum_,
    node_,
    p_internal,
    p_node_parent,
    p_regular,
    p_system,
)
from bench.language.core.const import StructType
from bench.pb2 import ClaimData

if TYPE_CHECKING:
    from bench.language import Action, Flow, Kit, Page, Resource, Run

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.CLAIM_TYPE)
class ClaimType(BuiltinEnum):
    SHARED = 10, "Shared", "Shared concurrent access", "fas fa-users"
    RESERVED = 20, "Reserved", "Shared, but reserved for exclusive use", "fas fa-user-unlock"
    EXCLUSIVE = 30, "Exclusive", "Exclusive access", "fas fa-lock"


@enum_(EnumType.CLAIM_STATUS)
class ClaimStatus(BuiltinEnum):
    # pre
    PENDING = 10, "Pending", "Pending", "fas fa-clock", ColorType.GRAY
    # open
    ACTIVE = 20, "Active", "Active concurrent access", "fas fa-lock", ColorType.GREEN
    # closed
    CANCELLED = 30, "Cancelled", None, "fas fa-circle-xmark", ColorType.RED
    REJECTED = 31, "Rejected", None, "fas fa-circle-exclamation", ColorType.RED
    RELEASED = 32, "Released", None, "fas fa-circle-check", ColorType.GRAY


@node_(NodeType.CLAIM)
class Claim(IsRuntime, IsOwnable, IsTraceable, IsInstantiable, PackageNode[ClaimData]):
    """A Claim on a Resource (which may be granted/rejected and is eventually closed)."""

    # meta
    parent: Union["Page", "Kit", "Flow", "Action", "Run", None] = p_node_parent(
        4, NodeType.PAGE, NodeType.KIT, NodeType.FLOW, NodeType.ACTION, NodeType.RUN, ckless=True
    )
    type: ClaimType = p_system(30, require=True)

    # status
    status: ClaimStatus = p_regular(50, default=ClaimStatus.PENDING)
    duration: Optional[timedelta] = p_internal(51, default=None)
    opened_at: Optional[datetime] = p_internal(52, default=None)
    granted_at: Optional[datetime] = p_internal(53, default=None)
    closed_at: Optional[datetime] = p_internal(54, default=None)

    # content
    resource: Optional["Resource"] = p_regular(
        60,
        require=False,
        array=False,
        references=RESOURCE_NODE_TYPES.tuple,
        description="The Resource this claim is on.",
    )
    resource_selection: Optional["Selection"] = p_regular(
        65,
        require=False,
        array=False,
        struct=StructType.SELECTION,
        description="The potential Resources this claim is on.",
    )
    if TYPE_CHECKING:
        resource_ptr: Optional[NodeReference] = None
        resource_id: Optional[UUID] = None
