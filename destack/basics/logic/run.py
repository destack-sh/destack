from datetime import timedelta
from typing import TYPE_CHECKING, Optional

from destack.core import (
    Entity,
    EnumType,
    Event,
    NodeType,
    OptionEnum,
    ReferenceType,
    Timestamp,
    declare_entity,
    declare_enum,
    declare_event,
    declare_option,
    declare_property,
)

if TYPE_CHECKING:
    from destack import Action


@declare_enum(EnumType.RUN_STATUS)
class RunStatus(OptionEnum):
    # pre
    SCHEDULED = declare_option(2, "Scheduled", description="Scheduled for sometime")
    # active
    RUNNING = declare_option(10, "Running", description="Actively running")
    # interrupted
    PAUSED = declare_option(21, "Paused", description="Paused manually")
    YIELDED = declare_option(23, "Yielded", description="Yielded to someone")
    # terminal
    CANCELLED = declare_option(51, "Cancelled", description="Cancelled before running")
    ABORTED = declare_option(52, "Aborted", description="Aborted while running")
    FAILED = declare_option(53, "Failed", description="Failed due to an error")
    COMPLETED = declare_option(54, "Completed", description="Completed successfully")

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


@declare_event(NodeType.RUN_EVENT, is_abstract=True)
class RunEvent(Event):
    """An Event regarding a Run."""

    run: "Run" = declare_property(
        101,
        reference_type=ReferenceType.LOCATION,
        tag=None,
    )
    target: Optional["Entity"] = declare_property(
        110,
        reference_type=ReferenceType.LOCATION,
        tag=None,
    )


@declare_event(NodeType.RUN_STARTED_EVENT)
class RunStartedEvent(
    RunEvent,
):
    """A Run was started."""

    pass


@declare_event(NodeType.RUN_PAUSE_REQUESTED_EVENT)
class RunPauseRequestedEvent(RunEvent):
    """A Run was paused."""

    pass


@declare_event(NodeType.RUN_PAUSED_EVENT)
class RunPausedEvent(RunEvent):
    """A Run was paused."""

    pass


@declare_event(NodeType.RUN_RESUME_REQUESTED_EVENT)
class RunResumeRequestedEvent(RunEvent):
    """A Run was resumed."""

    pass


@declare_event(NodeType.RUN_RESUMED_EVENT)
class RunResumedEvent(RunEvent):
    """A Run was resumed."""

    pass


@declare_event(NodeType.RUN_STOP_REQUESTED_EVENT)
class RunStopRequestedEvent(RunEvent):
    """A Run was stopped."""

    pass


@declare_event(NodeType.RUN_FAILED_EVENT)
class RunFailedEvent(RunEvent):
    """A Run failed."""

    pass


@declare_event(NodeType.RUN_COMPLETED_EVENT)
class RunCompletedEvent(RunEvent):
    """A Run completed."""

    pass


@declare_entity(
    NodeType.RUN,
    event_types=(NodeType.RUN_EVENT,),
    is_abstract=True,
)
class Run(Entity):
    """
    Run of an Action.
    """

    action: "Action" = declare_property(
        111,
        reference_type=ReferenceType.LOCATION,
        tag=None,
    )
    status: RunStatus = declare_property(
        112,
        is_repr=True,
        tag=None,
    )
    duration: Optional[timedelta] = declare_property(
        113,
        default=None,
        description="Duration from first attempt start to last attempt termination.",
        is_repr=True,
        tag=None,
    )
    scheduled_at: Optional[Timestamp] = declare_property(
        116,
        description="When the Run is scheduled to start.",
        tag=None,
    )
    started_at: Optional[Timestamp] = declare_property(
        117,
        description="When the Run first started.",
        is_repr=True,
        tag=None,
    )
    seen_at: Optional[Timestamp] = declare_property(
        118,
        description="When the Run was last active.",
        tag=None,
    )
    interrupted_at: Optional[Timestamp] = declare_property(
        119,
        description="When the Run was interrupted.",
        tag=None,
    )
    terminated_at: Optional[Timestamp] = declare_property(
        120,
        description="When the Run was last terminated.",
        tag=None,
    )
