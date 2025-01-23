from functools import cached_property
from typing import TYPE_CHECKING, Any, Optional, Union, cast

from bench.language.core import (
    BlockType,
    CustomObject,
    EnumType,
    FieldType,
    NodeType,
    Struct,
    StructType,
    coerce_custom_object_scalar,
    constraint,
    enum_,
    p_internal,
    p_regular,
    p_value_packed,
    p_value_runtime,
    struct_,
)
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import Action, Block, CustomObject, NodeReference, TypeBase


# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.CALL_ERROR_MODE)
class CallErrorMode(IdEnum):
    FAIL = 1
    IGNORE = 2
    TERMINATE = 3


@enum_(EnumType.CALL_TERMINATION_MODE)
class CallTerminationMode(IdEnum):
    STOP = 1
    RETURN = 2
    DEFER = 3


@struct_(StructType.CALL)
class Call(Struct):
    """A Call to something."""

    node: Union["Block", "Action", None] = p_regular(
        31,
        require=True,
        references=(NodeType.BLOCK, NodeType.ACTION),
        constraint=constraint(node_subtypes=[BlockType.FLOW]),
    )
    if TYPE_CHECKING:
        node_ptr: NodeReference | None = None
        node_id: str | None = None
    value_packed: Any = p_value_packed(33)
    value: Any = p_value_runtime(
        33, type=FieldType.INPUT, typ=lambda self: cast(Call, self).value_type
    )
    on_terminate: CallTerminationMode = p_regular(35, default=CallTerminationMode.STOP)

    @cached_property
    def value_type(self) -> Optional["TypeBase"]:
        node = self.node
        return (
            node.to_type_maybe(of="value", field_types=[FieldType.VARIABLE, FieldType.INPUT])
            if node is not None
            else None
        )

    @staticmethod
    def new(
        node: "Block | Action",
        value: CustomObject | None = None,
        **kwargs,
    ) -> "Call":
        value_type = node.to_type_maybe(
            of="value", field_types=[FieldType.VARIABLE, FieldType.INPUT]
        )
        assert value_type is not None, f"no call value type for {node!r}"
        return Call(
            node=node,
            value=coerce_custom_object_scalar(value or {}, value_type),
            **kwargs,
        )


#
# Calls/planning
#


@struct_(StructType.CALL_PLAN)
class CallPlan(Struct):
    """A plan for some (potentially interleaved) Calls."""

    on_terminate: CallTerminationMode = p_internal(33, default=CallTerminationMode.STOP)
    on_error: CallErrorMode = p_internal(34, default=CallErrorMode.TERMINATE)

    calls: list[Call] = p_internal(40, array=True, struct=StructType.CALL)

    @classmethod
    def from_call(cls, call: Call) -> "CallPlan":
        return CallPlan(calls=[call])

    @staticmethod
    def new(
        *calls: Call,
        on_error: CallErrorMode = CallErrorMode.TERMINATE,
        on_terminate: CallTerminationMode = CallTerminationMode.STOP,
    ) -> "CallPlan":
        return CallPlan(calls=list(calls), on_error=on_error, on_terminate=on_terminate)


def call(node: "Block | Action", **kwargs) -> "Call":
    return Call.new(node, **kwargs)


def call_serial(
    *calls: Call,
    on_error: CallErrorMode = CallErrorMode.TERMINATE,
    on_terminate: CallTerminationMode = CallTerminationMode.STOP,
) -> "CallPlan":
    return CallPlan.new(*calls, on_error=on_error, on_terminate=on_terminate)


def call_single(
    node: "Block | Action",
    on_error: CallErrorMode = CallErrorMode.FAIL,
    on_terminate: CallTerminationMode = CallTerminationMode.STOP,
    **kwargs,
) -> "CallPlan":
    return CallPlan.new(
        call(node, **kwargs),
        on_error=on_error,
        on_terminate=on_terminate,
    )
