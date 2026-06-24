# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.mir.metadata.dispatch
import destack._generated.mir.metadata.profile
import destack._generated.mir.tree.call
import destack._generated.mir.tree.constant
import destack._generated.mir.tree.immediate
import destack._generated.mir.tree.intrinsic
import destack._generated.mir.tree.memory
import destack._generated.mir.tree.node
import destack._generated.mir.tree.operator
import destack._generated.mir.tree.tensor
import destack._generated.mir.tree.value
import destack._generated.mir.tree.vector

@dataclass(frozen=True, slots=True)
class InstructionError:
    """Recovered invalid instruction syntax."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionConst:
    """Load a constant value."""

    # the SSA value to define
    destination: destack._generated.mir.tree.value.Value
    # the constant value to load
    value: destack._generated.mir.tree.constant.Constant
    kind: typing.Literal["const"] = "const"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionBinary:
    """Binary operation (e.g., add, subtract, compare)."""

    # the SSA value to define with the result
    destination: destack._generated.mir.tree.value.Value
    # the binary operator to apply
    operator: destack._generated.mir.tree.operator.BinaryOperator
    # the left-hand operand
    left: destack._generated.mir.tree.value.Value
    # the right-hand operand
    right: destack._generated.mir.tree.value.Value
    kind: typing.Literal["binary"] = "binary"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionUnary:
    """Unary operation (e.g., negate, not)."""

    # the SSA value to define with the result
    destination: destack._generated.mir.tree.value.Value
    # the unary operator to apply
    operator: destack._generated.mir.tree.operator.UnaryOperator
    # the operand
    argument: destack._generated.mir.tree.value.Value
    kind: typing.Literal["unary"] = "unary"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionCast:
    """Cast between types (bitcast, truncate, extend, etc.)."""

    # the SSA value to define with the converted result
    destination: destack._generated.mir.tree.value.Value
    # the cast operator to perform
    operator: CastOperator
    # the value to cast
    argument: destack._generated.mir.tree.value.Value
    # the target type to cast to
    to_type: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["cast"] = "cast"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionSelect:
    """Select a value based on a boolean condition."""

    # the SSA value to define with the selected result
    destination: destack._generated.mir.tree.value.Value
    # the boolean condition (must be bool type)
    condition: destack._generated.mir.tree.value.Value
    # the value returned if condition is true
    then_value: destack._generated.mir.tree.value.Value
    # the value returned if condition is false
    else_value: destack._generated.mir.tree.value.Value
    kind: typing.Literal["select"] = "select"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionLocalGet:
    """Load from a local variable (stack slot)."""

    # the SSA value to define with the loaded value
    destination: destack._generated.mir.tree.value.Value
    # the local variable to load from
    local: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["localGet"] = "localGet"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionLocalAddr:
    """Get the address of a local variable (stack slot)."""

    # the SSA value to define with the local address
    destination: destack._generated.mir.tree.value.Value
    # the local variable to take the address of
    local: destack._generated.mir.tree.node.LocalNodeId
    # the result type of the address
    result_type: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["localAddr"] = "localAddr"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionLocalSet:
    """Store to a local variable (stack slot)."""

    # the local variable to store to
    local: destack._generated.mir.tree.node.LocalNodeId
    # the value to store
    value: destack._generated.mir.tree.value.Value
    kind: typing.Literal["localSet"] = "localSet"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionGlobalAddr:
    """Get the address of a mutable global variable."""

    # the SSA value to define with the pointer
    destination: destack._generated.mir.tree.value.Value
    # the global variable to get the address of
    global_: destack._generated.mir.tree.node.LocalNodeId
    # the result type of the address
    result_type: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["globalAddr"] = "globalAddr"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionFunctionAddr:
    """Get a function pointer for a function (function.address)."""

    # the SSA value to define with the function pointer
    destination: destack._generated.mir.tree.value.Value
    # the function to take the address of
    function: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["functionAddr"] = "functionAddr"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionFunctionBind:
    """Bind one environment to a function and produce a function value (function.bind)."""

    # the SSA value to define with the function value
    destination: destack._generated.mir.tree.value.Value
    # the function to pair with the environment
    function: destack._generated.mir.tree.node.LocalNodeId
    # the environment value to capture
    environment: destack._generated.mir.tree.value.Value
    kind: typing.Literal["functionBind"] = "functionBind"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionFunctionPointer:
    """Project the function pointer from one function value (function.pointer)."""

    # the SSA value to define with the function pointer
    destination: destack._generated.mir.tree.value.Value
    # the function value to project
    function: destack._generated.mir.tree.value.Value
    kind: typing.Literal["functionPointer"] = "functionPointer"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionFunctionEnvironment:
    """Project the environment from one function value (function.environment)."""

    # the SSA value to define with the environment
    destination: destack._generated.mir.tree.value.Value
    # the function value to project
    function: destack._generated.mir.tree.value.Value
    kind: typing.Literal["functionEnvironment"] = "functionEnvironment"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionFunctionEnvironmentCurrent:
    """Load the hidden environment for the current function (function.environment.current)."""

    # the SSA value to define with the hidden environment pointer
    destination: destack._generated.mir.tree.value.Value
    kind: typing.Literal["functionEnvironmentCurrent"] = "functionEnvironmentCurrent"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionLoad:
    """Load from a pointer (dereference)."""

    # the SSA value to define with the loaded value
    destination: destack._generated.mir.tree.value.Value
    # the pointer to load from
    pointer: destack._generated.mir.tree.value.Value
    # the loaded value type
    result_type: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["load"] = "load"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionStore:
    """Store to a pointer (write through pointer)."""

    # the pointer to store to
    pointer: destack._generated.mir.tree.value.Value
    # the value to store
    value: destack._generated.mir.tree.value.Value
    kind: typing.Literal["store"] = "store"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionStruct:
    """Construct a struct from field values."""

    # the SSA value to define with the constructed struct
    destination: destack._generated.mir.tree.value.Value
    # the struct type to construct
    ty: destack._generated.mir.tree.node.LocalNodeId
    # the field values (stored in Tree's argument buffer)
    fields: destack._generated.mir.tree.value.ValueSlice
    kind: typing.Literal["struct"] = "struct"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionTuple:
    """Construct a tuple from element values."""

    # the SSA value to define with the constructed tuple
    destination: destack._generated.mir.tree.value.Value
    # the tuple type to construct
    ty: destack._generated.mir.tree.node.LocalNodeId
    # the element values (stored in Tree's argument buffer)
    elements: destack._generated.mir.tree.value.ValueSlice
    kind: typing.Literal["tuple"] = "tuple"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionArray:
    """Construct an array from element values."""

    # the SSA value to define with the constructed array
    destination: destack._generated.mir.tree.value.Value
    # the fixed array type to construct
    ty: destack._generated.mir.tree.node.LocalNodeId
    # the element values (stored in Tree's argument buffer)
    elements: destack._generated.mir.tree.value.ValueSlice
    kind: typing.Literal["array"] = "array"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionFieldGet:
    """Extract a static layout slot from an aggregate value (field.get)."""

    # the SSA value to define with the extracted field
    destination: destack._generated.mir.tree.value.Value
    # the aggregate value to extract from
    aggregate: destack._generated.mir.tree.value.Value
    # the zero-based layout slot index
    index: int
    kind: typing.Literal["fieldGet"] = "fieldGet"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionFieldSet:
    """Insert a value into a static layout slot (field.set)."""

    # the SSA value to define with the new aggregate
    destination: destack._generated.mir.tree.value.Value
    # the original aggregate value
    aggregate: destack._generated.mir.tree.value.Value
    # the zero-based layout slot index to update
    index: int
    # the value to insert at the field
    value: destack._generated.mir.tree.value.Value
    kind: typing.Literal["fieldSet"] = "fieldSet"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionFieldAddr:
    """Get the address of a static layout slot from an addressable aggregate (field.address)."""

    # the SSA value to define with the field address
    destination: destack._generated.mir.tree.value.Value
    # the aggregate base to project from
    aggregate: destack._generated.mir.tree.value.Value
    # the zero-based layout slot index
    index: int
    # the result type of the address
    result_type: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["fieldAddr"] = "fieldAddr"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionElementAddr:
    """Get the address of an element from an addressable indexed value (element.address)."""

    # the SSA value to define with the element address
    destination: destack._generated.mir.tree.value.Value
    # the indexed base to project from
    array: destack._generated.mir.tree.value.Value
    # the index of the element (runtime value)
    index: destack._generated.mir.tree.value.Value
    # the result type of the address
    result_type: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["elementAddr"] = "elementAddr"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionSliceView:
    """Form a non-owning slice view over a contiguous source region."""

    # the SSA value to define with the slice view
    destination: destack._generated.mir.tree.value.Value
    # the source slice value
    source: destack._generated.mir.tree.value.Value
    # the start index inside the source slice
    start: destack._generated.mir.tree.value.Value
    # the number of elements in the result
    length: destack._generated.mir.tree.value.Value
    # the result slice type
    result_type: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["sliceView"] = "sliceView"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionSliceLength:
    """Read the runtime length from a slice descriptor."""

    # the SSA value to define with the length
    destination: destack._generated.mir.tree.value.Value
    # the slice value whose length is read
    slice: destack._generated.mir.tree.value.Value
    kind: typing.Literal["sliceLength"] = "sliceLength"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionDynamicPayload:
    """Read the erased payload from a dynamic value."""

    # the SSA value to define with the payload
    destination: destack._generated.mir.tree.value.Value
    # the dynamic value whose payload is read
    dynamic: destack._generated.mir.tree.value.Value
    # the result type of the payload value
    result_type: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["dynamicPayload"] = "dynamicPayload"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionDynamicType:
    """Read the concrete type id from a dynamic value."""

    # the SSA value to define with the type id
    destination: destack._generated.mir.tree.value.Value
    # the dynamic value whose concrete type is read
    dynamic: destack._generated.mir.tree.value.Value
    kind: typing.Literal["dynamicType"] = "dynamicType"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionVariantTag:
    """Read the active tag from a physical tagged sum value."""

    # the SSA value to define with the active tag
    destination: destack._generated.mir.tree.value.Value
    # the variant value whose tag is read
    variant: destack._generated.mir.tree.value.Value
    kind: typing.Literal["variantTag"] = "variantTag"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionVariantPayload:
    """Extract the payload selected by a concrete variant tag."""

    # the SSA value to define with the payload
    destination: destack._generated.mir.tree.value.Value
    # the variant value whose payload is extracted
    variant: destack._generated.mir.tree.value.Value
    # the selected variant tag
    tag: destack._generated.mir.tree.constant.Constant
    kind: typing.Literal["variantPayload"] = "variantPayload"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionVectorSplat:
    """Broadcast a scalar to all vector lanes."""

    # the SSA value to define with the vector result
    destination: destack._generated.mir.tree.value.Value
    # the scalar value to broadcast
    value: destack._generated.mir.tree.value.Value
    kind: typing.Literal["vectorSplat"] = "vectorSplat"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionVectorExtract:
    """Extract a lane from a vector."""

    # the SSA value to define with the extracted lane
    destination: destack._generated.mir.tree.value.Value
    # the vector value to extract from
    vector: destack._generated.mir.tree.value.Value
    # the lane index to extract
    index: destack._generated.mir.tree.value.Value
    kind: typing.Literal["vectorExtract"] = "vectorExtract"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionVectorInsert:
    """Insert a lane into a vector."""

    # the SSA value to define with the updated vector
    destination: destack._generated.mir.tree.value.Value
    # the original vector value
    vector: destack._generated.mir.tree.value.Value
    # the lane index to update
    index: destack._generated.mir.tree.value.Value
    # the lane value to insert
    value: destack._generated.mir.tree.value.Value
    kind: typing.Literal["vectorInsert"] = "vectorInsert"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionVectorShuffle:
    """Shuffle vector lanes using a constant mask."""

    # the SSA value to define with the shuffled result
    destination: destack._generated.mir.tree.value.Value
    # the left vector operand
    left: destack._generated.mir.tree.value.Value
    # the right vector operand
    right: destack._generated.mir.tree.value.Value
    # the shuffle mask indices
    mask: destack._generated.mir.tree.immediate.IndexSlice
    kind: typing.Literal["vectorShuffle"] = "vectorShuffle"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionVectorSelect:
    """Select vector lanes based on a boolean mask."""

    # the SSA value to define with the selected result
    destination: destack._generated.mir.tree.value.Value
    # the boolean mask vector
    mask: destack._generated.mir.tree.value.Value
    # the value returned if the mask lane is true
    then_value: destack._generated.mir.tree.value.Value
    # the value returned if the mask lane is false
    else_value: destack._generated.mir.tree.value.Value
    kind: typing.Literal["vectorSelect"] = "vectorSelect"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionVectorReduce:
    """Reduce a vector to a scalar."""

    # the SSA value to define with the reduced result
    destination: destack._generated.mir.tree.value.Value
    # the reduction operator to apply
    operator: destack._generated.mir.tree.vector.VectorReduceOperator
    # the vector value to reduce
    vector: destack._generated.mir.tree.value.Value
    kind: typing.Literal["vectorReduce"] = "vectorReduce"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionVectorCompare:
    """Compare two vectors elementwise."""

    # the SSA value to define with the comparison result
    destination: destack._generated.mir.tree.value.Value
    # the comparison operator to apply
    operator: destack._generated.mir.tree.operator.BinaryOperator
    # the left vector operand
    left: destack._generated.mir.tree.value.Value
    # the right vector operand
    right: destack._generated.mir.tree.value.Value
    kind: typing.Literal["vectorCompare"] = "vectorCompare"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionVectorConvert:
    """Convert vector element types with an explicit mode."""

    # the SSA value to define with the converted vector
    destination: destack._generated.mir.tree.value.Value
    # the conversion mode to apply
    mode: destack._generated.mir.tree.vector.VectorConvertMode
    # the vector value to convert
    vector: destack._generated.mir.tree.value.Value
    kind: typing.Literal["vectorConvert"] = "vectorConvert"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionTensorSplat:
    """Broadcast a scalar to all tensor elements."""

    # the SSA value to define with the tensor result
    destination: destack._generated.mir.tree.value.Value
    # the scalar value to broadcast
    value: destack._generated.mir.tree.value.Value
    kind: typing.Literal["tensorSplat"] = "tensorSplat"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionTensorLoad:
    """Load a tensor element from a tensor reference."""

    # the SSA value to define with the loaded element
    destination: destack._generated.mir.tree.value.Value
    # the tensor reference to load from
    view: destack._generated.mir.tree.value.Value
    # the index values (stored in Tree's argument buffer)
    indices: destack._generated.mir.tree.value.ValueSlice
    kind: typing.Literal["tensorLoad"] = "tensorLoad"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionTensorExtract:
    """Extract a tensor element from a tensor value."""

    # the SSA value to define with the extracted element
    destination: destack._generated.mir.tree.value.Value
    # the tensor value to extract from
    tensor: destack._generated.mir.tree.value.Value
    # the index values (stored in Tree's argument buffer)
    indices: destack._generated.mir.tree.value.ValueSlice
    kind: typing.Literal["tensorExtract"] = "tensorExtract"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionTensorStore:
    """Store a tensor element into a tensor reference."""

    # the tensor reference to store into
    view: destack._generated.mir.tree.value.Value
    # the index values (stored in Tree's argument buffer)
    indices: destack._generated.mir.tree.value.ValueSlice
    # the value to store
    value: destack._generated.mir.tree.value.Value
    kind: typing.Literal["tensorStore"] = "tensorStore"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionTensorFill:
    """Fill a tensor reference with a scalar value."""

    # the tensor reference to fill
    view: destack._generated.mir.tree.value.Value
    # the scalar value to write
    value: destack._generated.mir.tree.value.Value
    kind: typing.Literal["tensorFill"] = "tensorFill"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionTensorCopy:
    """Copy elements from a source tensor reference into a destination tensor reference."""

    # the destination tensor reference
    target: destack._generated.mir.tree.value.Value
    # the source tensor reference
    source: destack._generated.mir.tree.value.Value
    kind: typing.Literal["tensorCopy"] = "tensorCopy"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionTensorReshape:
    """Reshape a tensor value into a new shape."""

    # the SSA value to define with the reshaped tensor
    destination: destack._generated.mir.tree.value.Value
    # the tensor value to reshape
    tensor: destack._generated.mir.tree.value.Value
    # the shape values (stored in Tree's argument buffer)
    shape: destack._generated.mir.tree.value.ValueSlice
    kind: typing.Literal["tensorReshape"] = "tensorReshape"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionTensorBroadcast:
    """Broadcast a tensor into a larger shape."""

    # the SSA value to define with the broadcasted tensor
    destination: destack._generated.mir.tree.value.Value
    # the tensor value to broadcast
    tensor: destack._generated.mir.tree.value.Value
    # the operand dimensions mapped into the result
    dimensions: destack._generated.mir.tree.immediate.IndexSlice
    kind: typing.Literal["tensorBroadcast"] = "tensorBroadcast"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionTensorTranspose:
    """Permute tensor dimensions."""

    # the SSA value to define with the transposed tensor
    destination: destack._generated.mir.tree.value.Value
    # the tensor value to transpose
    tensor: destack._generated.mir.tree.value.Value
    # the permutation of dimensions
    permutation: destack._generated.mir.tree.immediate.IndexSlice
    kind: typing.Literal["tensorTranspose"] = "tensorTranspose"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionTensorCast:
    """Refine a tensor type without changing its contents."""

    # the SSA value to define with the cast tensor
    destination: destack._generated.mir.tree.value.Value
    # the tensor value to cast
    tensor: destack._generated.mir.tree.value.Value
    kind: typing.Literal["tensorCast"] = "tensorCast"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionTensorView:
    """Create a view into a tensor reference."""

    # the SSA value to define with the view result
    destination: destack._generated.mir.tree.value.Value
    # the tensor reference to view
    view: destack._generated.mir.tree.value.Value
    # the view arguments (offsets, sizes, strides) stored in Tree's argument buffer
    arguments: destack._generated.mir.tree.value.ValueSlice
    # the number of offset values
    offsets_count: int
    # the number of size values
    sizes_count: int
    # the number of stride values
    strides_count: int
    kind: typing.Literal["tensorView"] = "tensorView"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionTensorSlice:
    """Slice a tensor by offsets, sizes, and strides."""

    # the SSA value to define with the sliced tensor
    destination: destack._generated.mir.tree.value.Value
    # the tensor value to slice
    tensor: destack._generated.mir.tree.value.Value
    # the slice arguments (offsets, sizes, strides) stored in Tree's argument buffer
    arguments: destack._generated.mir.tree.value.ValueSlice
    # the number of offset values
    offsets_count: int
    # the number of size values
    sizes_count: int
    # the number of stride values
    strides_count: int
    kind: typing.Literal["tensorSlice"] = "tensorSlice"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionTensorPad:
    """Pad a tensor with low, high, and interior padding."""

    # the SSA value to define with the padded tensor
    destination: destack._generated.mir.tree.value.Value
    # the tensor value to pad
    tensor: destack._generated.mir.tree.value.Value
    # the padding arguments (low, high, interior) stored in Tree's argument buffer
    arguments: destack._generated.mir.tree.value.ValueSlice
    # the number of low padding values
    low_count: int
    # the number of high padding values
    high_count: int
    # the number of interior padding values
    interior_count: int
    # the scalar padding value
    value: destack._generated.mir.tree.value.Value
    kind: typing.Literal["tensorPad"] = "tensorPad"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionTensorConcat:
    """Concatenate tensors along a dimension."""

    # the SSA value to define with the concatenated tensor
    destination: destack._generated.mir.tree.value.Value
    # the tensor operands stored in Tree's argument buffer
    tensors: destack._generated.mir.tree.value.ValueSlice
    # the concatenation axis
    axis: int
    kind: typing.Literal["tensorConcat"] = "tensorConcat"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionTensorCompare:
    """Compare two tensors elementwise."""

    # the SSA value to define with the comparison result
    destination: destack._generated.mir.tree.value.Value
    # the comparison operator to apply
    operator: destack._generated.mir.tree.operator.BinaryOperator
    # the left tensor operand
    left: destack._generated.mir.tree.value.Value
    # the right tensor operand
    right: destack._generated.mir.tree.value.Value
    kind: typing.Literal["tensorCompare"] = "tensorCompare"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionTensorSelect:
    """Select tensor elements based on a boolean mask."""

    # the SSA value to define with the selected tensor
    destination: destack._generated.mir.tree.value.Value
    # the boolean mask tensor
    mask: destack._generated.mir.tree.value.Value
    # the tensor returned if the mask element is true
    then_value: destack._generated.mir.tree.value.Value
    # the tensor returned if the mask element is false
    else_value: destack._generated.mir.tree.value.Value
    kind: typing.Literal["tensorSelect"] = "tensorSelect"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionTensorReduce:
    """Reduce a tensor along axes with a fixed operator."""

    # the SSA value to define with the reduced tensor
    destination: destack._generated.mir.tree.value.Value
    # the reduction operator to apply
    operator: destack._generated.mir.tree.tensor.TensorReduceOperator
    # the tensor value to reduce
    tensor: destack._generated.mir.tree.value.Value
    # the initial value for the reduction
    initial: destack._generated.mir.tree.value.Value
    # the axes to reduce
    axes: destack._generated.mir.tree.immediate.IndexSlice
    kind: typing.Literal["tensorReduce"] = "tensorReduce"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionTensorIndexReduce:
    """Reduce a tensor along one axis and return selected source indices."""

    # the SSA value to define with the index tensor
    destination: destack._generated.mir.tree.value.Value
    # the index reduction operator to apply
    operator: destack._generated.mir.tree.tensor.TensorIndexReduceOperator
    # the tensor value to reduce
    tensor: destack._generated.mir.tree.value.Value
    # the axis to reduce
    axis: int
    # the behavior for equal selected values
    tie_break: destack._generated.mir.tree.tensor.TensorIndexTieBreak
    kind: typing.Literal["tensorIndexReduce"] = "tensorIndexReduce"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionTensorDot:
    """Dot product of two tensors."""

    # the SSA value to define with the dot result
    destination: destack._generated.mir.tree.value.Value
    # the left operand
    left: destack._generated.mir.tree.value.Value
    # the right operand
    right: destack._generated.mir.tree.value.Value
    # the dot dimension numbers
    immediate: destack._generated.mir.tree.immediate.TensorImmediateId
    kind: typing.Literal["tensorDot"] = "tensorDot"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionTensorConvolution:
    """Convolution between an input tensor and a kernel tensor."""

    # the SSA value to define with the convolution result
    destination: destack._generated.mir.tree.value.Value
    # the input tensor
    input: destack._generated.mir.tree.value.Value
    # the kernel tensor
    kernel: destack._generated.mir.tree.value.Value
    # the convolution dimension numbers
    immediate: destack._generated.mir.tree.immediate.TensorImmediateId
    kind: typing.Literal["tensorConvolution"] = "tensorConvolution"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionTensorGather:
    """Gather slices from a tensor based on indices."""

    # the SSA value to define with the gathered tensor
    destination: destack._generated.mir.tree.value.Value
    # the operand tensor
    operand: destack._generated.mir.tree.value.Value
    # the indices tensor
    indices: destack._generated.mir.tree.value.Value
    # the gather dimension numbers
    immediate: destack._generated.mir.tree.immediate.TensorImmediateId
    kind: typing.Literal["tensorGather"] = "tensorGather"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionTensorScatter:
    """Scatter updates into a tensor based on indices."""

    # the SSA value to define with the scatter result
    destination: destack._generated.mir.tree.value.Value
    # the operand tensor
    operand: destack._generated.mir.tree.value.Value
    # the indices tensor
    indices: destack._generated.mir.tree.value.Value
    # the updates tensor
    updates: destack._generated.mir.tree.value.Value
    # the scatter dimension numbers
    immediate: destack._generated.mir.tree.immediate.TensorImmediateId
    # the scatter update mode
    mode: destack._generated.mir.tree.tensor.TensorScatterMode
    kind: typing.Literal["tensorScatter"] = "tensorScatter"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionTensorConvert:
    """Convert a tensor element type."""

    # the SSA value to define with the converted tensor
    destination: destack._generated.mir.tree.value.Value
    # the conversion mode to apply
    mode: destack._generated.mir.tree.tensor.TensorConvertMode
    # the tensor value to convert
    tensor: destack._generated.mir.tree.value.Value
    kind: typing.Literal["tensorConvert"] = "tensorConvert"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionCall:
    """Call a function directly."""

    # the SSA value to define with the return value, if any
    destination: destack._generated.mir.tree.value.Value | None
    # the function to call
    function: destack._generated.mir.tree.node.LocalNodeId
    # the shared call payload
    call: destack._generated.mir.tree.call.Call
    kind: typing.Literal["call"] = "call"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionCallVirtual:
    """Call a virtual method through a virtual dispatch slot."""

    # the SSA value to define with the return value, if any
    destination: destack._generated.mir.tree.value.Value | None
    # the receiver value for dispatch
    receiver: destack._generated.mir.tree.value.Value
    # the class type declaring this dispatch slot
    class_: destack._generated.mir.tree.node.LocalNodeId
    # the dispatch slot for the method
    slot: destack._generated.mir.metadata.dispatch.DispatchSlot
    # the shared call payload
    call: destack._generated.mir.tree.call.Call
    kind: typing.Literal["callVirtual"] = "callVirtual"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionCallDynamic:
    """Call through a dynamic dispatch table slot."""

    # the SSA value to define with the return value, if any
    destination: destack._generated.mir.tree.value.Value | None
    # the receiver value for dispatch
    receiver: destack._generated.mir.tree.value.Value
    # the dynamic constraint type declaring this dispatch slot
    constraint: destack._generated.mir.tree.node.LocalNodeId
    # the dispatch slot for the method
    slot: destack._generated.mir.metadata.dispatch.DispatchSlot
    # the shared call payload
    call: destack._generated.mir.tree.call.Call
    kind: typing.Literal["callDynamic"] = "callDynamic"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionCallIndirect:
    """Call through a function pointer (call.indirect)."""

    # the SSA value to define with the return value, if any
    destination: destack._generated.mir.tree.value.Value | None
    # the function pointer or function value to call
    callee: destack._generated.mir.tree.value.Value
    # the shared call payload
    call: destack._generated.mir.tree.call.Call
    kind: typing.Literal["callIndirect"] = "callIndirect"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionNewZeroed:
    """Allocate zeroed typed heap storage (`new.zeroed`)."""

    # the SSA value to define with the allocated reference
    destination: destack._generated.mir.tree.value.Value
    # the type of the struct to allocate
    layout: destack._generated.mir.tree.node.LocalNodeId
    # the result type of the allocation
    result_type: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["newZeroed"] = "newZeroed"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionNewUninit:
    """Allocate uninitialized typed heap storage (`new.uninit`)."""

    # the SSA value to define with the initialization token
    destination: destack._generated.mir.tree.value.Value
    # the type of the struct to allocate
    layout: destack._generated.mir.tree.node.LocalNodeId
    # the result type of the allocation
    result_type: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["newUninit"] = "newUninit"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionNewComplete:
    """Complete one initialized heap allocation (`new.complete`)."""

    # the SSA value to define with the completed allocation
    destination: destack._generated.mir.tree.value.Value
    # the initialization token to complete
    value: destack._generated.mir.tree.value.Value
    # the result type of the completed value
    result_type: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["newComplete"] = "newComplete"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionNewSliceZeroed:
    """Allocate zeroed typed repeated heap storage (`new.slice.zeroed`)."""

    # the SSA value to define with the allocated slice
    destination: destack._generated.mir.tree.value.Value
    # the element type of the repeated storage
    element: destack._generated.mir.tree.node.LocalNodeId
    # the number of elements (runtime value)
    length: destack._generated.mir.tree.value.Value
    # the result type of the allocation
    result_type: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["newSliceZeroed"] = "newSliceZeroed"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionNewSliceUninit:
    """Allocate uninitialized typed repeated heap storage (`new.slice.uninit`)."""

    # the SSA value to define with the initialization token
    destination: destack._generated.mir.tree.value.Value
    # the element type of the repeated storage
    element: destack._generated.mir.tree.node.LocalNodeId
    # the number of elements (runtime value)
    length: destack._generated.mir.tree.value.Value
    # the result type of the allocation
    result_type: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["newSliceUninit"] = "newSliceUninit"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionFree:
    """Release unique heap storage (`free`)."""

    # the unique heap reference to free
    value: destack._generated.mir.tree.value.Value
    kind: typing.Literal["free"] = "free"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionFrameAllocZeroed:
    """Allocate zeroed frame-scoped storage (`frame.alloc.zeroed`)."""

    # the SSA value to define with the frame allocation pointer
    destination: destack._generated.mir.tree.value.Value
    # the type of the value to allocate
    layout: destack._generated.mir.tree.node.LocalNodeId
    # the result type of the allocation
    result_type: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["frameAllocZeroed"] = "frameAllocZeroed"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionFrameAllocUninit:
    """Allocate uninitialized frame-scoped storage (`frame.alloc.uninit`)."""

    # the SSA value to define with the frame allocation pointer
    destination: destack._generated.mir.tree.value.Value
    # the type of the value to allocate
    layout: destack._generated.mir.tree.node.LocalNodeId
    # the result type of the allocation
    result_type: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["frameAllocUninit"] = "frameAllocUninit"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionPin:
    """Stabilize one heap value against movement (`pin`)."""

    # the SSA value to define with the pinned reference
    destination: destack._generated.mir.tree.value.Value
    # the heap value to pin
    value: destack._generated.mir.tree.value.Value
    # the result type of the pinned reference
    result_type: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["pin"] = "pin"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionUnpin:
    """Release one heap pin (`unpin`)."""

    # the heap value to unpin
    value: destack._generated.mir.tree.value.Value
    kind: typing.Literal["unpin"] = "unpin"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionBarrierWrite:
    """Record a managed reference write for the collector."""

    # the managed object whose reference range changed
    object: destack._generated.mir.tree.value.Value
    # the byte offset of the changed reference range
    offset: destack._generated.mir.tree.value.Value
    # the changed byte length
    byte_len: destack._generated.mir.tree.value.Value
    kind: typing.Literal["barrierWrite"] = "barrierWrite"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionAtomicLoad:
    """Load from memory atomically."""

    # the SSA value to define with the loaded result
    destination: destack._generated.mir.tree.value.Value
    # the pointer to load from
    pointer: destack._generated.mir.tree.value.Value
    # the loaded value type
    result_type: destack._generated.mir.tree.node.LocalNodeId
    # the atomic access
    access: destack._generated.mir.tree.memory.AtomicAccess
    kind: typing.Literal["atomicLoad"] = "atomicLoad"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionAtomicStore:
    """Store to memory atomically."""

    # the pointer to store to
    pointer: destack._generated.mir.tree.value.Value
    # the value to store
    value: destack._generated.mir.tree.value.Value
    # the atomic access
    access: destack._generated.mir.tree.memory.AtomicAccess
    kind: typing.Literal["atomicStore"] = "atomicStore"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionAtomicCompareExchange:
    """Compare exchange one memory location atomically."""

    # the SSA value to define with the old value and success flag
    destination: destack._generated.mir.tree.value.Value
    # the pointer to update
    pointer: destack._generated.mir.tree.value.Value
    # the expected current value
    expected: destack._generated.mir.tree.value.Value
    # the replacement value
    new_value: destack._generated.mir.tree.value.Value
    # whether the compare exchange is weak
    is_weak: bool
    # the compare exchange access
    access: destack._generated.mir.tree.memory.CompareExchangeAccess
    kind: typing.Literal["atomicCompareExchange"] = "atomicCompareExchange"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionAtomicRmw:
    """Apply one atomic read modify write operation."""

    # the SSA value to define with the old value
    destination: destack._generated.mir.tree.value.Value
    # the read modify write operator
    operator: destack._generated.mir.tree.memory.AtomicRmwOperator
    # the pointer to update
    pointer: destack._generated.mir.tree.value.Value
    # the value argument for the operator
    value: destack._generated.mir.tree.value.Value
    # the atomic access
    access: destack._generated.mir.tree.memory.AtomicAccess
    kind: typing.Literal["atomicRmw"] = "atomicRmw"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionAtomicFence:
    """Publish one memory fence."""

    # the fence access
    access: destack._generated.mir.tree.memory.FenceAccess
    kind: typing.Literal["atomicFence"] = "atomicFence"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionAssume:
    """Assume a condition is true (UB if false)."""

    # the condition to assume
    condition: destack._generated.mir.tree.value.Value
    kind: typing.Literal["assume"] = "assume"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionProfileIncrement:
    """Increment one profile counter."""

    # the counter to increment
    counter: destack._generated.mir.metadata.profile.CounterId
    kind: typing.Literal["profileIncrement"] = "profileIncrement"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionProfileValue:
    """Record one profiled runtime value."""

    # the counter receiving the sampled value
    counter: destack._generated.mir.metadata.profile.CounterId
    # the sampled MIR value
    value: destack._generated.mir.tree.value.Value
    kind: typing.Literal["profileValue"] = "profileValue"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class InstructionIntrinsic:
    """Call a compiler intrinsic."""

    # the SSA value to define with the result, if any
    destination: destack._generated.mir.tree.value.Value | None
    # the intrinsic to call
    intrinsic: destack._generated.mir.tree.intrinsic.Intrinsic
    # the arguments to pass
    arguments: destack._generated.mir.tree.value.ValueSlice
    kind: typing.Literal["intrinsic"] = "intrinsic"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Instructions produce SSA values and perform "operations"."""
