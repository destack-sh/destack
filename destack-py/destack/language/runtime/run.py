from datetime import datetime, timedelta
from typing import TYPE_CHECKING, Optional

from fastuuid import UUID

from destack.language.core import (
    Analytic,
    BuiltinEnum,
    EnumType,
    Event,
    Indexed,
    IsExtensible,
    IsRunnable,
    Node,
    NodeType,
    Particle,
    RunType,
    Spatial,
    enum_,
    node_,
    property_,
    property_parent_,
)
from destack.pb2 import RunData, RunEventData

if TYPE_CHECKING:
    from destack.language import Error, Interruption, NodeReference, Space


# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.RUN_STATUS)
class RunStatus(BuiltinEnum):
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


@enum_(EnumType.RUN_EVENT_TYPE)
class RunEventType(BuiltinEnum):
    """A Type of Run Event."""

    SCHEDULED = 1, "Scheduled", "Scheduled for sometime", "fas fa-clock"
    RUNNING = 10, "Running", "Actively running", "fas fa-circle-notch"
    REQUESTED_PAUSE = 20, "Requested Pause", "Requested to pause", "fas fa-circle-pause"
    PAUSED = 21, "Paused", "Paused manually", "fas fa-circle-pause"
    REQUESTED_RESUME = 22, "Requested Resume", "Requested to resume", "fas fa-circle-pause"
    RESUMED = 23, "Resumed", "Resumed manually", "fas fa-circle-pause"
    REQUESTED_CANCEL = 50, "Requested Cancel", "Requested to cancel", "fas fa-circle-xmark"
    CANCELLED = 51, "Cancelled", "Cancelled before running", "fas fa-circle-xmark"
    ABORTED = 52, "Aborted", "Aborted while running", "fas fa-circle-xmark"
    FAILED = 53, "Failed", "Failed due to an error", "fas fa-circle-xmark"
    COMPLETED = 54, "Completed", "Completed successfully", "fas fa-circle-check"


@node_(NodeType.RUN_EVENT)
class RunEvent(
    Event["Run"],
    Node[RunEventData],
):
    """A Event regarding a Run."""

    type: RunEventType = property_(30)
    node: "Run" = property_(35)
    target: Optional[IsRunnable] = property_(40)
    if TYPE_CHECKING:
        target_ptr: Optional[NodeReference] = None
        target_id: Optional[UUID] = None
        target_type: Optional[NodeType] = None


@node_(NodeType.RUN)
class Run(
    Spatial,
    Particle,
    Analytic,
    Indexed,
    IsExtensible,
    Node[RunData],
):
    """
    Run something somewhere, somehow.
    """

    # meta
    parent: Optional["Space"] = property_parent_(node_is_customizable=False)
    type: RunType = property_(30, can_write="system", is_repr=True)

    # content
    target: Optional[IsRunnable] = property_(40)
    if TYPE_CHECKING:
        target_ptr: Optional[NodeReference] = None
        target_id: Optional[UUID] = None
        target_type: Optional[NodeType] = None
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
    error: Optional["Error"] = property_(50, is_repr=True)
    interruption: Optional["Interruption"] = property_(
        51,
        node_space_from="self",
        description="The latest Interruption.",
        is_repr=True,
    )
    if TYPE_CHECKING:
        interruption_ptr: Optional[NodeReference] = None
        interruption_id: Optional[UUID] = None
