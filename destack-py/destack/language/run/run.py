from datetime import datetime, timedelta
from typing import TYPE_CHECKING, Optional

from fastuuid import UUID

from destack.language.core import (
    IsEnvironmental,
    IsExtensible,
    IsInFolder,
    IsParticle,
    IsRunnable,
    Node,
    NodeType,
    RunStatus,
    RunType,
    node_,
    property_,
)
from destack.pb2 import RunData

if TYPE_CHECKING:
    from destack.language import Error, Interruption, NodeReference, Thread


# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.RUN)
class Run(
    IsEnvironmental,
    IsExtensible,
    IsInFolder,
    IsParticle,
    Node[RunData],
):
    """
    Run something somewhere, somehow.
    """

    # meta
    type: RunType = property_(30, can_write="system", is_repr=True)
    thread: Optional["Thread"] = property_(
        38,
        node_space_from="self",
        description="The Thread to communicate with the Run. May be shared with other Runs.",
    )
    if TYPE_CHECKING:
        thread_ptr: Optional[NodeReference] = None
        thread_id: Optional[UUID] = None

    # content
    runnable: Optional[IsRunnable] = property_(67)
    if TYPE_CHECKING:
        runnable_ptr: Optional[NodeReference] = None
        runnable_id: Optional[UUID] = None

    status: RunStatus = property_(80, default=RunStatus.CREATED, is_repr=True)
    duration: Optional[timedelta] = property_(
        81,
        default=None,
        description="Duration from first attempt start to last attempt termination.",
        is_repr=True,
    )
    error: Optional["Error"] = property_(82, is_repr=True)
    interruption: Optional["Interruption"] = property_(
        83,
        node_space_from="self",
        description="The latest Interruption concerning the Node.",
        is_repr=True,
    )
    scheduled_at: Optional[datetime] = property_(
        85, description="When the Node is scheduled to start."
    )
    started_at: Optional[datetime] = property_(
        86, description="When the Node first started.", is_repr=True
    )
    active_at: Optional[datetime] = property_(87, description="When the Node was last active.")
    interrupted_at: Optional[datetime] = property_(88, description="When the Node was interrupted.")
    terminated_at: Optional[datetime] = property_(
        89, description="When the Node was last terminated."
    )
    requested_stop_at: Optional[datetime] = property_(
        90, description="When the Node was requested to stop."
    )
    requested_pause_at: Optional[datetime] = property_(
        91, description="When the Node was requested to pause."
    )
    requested_resume_at: Optional[datetime] = property_(
        92, description="When the Node was requested to resume."
    )
    if TYPE_CHECKING:
        interruption_ptr: Optional[NodeReference] = None
        interruption_id: Optional[UUID] = None

    def touch(self) -> None:
        """'Touch' the Node to update the active_at timestamp."""
        self.active_at = self._session.oracle.utc()

    @property
    def should_stop(self) -> bool:
        return not (self.status.is_terminal) and (self.requested_stop_at is not None)

    @property
    def should_pause(self) -> bool:
        return not (self.status.is_terminal or self.requested_stop_at is not None) and (
            self.requested_pause_at is not None
            and (
                self.requested_resume_at is None
                or self.requested_pause_at > self.requested_resume_at
            )
        )

    @property
    def should_resume(self) -> bool:
        return not (self.status.is_terminal or self.requested_stop_at is not None) and (
            self.requested_resume_at is not None
            and (
                self.requested_pause_at is None
                or self.requested_pause_at < self.requested_resume_at
            )
            and (self.interrupted_at is None or self.interrupted_at < self.requested_resume_at)
        )

    def pause(self):
        """Mark this Run as paused."""
        assert self._session is not None, f"{self!r} has no session"
        self.requested_pause_at = self._session.oracle.utc()

    def resume(self, _trigger_runtime: bool = True):
        """Mark this Run as resumed."""
        assert self._session is not None, f"{self!r} has no session"
        self.requested_resume_at = self._session.oracle.utc()

    def stop(self, _trigger_runtime: bool = True):
        """Mark this Run as stopped."""
        assert self._session is not None, f"{self!r} has no session"
        self.requested_stop_at = self._session.oracle.utc()

    cancel = abort = stop
