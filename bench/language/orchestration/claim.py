from datetime import datetime, timedelta
from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.language.core import (
    CLAIMABLE_NODE_TYPES,
    BuiltinEnum,
    Claimable,
    ColorType,
    EnumType,
    IsInstantiable,
    IsModal,
    IsNamed,
    IsOwnable,
    IsRuntime,
    NodeReference,
    NodeType,
    PackageNode,
    enum_,
    node_,
    p_internal,
    p_node_parent,
    p_regular,
)
from bench.pb2 import ClaimData
from bench.utils.fractional import INTEGER_ZERO

if TYPE_CHECKING:
    from bench.language import Action, Agent, Flow, Kit, Run, Thread

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.CLAIM_TYPE)
class ClaimType(BuiltinEnum):
    READ = 20, "Read", "Can read", "fas fa-eye", ColorType.GRAY
    # RESERVED? (read but may promote to write)
    WRITE = 40, "Write", "Can write", "fas fa-pencil", ColorType.BLUE
    # EXCLUSIVE_WRITE?


@enum_(EnumType.CLAIM_STATUS)
class ClaimStatus(BuiltinEnum):
    # pre
    REQUESTED = 1, "Pending", "Pending", "fas fa-clock", ColorType.BLUE
    # active
    OPEN = 10, "Active", "Active concurrent access", "fas fa-lock-open", ColorType.GREEN
    # inactive
    PAUSED = 20, "Paused", "Paused", "fas fa-pause", ColorType.PINK
    # terminal
    CLOSED = 30, "Closed", "Closed", "fas fa-power-off", ColorType.GRAY

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
    IsRuntime,
    IsOwnable,
    IsModal,
    IsInstantiable,
    IsNamed,
    PackageNode[ClaimData],
):
    """
    A Claim on something (like a Resource, runnable tool Node for a 'tool', or some other Node).
    Depending on the claim, a Claim may be instantiated and granted/rejected at runtime.
    """

    # meta
    parent: Union["Kit", "Flow", "Action", "Agent", "Thread", "Run", None] = p_node_parent(
        4,
        NodeType.KIT,
        NodeType.FLOW,
        NodeType.ACTION,
        NodeType.AGENT,
        NodeType.ROLE,
        NodeType.THREAD,
        NodeType.RUN,
        ckless=True,
    )
    type: ClaimType = p_regular(30, require=True)
    order_key: str = p_internal(33, default=INTEGER_ZERO)

    # status
    status: ClaimStatus = p_regular(50, default=ClaimStatus.REQUESTED)
    duration: Optional[timedelta] = p_internal(51, default=None)
    opened_at: Optional[datetime] = p_internal(52, default=None)
    paused_at: Optional[datetime] = p_internal(53, default=None)
    resumed_at: Optional[datetime] = p_internal(54, default=None)
    terminated_at: Optional[datetime] = p_internal(55, default=None)

    # content
    target: Optional[Claimable] = p_regular(
        60,
        require=False,
        array=False,
        references=CLAIMABLE_NODE_TYPES.tuple,
        description="The target Node this claim is about.",
    )
    target_template: Optional[Claimable] = p_regular(
        65,
        require=False,
        array=False,
        references=CLAIMABLE_NODE_TYPES.tuple,
        description="The template for a target Node.",
    )
    # target_selection/filter/....?
    if TYPE_CHECKING:
        target_ptr: Optional[NodeReference] = None
        target_id: Optional[UUID] = None
        target_template_ptr: Optional[NodeReference] = None
        target_template_id: Optional[UUID] = None

    @staticmethod
    def read(name: str, target: Claimable, **kwargs) -> "Claim":
        claim = Claim(type=ClaimType.READ, name=name, target=target, **kwargs)
        return claim

    @staticmethod
    def write(name: str, target: Claimable, **kwargs) -> "Claim":
        claim = Claim(type=ClaimType.WRITE, name=name, target=target, **kwargs)
        return claim
