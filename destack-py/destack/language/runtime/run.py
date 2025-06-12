from datetime import datetime, timedelta
from typing import TYPE_CHECKING, Optional

from fastuuid import UUID

from destack.language.core import (
    Analytic,
    Indexed,
    IsExtensible,
    IsRunnable,
    Node,
    NodeType,
    Particle,
    RunStatus,
    RunType,
    Spatial,
    node_,
    property_,
    property_parent_,
)
from destack.pb2 import RunData

if TYPE_CHECKING:
    from destack.language import Error, Interruption, NodeReference, Space


# pyright: reportIncompatibleVariableOverride=false


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
    status: RunStatus = property_(41, default=RunStatus.CREATED, is_repr=True)
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
    active_at: Optional[datetime] = property_(47, description="When the Run was last active.")
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
