from typing import TYPE_CHECKING, Any, Optional, Union, cast

from bench.language.const import BlockType, EnumType, NodeType, ObjectKind, StructType, enum_
from bench.language.field import TypeBase
from bench.language.node import BuiltinObject, NodeReference, Struct, object_, struct_
from bench.language.property import p_regular, p_value_packed, p_value_runtime
from bench.language.validation import constraint
from bench.language.value import coerce_custom_object
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import Block, Code, Field, ObjectMapping, Pipe, Step


@enum_(EnumType.AGENCY)
class Agency(IdEnum):
    CODE = 1, "Run specific hand-written code"
    DELEGATE = 2  # always run a specific delegate
    GENERATE = 3  # dynamically adapt or answer inputs every time


@object_()
class ActionBase(BuiltinObject):
    """Common base for 'actions' that do something using code somehow."""

    agency: Agency = p_regular(100, default=Agency.GENERATE)
    is_dynamic: bool = p_regular(
        101, default=False, description="Whether this action is dynamic w.r.t. its inputs."
    )
    code: Optional["Code"] = p_regular(
        111,
        default=None,
        require=False,
        array=False,
        struct=StructType.CODE,
        description="Current implementation code for this action.",
    )
    tools: list[Union["Block", "Field"]] = p_regular(
        112,
        require=False,
        array=True,
        references=(NodeType.BLOCK, NodeType.FIELD),
        constraint=constraint(node_subtypes=[BlockType.ACTION, BlockType.FLOW]),
        description="Available implementations and tools for this action.",
    )
    delegate: Optional["Block"] = p_regular(
        113,
        require=False,
        array=False,
        references=NodeType.BLOCK,
        constraint=constraint(node_subtypes=[BlockType.ACTION, BlockType.FLOW]),
        description="Current implementation for this action.",
    )
    if TYPE_CHECKING:
        tools_ptr: tuple["NodeReference", ...] = ()
        delegate_ptr: "NodeReference | None" = None


@struct_(StructType.CALL)
class Call(Struct):
    """A Call to a Run (inside/from the current Run usually)."""

    node: Union["Block", "Step"] = p_regular(
        30,
        require=True,
        references=NodeType.BLOCK,
        constraint=constraint(node_subtypes=[BlockType.ACTION, BlockType.FLOW]),
    )
    inputs_packed: Any = p_value_packed(31)
    inputs: Any = p_value_runtime(
        31, kind=ObjectKind.INPUT, typ=lambda self: cast(Call, self).input_type
    )
    mapping: Optional["ObjectMapping"] = p_regular(
        50,
        require=False,
        array=False,
        struct=StructType.OBJECT_MAPPING,
        description="Mapping for inputs from current node into called node.",
    )
    mapping_code: Optional["Code"] = p_regular(
        51,
        require=False,
        array=False,
        struct=StructType.CODE,
        description="Mapping for outputs from called node into new node. Takes precedence over mapping.",
    )

    @property
    def input_type(self) -> Optional["TypeBase"]:
        node = self.node
        return node.input_type if node is not None else None

    @staticmethod
    def new(node: "Block | Step", inputs: Any | None = None, **kwargs) -> "Call":
        input_type = node.input_type
        assert input_type is not None, f"no input type for {node!r}"
        return Call(
            node=node, inputs=coerce_custom_object(ObjectKind.INPUT, input_type, inputs), **kwargs
        )


@struct_(StructType.CONTINUE)
class Continue(Struct):
    """A "Continuation" of a Run somewhere (like in a Flow)."""

    node: Union["Block", "Step", "Pipe", None] = p_regular(
        30,
        require=True,
        references=NodeType.BLOCK,
        constraint=constraint(node_subtypes=[BlockType.ACTION, BlockType.FLOW]),
    )
    inputs_packed: Any = p_value_packed(31)
    inputs: Any = p_value_runtime(
        31, kind=ObjectKind.INPUT, typ=lambda self: cast(Continue, self).input_type
    )
    mapping: Optional["ObjectMapping"] = p_regular(
        50,
        require=False,
        array=False,
        struct=StructType.OBJECT_MAPPING,
        description="Mapping for inputs from current node into next node.",
    )
    mapping_code: Optional["Code"] = p_regular(
        51,
        require=False,
        array=False,
        struct=StructType.CODE,
        description="Mapping to get inputs for next node. Takes precedence over mapping.",
    )

    @property
    def input_type(self) -> Optional["TypeBase"]:
        node = self.node
        return node.input_type if node is not None else None

    @staticmethod
    def new(node: "Block | Step | Pipe", inputs: Any | None = None, **kwargs) -> "Continue":
        input_type = node.input_type
        assert input_type is not None, f"no input type for {node!r}"
        return Continue(
            node=node, inputs=coerce_custom_object(ObjectKind.INPUT, input_type, inputs), **kwargs
        )

    at = new
