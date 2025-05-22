from datetime import datetime, timedelta
from typing import TYPE_CHECKING, Optional, Union

from fastuuid import UUID

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    IsClaimable,
    IsInPackage,
    IsInstantiable,
    IsModal,
    IsNamed,
    IsOrdered,
    IsOwnable,
    Node,
    NodeReference,
    NodeType,
    enum_,
    node_,
    p_internal,
    p_node_parent,
    p_regular,
)
from bench.pb2 import ClaimData

if TYPE_CHECKING:
    from bench.language import Action, Agent, Flow, Run, Service, Thread

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.CLAIM_TYPE)
class ClaimType(BuiltinEnum):
    READ = 20, "Read", "Can read", "fas fa-eye"
    # RESERVED? (read but may promote to write)
    WRITE = 40, "Write", "Can write", "fas fa-pencil"
    # EXCLUSIVE_WRITE?


@enum_(EnumType.CLAIM_STATUS)
class ClaimStatus(BuiltinEnum):
    # pre
    REQUESTED = 1, "Pending", "Pending", "fas fa-clock"
    # active
    OPEN = 10, "Active", "Active concurrent access", "fas fa-lock-open"
    # inactive
    PAUSED = 20, "Paused", "Paused", "fas fa-pause"
    # terminal
    CLOSED = 30, "Closed", "Closed", "fas fa-power-off"

    @property
    def is_pending(self) -> bool:
        return self < 10

    @property
    def is_active(self) -> bool:
        return self >= 10 and self < 20

    @property
    def is_inactive(self) -> bool:
        return self >= 20 and self < 30

    @property
    def is_terminal(self) -> bool:
        return self >= 30


@node_(NodeType.CLAIM)
class Claim(
    IsOwnable,
    IsModal,
    IsInstantiable,
    IsOrdered,
    IsNamed,
    IsInPackage,
    Node[ClaimData],
):
    """
    A Claim on something (like a Resource, runnable tool Node for a 'tool', or some other Node).
    Depending on the claim, a Claim may be instantiated and granted/rejected at runtime.
    """

    # meta
    parent: Union["Service", "Flow", "Action", "Agent", "Thread", "Run", None] = p_node_parent(4)
    type: ClaimType = p_regular(30)

    # status
    status: ClaimStatus = p_regular(50, default=ClaimStatus.REQUESTED)
    duration: Optional[timedelta] = p_internal(51)
    opened_at: Optional[datetime] = p_internal(52)
    paused_at: Optional[datetime] = p_internal(53)
    resumed_at: Optional[datetime] = p_internal(54)
    terminated_at: Optional[datetime] = p_internal(55)

    # content
    target: Optional[IsClaimable] = p_regular(
        60,
        description="The target Node this claim is about.",
    )
    target_template: Optional[IsClaimable] = p_regular(
        61,
        description="The template for a target Node.",
    )
    # target_selection/filter/....?
    is_hidden: bool = p_regular(65, default=False)
    if TYPE_CHECKING:
        target_ptr: Optional[NodeReference] = None
        target_id: Optional[UUID] = None
        target_template_ptr: Optional[NodeReference] = None
        target_template_id: Optional[UUID] = None

    @staticmethod
    def read(name: str, target: IsClaimable, **kwargs) -> "Claim":
        claim = Claim(type=ClaimType.READ, name=name, target=target, **kwargs)
        return claim

    @staticmethod
    def write(name: str, target: IsClaimable, **kwargs) -> "Claim":
        claim = Claim(type=ClaimType.WRITE, name=name, target=target, **kwargs)
        return claim