Instruction: typing.TypeAlias = (
    InstructionError
    | InstructionConst
    | InstructionBinary
    | InstructionUnary
    | InstructionCast
    | InstructionSelect
    | InstructionLocalGet
    | InstructionLocalAddr
    | InstructionLocalSet
    | InstructionGlobalAddr
    | InstructionFunctionAddr
    | InstructionFunctionBind
    | InstructionFunctionPointer
    | InstructionFunctionEnvironment
    | InstructionFunctionEnvironmentCurrent
    | InstructionLoad
    | InstructionStore
    | InstructionStruct
    | InstructionTuple
    | InstructionArray
    | InstructionFieldGet
    | InstructionFieldSet
    | InstructionFieldAddr
    | InstructionElementAddr
    | InstructionSliceView
    | InstructionSliceLength
    | InstructionDynamicPayload
    | InstructionDynamicType
    | InstructionVariantTag
    | InstructionVariantPayload
    | InstructionVectorSplat
    | InstructionVectorExtract
    | InstructionVectorInsert
    | InstructionVectorShuffle
    | InstructionVectorSelect
    | InstructionVectorReduce
    | InstructionVectorCompare
    | InstructionVectorConvert
    | InstructionTensorSplat
    | InstructionTensorLoad
    | InstructionTensorExtract
    | InstructionTensorStore
    | InstructionTensorFill
    | InstructionTensorCopy
    | InstructionTensorReshape
    | InstructionTensorBroadcast
    | InstructionTensorTranspose
    | InstructionTensorCast
    | InstructionTensorView
    | InstructionTensorSlice
    | InstructionTensorPad
    | InstructionTensorConcat
    | InstructionTensorCompare
    | InstructionTensorSelect
    | InstructionTensorReduce
    | InstructionTensorIndexReduce
    | InstructionTensorDot
    | InstructionTensorConvolution
    | InstructionTensorGather
    | InstructionTensorScatter
    | InstructionTensorConvert
    | InstructionCall
    | InstructionCallVirtual
    | InstructionCallDynamic
    | InstructionCallIndirect
    | InstructionNewZeroed
    | InstructionNewUninit
    | InstructionNewComplete
    | InstructionNewSliceZeroed
    | InstructionNewSliceUninit
    | InstructionFree
    | InstructionFrameAllocZeroed
    | InstructionFrameAllocUninit
    | InstructionPin
    | InstructionUnpin
    | InstructionBarrierWrite
    | InstructionAtomicLoad
    | InstructionAtomicStore
    | InstructionAtomicCompareExchange
    | InstructionAtomicRmw
    | InstructionAtomicFence
    | InstructionAssume
    | InstructionProfileIncrement
    | InstructionProfileValue
    | InstructionIntrinsic
)

