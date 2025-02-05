from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union

from bench.language.core import (
    NAME_CONSTRAINT,
    BuiltinEnum,
    EnumType,
    NodeType,
    SourceNode,
    StructType,
    enum_,
    node_,
    p_node_parent,
    p_regular,
    p_system,
)
from bench.language.core.node import subnode_
from bench.pb2.lang_pb2 import TriggerData

from .schedule import Schedule

if TYPE_CHECKING:
    from bench.language import (
        Action,
        Channel,
        Class,
        Interruption,
        Message,
        Node,
        Package,
        Page,
        Run,
        Text,
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


@enum_(EnumType.TRIGGER_EFFECT)
class TriggerEffect(BuiltinEnum):
    # run
    START_RUN = 1, "Start a new Run"
    CONTINUE_RUN = 2, "Continue an existing or start a new Run"
    # interruption
    CANCEL_INTERRUPTION = 20, "Cancel an Interruption"
    COMPLETE_INTERRUPTION = 21, "Complete an Interruption"


@node_(NodeType.TRIGGER, has_subtypes=True)
class Trigger(SourceNode[TriggerData]):
    """
    A Trigger is an event-driven condition that affects the runtime.
    Depending on its type and scope, it causes some effect (like starting a Run or interrupting a Task).
    """

    parent: Union["Action", "Run", None] = p_node_parent(4, NodeType.ACTION, NodeType.RUN)

    # meta
    type: TriggerType = p_system(30, require=True)
    name: str = p_regular(32, constraint=NAME_CONSTRAINT)
    text: Optional["Text"] = p_regular(33, require=False, struct=StructType.TEXT)
    effect: TriggerEffect = p_regular(34, require=True)
    scope: Union["Page", "Package"] = p_regular(
        35,
        require=True,
        array=False,
        references=(NodeType.PAGE, NodeType.PACKAGE),
        description="The source Node this Trigger is scoped to.",
    )
    run: Optional["Run"] = p_regular(
        36,
        require=False,
        array=False,
        references=NodeType.RUN,
        description="The Run this Trigger is scoped to.",
    )
    interruption: Optional["Interruption"] = p_regular(
        37,
        require=False,
        array=False,
        references=NodeType.INTERRUPTION,
        description="The interruption this Trigger is for.",
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
    clazz: Optional["Class"] = p_regular(
        102,
        require=False,
        array=False,
        references=NodeType.CLASS,
        description="The Message type.",
    )
    reply_to: Optional["Message"] = p_regular(
        103,
        require=False,
        array=False,
        references=NodeType.MESSAGE,
        description="The Message this Trigger is replying to.",
    )
    # to: roles, identities, users, teams, ...
    # condition, edit, ...?
