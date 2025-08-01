from datetime import datetime, timedelta
from typing import TYPE_CHECKING, Optional, Union

from destack.language.core import (
    Action,
    Entity,
    Enum,
    EnumType,
    Event,
    NodeType,
    builtin_entity,
    builtin_enum,
    builtin_event,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import NodeReference


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


@builtin_event(NodeType.RUN_EVENT, is_abstract=True)
class RunEvent(Event):
    """An Event regarding a Run."""

    run: "Run" = builtin_property(101)
    target: Optional["Entity"] = builtin_property(110)
    if TYPE_CHECKING:
        target_ptr: Optional[NodeReference] = None


@builtin_event(NodeType.RUN_STARTED_EVENT)
class RunStartedEvent(
    RunEvent,
):
    """A Run was started."""

    pass


@builtin_event(NodeType.RUN_PAUSE_REQUESTED_EVENT)
class RunPauseRequestedEvent(RunEvent):
    """A Run was paused."""

    pass


@builtin_event(NodeType.RUN_PAUSED_EVENT)
class RunPausedEvent(RunEvent):
    """A Run was paused."""

    pass


@builtin_event(NodeType.RUN_RESUME_REQUESTED_EVENT)
class RunResumeRequestedEvent(RunEvent):
    """A Run was resumed."""

    pass


@builtin_event(NodeType.RUN_RESUMED_EVENT)
class RunResumedEvent(RunEvent):
    """A Run was resumed."""

    pass


@builtin_event(NodeType.RUN_STOP_REQUESTED_EVENT)
class RunStopRequestedEvent(RunEvent):
    """A Run was stopped."""

    pass


@builtin_event(NodeType.RUN_FAILED_EVENT)
class RunFailedEvent(RunEvent):
    """A Run failed."""

    pass


@builtin_event(NodeType.RUN_COMPLETED_EVENT)
class RunCompletedEvent(RunEvent):
    """A Run completed."""

    pass


@builtin_entity(
    NodeType.RUN,
    event_types=(NodeType.RUN_EVENT,),
    is_abstract=True,
)
class Run(Entity):
    """
    Run of an Action.
    """

    parent: Union["Action", "Run", None] = builtin_property_parent()
    action: "Action" = builtin_property(111)
    status: RunStatus = builtin_property(112, is_repr=True)
    duration: Optional[timedelta] = builtin_property(
        113,
        default=None,
        description="Duration from first attempt start to last attempt termination.",
        is_repr=True,
    )
    scheduled_at: Optional[datetime] = builtin_property(
        116, description="When the Run is scheduled to start."
    )
    started_at: Optional[datetime] = builtin_property(
        117, description="When the Run first started.", is_repr=True
    )
    seen_at: Optional[datetime] = builtin_property(118, description="When the Run was last active.")
    interrupted_at: Optional[datetime] = builtin_property(
        119, description="When the Run was interrupted."
    )
    terminated_at: Optional[datetime] = builtin_property(
        120, description="When the Run was last terminated."
    )
    if TYPE_CHECKING:
        target_ptr: Optional[NodeReference] = None
