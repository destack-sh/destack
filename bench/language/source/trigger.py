from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union

from bench.language.core import (
    NAME_CONSTRAINT,
    BuiltinEnum,
    ColorType,
    EnumType,
    IsModal,
    IsTemplatable,
    NodeType,
    PackageNode,
    StructType,
    Text,
    enum_,
    node_,
    p_node_parent,
    p_regular,
    p_system,
)
from bench.pb2 import TriggerData

if TYPE_CHECKING:
    from bench.language import (
        Action,
        Interruption,
        Plan,
        Run,
        Task,
    )

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.TRIGGER_TYPE)
class TriggerType(BuiltinEnum):
    # flow
    START = 10, "Start", "Run when a Flow is started", "fas fa-play"
    MESSAGE = 20, "Message", "Run when a Message is received", "fas fa-envelope"
    SCHEDULE = 30, "Schedule", "Run on a schedule", "fas fa-clock"
    # RECORD, LOG, EDIT, ...


@enum_(EnumType.TRIGGER_STATUS)
class TriggerStatus(BuiltinEnum):
    INACTIVE = 1, "Inactive", "Not actively triggering", "fas fa-bolt-slash", ColorType.GRAY
    OPEN = 10, "Open", "Actively triggering", "fas fa-bolt", ColorType.BLUE
    # CLOSED = 20, "Closed", "No longer triggering", "fas fa-toggle-large-off", ColorType.GRAY

    @property
    def is_open(self) -> bool:
        return self.id >= 10 and self.id < 20


@enum_(EnumType.TRIGGER_EFFECT)
class TriggerEffect(BuiltinEnum):
    # run
    RUN = 10, "Start Run", "Start a new Run", "fas fa-play"
    # interruption
    # TASK = 50, "Instantiate Task", "Create a new Task from a template", "fas fa-tasks"


@node_(NodeType.TRIGGER)
class Trigger(IsTemplatable, IsModal, PackageNode[TriggerData]):
    """
    A Trigger is an event-driven condition that, once met, affects the Bench somehow.
    """

    parent: Union["Action", "Plan", "Task", None] = p_node_parent(
        4, NodeType.ACTION, NodeType.PLAN, NodeType.TASK, NodeType.RUN
    )

    # meta
    type: TriggerType = p_system(30, require=True)
    name: str | None = p_regular(32, constraint=NAME_CONSTRAINT)
    text: Optional[Text] = p_regular(33, require=False, struct=StructType.TEXT)
    effect: TriggerEffect = p_regular(34, require=True)
    run_root: Optional["Run"] = p_regular(
        36,
        require=False,
        array=False,
        references=NodeType.RUN,
        description="The root Run this Trigger is scoped to.",
    )
    run: Optional["Run"] = p_regular(
        37,
        require=False,
        array=False,
        references=NodeType.RUN,
        description="The Run this Trigger is scoped to.",
    )
    interruption: Optional["Interruption"] = p_regular(
        38,
        require=False,
        array=False,
        references=NodeType.INTERRUPTION,
    )

    # content
    # schedule, ...

    # status
    status: TriggerStatus = p_regular(40, default=TriggerStatus.OPEN)
    processed_at: Optional[datetime] = p_system(41, default=None)
    processed_count: int = p_system(42, default=0)
    processed_key: Optional[str] = p_system(43, default=None)
    closed_at: Optional[datetime] = p_system(45, default=None)

    @property
    def is_open(self) -> bool:
        return self.status.is_open

    @staticmethod
    def on_start(
        name: str | None = None,
        text: Text | None = None,
        *,
        effect: TriggerEffect = TriggerEffect.RUN,
    ) -> "Trigger":
        return Trigger(type=TriggerType.START, name=name, text=text, effect=effect)

    @staticmethod
    def on_message(
        name: str | None = None,
        text: Text | None = None,
        *,
        effect: TriggerEffect = TriggerEffect.RUN,
    ) -> "Trigger":
        trigger = Trigger(type=TriggerType.MESSAGE, name=name, text=text, effect=effect)
        return trigger
