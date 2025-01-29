from functools import cached_property
from typing import TYPE_CHECKING, Any, Optional, Union, cast

from bench.language.core import (
    BlockType,
    BuiltinEnum,
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

if TYPE_CHECKING:
    from bench.language import Action, Block, CustomObject, NodeReference, Text, TypeBase


# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.CALL_EXECUTION_MODE)
class CallExecutionMode(BuiltinEnum):
    """How to execute Calls."""

    SERIAL = 1, "Run calls after each other"
    PARALLEL = 2, "Run calls at the same time"


@enum_(EnumType.CALL_FAILURE_MODE)
class CallFailureMode(BuiltinEnum):
    """What to do when a Call fails."""

    FAIL = 1, "Fail the plan"
    CONTINUE = 2, "Continue the plan"
    COMPLETE = 3, "Complete the plan"


@enum_(EnumType.CALL_TERMINATION_MODE)
class CallTerminationMode(BuiltinEnum):
    """What to do when a Call terminates."""

    PASS = 1, "Do nothing"
    RETURN = 2, "Return to the caller"


@struct_(StructType.CALL)
class Call(Struct):
    """A Call to something (to be materialized into a Run)."""

    node: Union["Block", "Action", None] = p_regular(
        33,
        require=True,
        references=(NodeType.BLOCK, NodeType.ACTION),
        constraint=constraint(node_subtypes=[BlockType.FLOW]),
    )
    if TYPE_CHECKING:
        node_ptr: NodeReference | None = None
        node_id: str | None = None
    value_packed: Any = p_value_packed(34)
    value: Any = p_value_runtime(
        34, type=FieldType.INPUT, typ=lambda self: cast(Call, self).value_type
    )

    title: str | None = p_regular(40, default=None)
    text: Optional["Text"] = p_regular(41, default=None, struct=StructType.TEXT)

    def __content_str__(self) -> str:
        content_parts: list[str] = []
        if node := self.node:
            content_parts.append(repr(node))
        else:
            content_parts.append("???")
        if title := self.title:
            content_parts.append(repr(title))
        if (value := self.value) is not None and (value_str := str(value)):
            content_parts.append(value_str)
        return " ".join(content_parts)

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
        *,
        title: str | None = None,
        text: "Text | None" = None,
        **kwargs,
    ) -> "Call":
        from bench.language.source import Action, Block

        if not isinstance(node, (Block, Action)):
            raise ValueError(f"invalid node type for Call: {type(node)}")

        value_type = node.to_type_maybe(
            of="value", field_types=[FieldType.VARIABLE, FieldType.INPUT]
        )
        assert value_type is not None, f"no call value type for {node!r}"
        value = coerce_custom_object_scalar(value or kwargs, value_type)
        return Call(node=node, title=title, text=text, value=value, **kwargs)


#
# Calls/planning
#


@struct_(StructType.CALL_PLAN)
class CallPlan(Struct):
    """A plan for some (potentially interleaved) Calls."""

    execution: CallExecutionMode = p_internal(31, default=CallExecutionMode.SERIAL)
    on_error: CallFailureMode = p_internal(33, default=CallFailureMode.FAIL)
    on_terminate: CallTerminationMode = p_internal(34, default=CallTerminationMode.PASS)

    calls: list[Call] = p_internal(40, array=True, struct=StructType.CALL)

    @classmethod
    def from_call(cls, call: Call) -> "CallPlan":
        return CallPlan(calls=[call])

    @staticmethod
    def new(
        *calls: Call,
        execution: CallExecutionMode = CallExecutionMode.SERIAL,
        on_error: CallFailureMode = CallFailureMode.FAIL,
        on_terminate: CallTerminationMode = CallTerminationMode.PASS,
    ) -> "CallPlan":
        return CallPlan(
            calls=list(calls),
            execution=execution,
            on_error=on_error,
            on_terminate=on_terminate,
        )


def call(
    node: "Block | Action",
    title: str | None = None,
    text: "Text | None" = None,
    **kwargs,
) -> "Call":
    return Call.new(node, title=title, text=text, **kwargs)


def call_serial(
    *calls: Call,
    on_error: CallFailureMode = CallFailureMode.FAIL,
    on_terminate: CallTerminationMode = CallTerminationMode.PASS,
) -> "CallPlan":
    """Call the given nodes in series."""
    return CallPlan.new(*calls, on_error=on_error, on_terminate=on_terminate)


def call_parallel(
    *calls: Call,
    on_error: CallFailureMode = CallFailureMode.FAIL,
    on_terminate: CallTerminationMode = CallTerminationMode.PASS,
) -> "CallPlan":
    """Call the given nodes in parallel."""
    return CallPlan.new(
        *calls, execution=CallExecutionMode.PARALLEL, on_error=on_error, on_terminate=on_terminate
    )


def call_none() -> "CallPlan":
    """Call nothing. Useful when you need *some* plan (like for Action.plans as output)."""
    return CallPlan.new()