def encode_instruction(writer: BinaryWriter, value: Instruction) -> None: ...
def decode_instruction(reader: BinaryReader) -> Instruction: ...
def to_json_instruction(value: Instruction) -> Json: ...
def from_json_instruction(value: Json) -> Instruction: ...

"""Kind of type cast."""
CastOperator: typing.TypeAlias = (
    typing.Literal["bitcast"]
    | typing.Literal["truncate"]
    | typing.Literal["zeroExtend"]
    | typing.Literal["signExtend"]
    | typing.Literal["floatToSignedInt"]
    | typing.Literal["floatToUnsignedInt"]
    | typing.Literal["floatToSignedIntSaturating"]
    | typing.Literal["floatToUnsignedIntSaturating"]
    | typing.Literal["signedIntToFloat"]
    | typing.Literal["unsignedIntToFloat"]
    | typing.Literal["floatTruncate"]
    | typing.Literal["floatExtend"]
    | typing.Literal["floatConvert"]
    | typing.Literal["pointerToInt"]
    | typing.Literal["intToPointer"]
)

def encode_cast_operator(writer: BinaryWriter, value: CastOperator) -> None: ...
def decode_cast_operator(reader: BinaryReader) -> CastOperator: ...
def to_json_cast_operator(value: CastOperator) -> Json: ...
def from_json_cast_operator(value: Json) -> CastOperator: ...

