from typing import TYPE_CHECKING, Any, Optional, Union, cast

from bench.language.const import BlockType, EnumType, NodeType, ObjectKind, StructType, enum_
from bench.language.node import BuiltinObject, NodeReference, Struct, object_, struct_
from bench.language.property import p_regular, p_value_packed, p_value_runtime
from bench.language.validation import constraint
from bench.language.value import coerce_custom_object
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import Block, Code, Pipe, RunOptions, Step, Text


@enum_(EnumType.ACTION_MODE)
class ActionMode(IdEnum):
    STRICT = 1  # always run as specified
    ADAPTIVE = 2  # run as specified by default but adapt if out of date or error
    DYNAMIC = 3  # dynamically adapt to inputs every time


@object_()
class ActionBase(BuiltinObject):
    """Common base for 'actions' that do something using code somehow."""

    mode: ActionMode = p_regular(100, default=ActionMode.ADAPTIVE)
    text: Optional["Text"] = p_regular(
        101,
        default=None,
        require=False,
        array=False,
        struct=StructType.TEXT,
        description="Text description of this action.",
    )
    code: Optional["Code"] = p_regular(
        102,
        default=None,
        require=False,
        array=False,
        struct=StructType.CODE,
        description="Current implementation code for this action.",
    )
    node: Optional["Block"] = p_regular(
        103,
        require=False,
        references=NodeType.BLOCK,
        constraint=constraint(block_types=[BlockType.ACTION, BlockType.FLOW]),
        description="Current implementation for this action (may be wrapped in code).",
    )
    run_options: Optional["RunOptions"] = p_regular(
        110, default=None, require=False, array=False, struct=StructType.RUN_OPTIONS
    )
    if TYPE_CHECKING:
        node_ptr: Optional["NodeReference"] = None


@struct_(StructType.CALL)
class Call(Struct):
    """A context-specific call to a Run inside the current Run."""

    node: Union["Block", "Step"] = p_regular(
        30,
        require=True,
        references=NodeType.BLOCK,
        constraint=constraint(block_types=[BlockType.ACTION, BlockType.FLOW]),
    )
    inputs_packed: Any = p_value_packed(31)
    inputs: Any = p_value_runtime(
        31, kind=ObjectKind.INPUT, typ=lambda self: cast(Call, self).node.input_type
    )
    # inputs, ...?

    @staticmethod
    def new(node: "Block | Step", inputs: Any, **kwargs) -> "Call":
        input_type = node.input_type
        assert input_type is not None, f"no input type for {node!r}"
        return Call(
            node=node, inputs=coerce_custom_object(ObjectKind.INPUT, input_type, inputs), **kwargs
        )


@struct_(StructType.CONTINUE)
class Continue(Struct):
    """A context-specific continuation for a Run to proceed elsewhere (like in a Flow)."""

    node: Union["Block", "Step", "Pipe"] = p_regular(
        30,
        require=True,
        references=NodeType.BLOCK,
        constraint=constraint(block_types=[BlockType.ACTION, BlockType.FLOW]),
    )
    # inputs, ...?
    mapping: Optional["Code"] = p_regular(
        50,
        require=False,
        array=False,
        struct=StructType.CODE,
        description="Mapping to get inputs for next node.",
    )

    @staticmethod
    def new(node: "Block | Step | Pipe", **kwargs) -> "Continue":
        return Continue(node=node, **kwargs)


@struct_(StructType.CONTEXT)
class Context(Struct):
    """Context for a Run."""

    pass


class ContextBuilder:
    def __init__(self, task: Optional["Text"] = None) -> None:
        self._task: Optional[Text] = task

    def copy(self) -> "ContextBuilder":
        return ContextBuilder(task=self._task)

    def task(self, task: "Text") -> "ContextBuilder":
        copy = self.copy()
        copy._task = task
        return copy
