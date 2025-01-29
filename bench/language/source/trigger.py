from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union

from bench.language.core import (
    NAME_CONSTRAINT,
    BlockType,
    EnumType,
    NodeType,
    SourceNode,
    StructType,
    constraint,
    enum_,
    node_,
    p_node_parent,
    p_regular,
    p_system,
)
from bench.language.core.node import subnode_
from bench.pb2.lang_pb2 import TriggerData
from bench.utils.func import IdEnum

from .schedule import Schedule

if TYPE_CHECKING:
    from bench.language import Action, Block, Interruption, Message, Package, Run, Text

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.TRIGGER_TYPE)
class TriggerType(IdEnum):
    SCHEDULE = 10
    MESSAGE = 20


@enum_(EnumType.TRIGGER_STATUS)
class TriggerStatus(IdEnum):
    INACTIVE = 1
    OPEN = 10
    CLOSED = 20


@enum_(EnumType.TRIGGER_EFFECT)
class TriggerEffect(IdEnum):
    # interruption
    CANCEL_INTERRUPTION = 20
    COMPLETE_INTERRUPTION = 21


@node_(NodeType.TRIGGER, has_subtypes=True)
class Trigger(SourceNode[TriggerData]):
    """A Trigger is a (possibly recurring) condition that, when met, affects the runtime."""

    parent: Union["Action", "Run", None] = p_node_parent(4, NodeType.ACTION, NodeType.RUN)

    # meta
    type: TriggerType = p_regular(30, require=True)
    name: str = p_regular(32, constraint=NAME_CONSTRAINT)
    text: Optional["Text"] = p_regular(33, require=False, struct=StructType.TEXT)
    effect: TriggerEffect = p_regular(34, require=True)
    interruption: Optional["Interruption"] = p_regular(
        35,
        require=False,
        array=False,
        references=NodeType.INTERRUPTION,
        description="The interruption this Trigger is for.",
    )
    scope: Union["Block", "Package"] = p_regular(
        36,
        require=False,
        array=False,
        references=(NodeType.BLOCK, NodeType.PACKAGE),
        description="The source Node this Trigger is scoped to.",
    )
    run: Optional["Run"] = p_regular(
        37,
        require=False,
        array=False,
        references=NodeType.RUN,
        description="The Run this Trigger is scoped to",
    )

    # status
    status: TriggerStatus = p_regular(40, default=TriggerStatus.OPEN)
    closed_at: Optional[datetime] = p_system(41, default=None)


@subnode_(TriggerType.SCHEDULE)
class ScheduleTrigger(Trigger):
    schedule: Optional["Schedule"] = p_regular(
        100, require=False, array=False, struct=StructType.SCHEDULE
    )


@subnode_(TriggerType.MESSAGE)
class MessageTrigger(Trigger):
    message: Optional["Message"] = p_regular(
        101,
        require=False,
        array=False,
        references=NodeType.MESSAGE,
        description="The Message (thread) this Trigger is for.",
    )
    message_type: Optional["Block"] = p_regular(
        102,
        require=False,
        array=False,
        references=NodeType.BLOCK,
        constraint=constraint(node_subtypes=[BlockType.MESSAGE]),
        description="The Message type.",
    )
    message_reply_to: Optional["Message"] = p_regular(
        103,
        require=False,
        array=False,
        references=NodeType.MESSAGE,
        description="The Message this Trigger is replying to.",
    )
    # to: roles, identities, users, teams, ... :MessageRouting
    # condition, edit, ...?
