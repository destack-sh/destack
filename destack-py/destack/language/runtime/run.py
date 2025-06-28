from datetime import datetime, timedelta
from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    Event,
    IsExtensible,
    IsRunnable,
    Node,
    NodeType,
    Spatial,
    builtin_enum,
    builtin_node,
    property_,
    property_parent_,
)
from destack.proto import (
    RunCompletedEventProto,
    RunFailedEventProto,
    RunPausedEventProto,
    RunPauseRequestedEventProto,
    RunProto,
    RunResumedEventProto,
    RunResumeRequestedEventProto,
    RunStartedEventProto,
    RunStopRequestedEventProto,
)

if TYPE_CHECKING:
    from destack.language import Interruption, NodeReference, Space


# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.RUN_STATUS)
class RunStatus(Enum):
    # pre
    SCHEDULED = 2, "Scheduled", "Scheduled for sometime", "fas fa-clock"
    # active
    RUNNING = 10, "Running", "Actively running", "fas fa-circle-notch"
    # interrupted
    PAUSED = 21, "Paused", "Paused manually", "fas fa-circle-pause"
    YIELDED = 23, "Yielded", "Yielded to someone", "fas fa-circle-pause"
    # terminal
    CANCELLED = 51, "Cancelled", "Cancelled before running", "fas fa-circle-xmark"
    ABORTED = 52, "Aborted", "Aborted while running", "fas fa-circle-xmark"
    FAILED = 53, "Failed", "Failed due to an error", "fas fa-circle-xmark"
    COMPLETED = 54, "Completed", "Completed successfully", "fas fa-circle-check"

    @property
    def is_pre(self) -> bool:
        return self < 10

    @property
    def is_active(self) -> bool:
        return self >= 10 and self < 20

    @property
    def is_interrupted(self) -> bool:
        return self >= 20 and self < 30

    @property
    def is_inactive(self) -> bool:
        return self >= 30 and self < 40

    @property
    def is_terminal(self) -> bool:
        return self >= 50

    @property
    def is_bad(self) -> bool:
        return self in (RunStatus.FAILED, RunStatus.ABORTED, RunStatus.CANCELLED)


@builtin_node(NodeType.RUN_STARTED_EVENT)
class RunStartedEvent(
    Event["Run"],
    Node[RunStartedEventProto],
):
    """An Event regarding a Run."""

    node: "Run" = property_(35)
    target: Optional[IsRunnable] = property_(40)
    if TYPE_CHECKING:
        target_ptr: Optional[NodeReference] = None


@builtin_node(NodeType.RUN_PAUSE_REQUESTED_EVENT)
class RunPauseRequestedEvent(
    Event["Run"],
    Node[RunPauseRequestedEventProto],
):
    """An Event regarding a Run."""

    node: "Run" = property_(35)
    target: Optional[IsRunnable] = property_(40)
    if TYPE_CHECKING:
        target_ptr: Optional[NodeReference] = None


@builtin_node(NodeType.RUN_PAUSED_EVENT)
class RunPausedEvent(
    Event["Run"],
    Node[RunPausedEventProto],
):
    """A Event regarding a Run."""

    node: "Run" = property_(35)
    target: Optional[IsRunnable] = property_(40)
    if TYPE_CHECKING:
        target_ptr: Optional[NodeReference] = None


@builtin_node(NodeType.RUN_RESUME_REQUESTED_EVENT)
class RunResumeRequestedEvent(
    Event["Run"],
    Node[RunResumeRequestedEventProto],
):
    """An Event regarding a Run."""

    node: "Run" = property_(35)
    target: Optional[IsRunnable] = property_(40)
    if TYPE_CHECKING:
        target_ptr: Optional[NodeReference] = None


@builtin_node(NodeType.RUN_RESUMED_EVENT)
class RunResumedEvent(
    Event["Run"],
    Node[RunResumedEventProto],
):
    """A Event regarding a Run."""

    node: "Run" = property_(35)
    target: Optional[IsRunnable] = property_(40)
    if TYPE_CHECKING:
        target_ptr: Optional[NodeReference] = None


@builtin_node(NodeType.RUN_STOP_REQUESTED_EVENT)
class RunStopRequestedEvent(
    Event["Run"],
    Node[RunStopRequestedEventProto],
):
    """An Event regarding a Run."""

    node: "Run" = property_(35)
    target: Optional[IsRunnable] = property_(40)
    if TYPE_CHECKING:
        target_ptr: Optional[NodeReference] = None


@builtin_node(NodeType.RUN_FAILED_EVENT)
class RunFailedEvent(
    Event["Run"],
    Node[RunFailedEventProto],
):
    """An Event regarding a Run."""

    node: "Run" = property_(35)
    target: Optional[IsRunnable] = property_(40)
    if TYPE_CHECKING:
        target_ptr: Optional[NodeReference] = None


@builtin_node(NodeType.RUN_COMPLETED_EVENT)
class RunCompletedEvent(
    Event["Run"],
    Node[RunCompletedEventProto],
):
    """An Event regarding a Run."""

    node: "Run" = property_(35)
    target: Optional[IsRunnable] = property_(40)
    if TYPE_CHECKING:
        target_ptr: Optional[NodeReference] = None


@builtin_node(NodeType.RUN)
class Run(
    Spatial,
    Entity,
    IsExtensible,
    Node[RunProto],
):
    """
    Run something somewhere, somehow.
    """

    parent: Optional["Space"] = property_parent_(node_is_customizable=False)
    target: Optional[IsRunnable] = property_(40)
    if TYPE_CHECKING:
        target_ptr: Optional[NodeReference] = None
    status: RunStatus = property_(41, is_repr=True)
    duration: Optional[timedelta] = property_(
        42,
        default=None,
        description="Duration from first attempt start to last attempt termination.",
        is_repr=True,
    )
    scheduled_at: Optional[datetime] = property_(
        45, description="When the Run is scheduled to start."
    )
    started_at: Optional[datetime] = property_(
        46, description="When the Run first started.", is_repr=True
    )
    seen_at: Optional[datetime] = property_(47, description="When the Run was last active.")
    interrupted_at: Optional[datetime] = property_(48, description="When the Run was interrupted.")
    terminated_at: Optional[datetime] = property_(
        49, description="When the Run was last terminated."
    )
    interruption: Optional["Interruption"] = property_(
        51,
        node_space_from="self",
        description="The latest Interruption.",
        is_repr=True,
    )
    if TYPE_CHECKING:
        interruption_ptr: Optional[NodeReference] = None