__all__ = [
    "Instruction",
    "encode_instruction",
    "decode_instruction",
    "to_json_instruction",
    "from_json_instruction",
    "InstructionError",
    "InstructionConst",
    "InstructionBinary",
    "InstructionUnary",
    "InstructionCast",
    "InstructionSelect",
    "InstructionLocalGet",
    "InstructionLocalAddr",
    "InstructionLocalSet",
    "InstructionGlobalAddr",
    "InstructionFunctionAddr",
    "InstructionFunctionBind",
    "InstructionFunctionPointer",
    "InstructionFunctionEnvironment",
    "InstructionFunctionEnvironmentCurrent",
    "InstructionLoad",
    "InstructionStore",
    "InstructionStruct",
    "InstructionTuple",
    "InstructionArray",
    "InstructionFieldGet",
    "InstructionFieldSet",
    "InstructionFieldAddr",
    "InstructionElementAddr",
    "InstructionSliceView",
    "InstructionSliceLength",
    "InstructionDynamicPayload",
    "InstructionDynamicType",
    "InstructionVariantTag",
    "InstructionVariantPayload",
    "InstructionVectorSplat",
    "InstructionVectorExtract",
    "InstructionVectorInsert",
    "InstructionVectorShuffle",
    "InstructionVectorSelect",
    "InstructionVectorReduce",
    "InstructionVectorCompare",
    "InstructionVectorConvert",
    "InstructionTensorSplat",
    "InstructionTensorLoad",
    "InstructionTensorExtract",
    "InstructionTensorStore",
    "InstructionTensorFill",
    "InstructionTensorCopy",
    "InstructionTensorReshape",
    "InstructionTensorBroadcast",
    "InstructionTensorTranspose",
    "InstructionTensorCast",
    "InstructionTensorView",
    "InstructionTensorSlice",
    "InstructionTensorPad",
    "InstructionTensorConcat",
    "InstructionTensorCompare",
    "InstructionTensorSelect",
    "InstructionTensorReduce",
    "InstructionTensorIndexReduce",
    "InstructionTensorDot",
    "InstructionTensorConvolution",
    "InstructionTensorGather",
    "InstructionTensorScatter",
    "InstructionTensorConvert",
    "InstructionCall",
    "InstructionCallVirtual",
    "InstructionCallDynamic",
    "InstructionCallIndirect",
    "InstructionNewZeroed",
    "InstructionNewUninit",
    "InstructionNewComplete",
    "InstructionNewSliceZeroed",
    "InstructionNewSliceUninit",
    "InstructionFree",
    "InstructionFrameAllocZeroed",
    "InstructionFrameAllocUninit",
    "InstructionPin",
    "InstructionUnpin",
    "InstructionBarrierWrite",
    "InstructionAtomicLoad",
    "InstructionAtomicStore",
    "InstructionAtomicCompareExchange",
    "InstructionAtomicRmw",
    "InstructionAtomicFence",
    "InstructionAssume",
    "InstructionProfileIncrement",
    "InstructionProfileValue",
    "InstructionIntrinsic",
    "CastOperator",
    "encode_cast_operator",
    "decode_cast_operator",
    "to_json_cast_operator",
    "from_json_cast_operator",
]
