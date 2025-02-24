from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union

from bench.language.core import (
    INLINE_SOURCE_NODE_TYPES,
    NAME_CONSTRAINT,
    BuiltinEnum,
    EnumType,
    InlineSourceNode,
    NodeType,
    SourceNode,
    StructType,
    Text,
    enum_,
    generate_node_name,
    node_,
    p_node_parent,
    p_regular,
    p_system,
    subnode_,
)
from bench.pb2 import TriggerData

from .schedule import Schedule

if TYPE_CHECKING:
    from bench.language import (
        Action,
        Channel,
        Interruption,
        Node,
        Package,
        Page,
        Run,
        Thread,
    )

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.TRIGGER_TYPE)
class TriggerType(BuiltinEnum):
    SCHEDULE = 10
    MESSAGE = 20
    # RECORD, LOG, EDIT, ...


@enum_(EnumType.TRIGGER_STATUS)
class TriggerStatus(BuiltinEnum):
    INACTIVE = 1
    OPEN = 10
    CLOSED = 20

    @property
    def is_open(self) -> bool:
        return self.id >= 10 and self.id < 20


@enum_(EnumType.TRIGGER_EFFECT)
class TriggerEffect(BuiltinEnum):
    # run
    START_RUN = 1, "Start a new Run"
    ENSURE_RUN = 2, "Continue an existing or start a new Run"
    REPLACE_RUN = 3, "Replace/restart (part of) the current Run"
    # interruption
    # CANCEL_INTERRUPTION, COMPLETE_INTERRUPTION, ...?


@node_(NodeType.TRIGGER, has_subtypes=True)
class Trigger(SourceNode[TriggerData]):
    """
    A Trigger is an event-driven condition that, once met, affects the runtime.
    Depending on its type and scope, it causes some effect (like starting a Run or interrupting a Task).
    """

    parent: Union["Action", "Run", None] = p_node_parent(4, NodeType.ACTION, NodeType.RUN)

    # meta
    type: TriggerType = p_system(30, require=True)
    name: str = p_regular(32, constraint=NAME_CONSTRAINT)
    text: Optional[Text] = p_regular(33, require=False, struct=StructType.TEXT)
    effect: TriggerEffect = p_regular(34, require=True)
    scope: Union["InlineSourceNode", "Package", None] = p_regular(
        35,
        require=False,
        array=False,
        references=(*INLINE_SOURCE_NODE_TYPES, NodeType.PACKAGE),
    )
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

    # status
    status: TriggerStatus = p_regular(40, default=TriggerStatus.OPEN)
    processed_at: Optional[datetime] = p_system(41, default=None)
    processed_count: int = p_system(42, default=0)
    processed_key: Optional[str] = p_system(43, default=None)
    closed_at: Optional[datetime] = p_system(45, default=None)

    @property
    def container(self) -> "Node | None":
        return self.scope

    @property
    def is_open(self) -> bool:
        return self.status.is_open

    @staticmethod
    def on_schedule(
        name: str | None = None,
        text: Text | None = None,
        *,
        effect: TriggerEffect = TriggerEffect.START_RUN,
        scope: Union["Page", "Package", None] = None,
    ) -> "Trigger":
        if name is None:
            name = generate_node_name(NodeType.TRIGGER, TriggerType.SCHEDULE, siblings=())
        trigger = Trigger(
            type=TriggerType.SCHEDULE, name=name, text=text, effect=effect, scope=scope
        )
        return trigger

    @staticmethod
    def on_message(
        name: str | None = None,
        text: Text | None = None,
        *,
        effect: TriggerEffect = TriggerEffect.START_RUN,
        scope: Union["Page", "Package", None] = None,
    ) -> "Trigger":
        if name is None:
            name = generate_node_name(NodeType.TRIGGER, TriggerType.MESSAGE, siblings=())
        trigger = Trigger(
            type=TriggerType.MESSAGE, name=name, text=text, effect=effect, scope=scope
        )
        return trigger


@subnode_(TriggerType.SCHEDULE)
class ScheduleTrigger(Trigger):
    schedule: Optional["Schedule"] = p_regular(
        100, require=False, array=False, struct=StructType.SCHEDULE
    )


@subnode_(TriggerType.MESSAGE)
class MessageTrigger(Trigger):
    channel: Optional["Channel"] = p_regular(
        100, require=False, array=False, references=NodeType.CHANNEL
    )
    thread: Optional["Thread"] = p_regular(
        101, require=False, array=False, references=NodeType.THREAD
    )
