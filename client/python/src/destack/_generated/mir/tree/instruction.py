# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_bool,
    json_field,
    json_int,
    json_object,
    json_optional,
    json_string,
)

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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


@dataclass(frozen=True, slots=True)
class InstructionConst:
    """Load a constant value."""

    # the SSA value to define
    destination: destack._generated.mir.tree.value.Value
    # the constant value to load
    value: destack._generated.mir.tree.constant.Constant
    kind: typing.Literal["const"] = "const"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


@dataclass(frozen=True, slots=True)
class InstructionLocalGet:
    """Load from a local variable (stack slot)."""

    # the SSA value to define with the loaded value
    destination: destack._generated.mir.tree.value.Value
    # the local variable to load from
    local: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["localGet"] = "localGet"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


@dataclass(frozen=True, slots=True)
class InstructionLocalSet:
    """Store to a local variable (stack slot)."""

    # the local variable to store to
    local: destack._generated.mir.tree.node.LocalNodeId
    # the value to store
    value: destack._generated.mir.tree.value.Value
    kind: typing.Literal["localSet"] = "localSet"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


@dataclass(frozen=True, slots=True)
class InstructionFunctionAddr:
    """Get a function pointer for a function (function.address)."""

    # the SSA value to define with the function pointer
    destination: destack._generated.mir.tree.value.Value
    # the function to take the address of
    function: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["functionAddr"] = "functionAddr"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


@dataclass(frozen=True, slots=True)
class InstructionFunctionPointer:
    """Project the function pointer from one function value (function.pointer)."""

    # the SSA value to define with the function pointer
    destination: destack._generated.mir.tree.value.Value
    # the function value to project
    function: destack._generated.mir.tree.value.Value
    kind: typing.Literal["functionPointer"] = "functionPointer"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


@dataclass(frozen=True, slots=True)
class InstructionFunctionEnvironment:
    """Project the environment from one function value (function.environment)."""

    # the SSA value to define with the environment
    destination: destack._generated.mir.tree.value.Value
    # the function value to project
    function: destack._generated.mir.tree.value.Value
    kind: typing.Literal["functionEnvironment"] = "functionEnvironment"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


@dataclass(frozen=True, slots=True)
class InstructionFunctionEnvironmentCurrent:
    """Load the hidden environment for the current function (function.environment.current)."""

    # the SSA value to define with the hidden environment pointer
    destination: destack._generated.mir.tree.value.Value
    kind: typing.Literal["functionEnvironmentCurrent"] = "functionEnvironmentCurrent"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


@dataclass(frozen=True, slots=True)
class InstructionStore:
    """Store to a pointer (write through pointer)."""

    # the pointer to store to
    pointer: destack._generated.mir.tree.value.Value
    # the value to store
    value: destack._generated.mir.tree.value.Value
    kind: typing.Literal["store"] = "store"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


@dataclass(frozen=True, slots=True)
class InstructionSliceLength:
    """Read the runtime length from a slice descriptor."""

    # the SSA value to define with the length
    destination: destack._generated.mir.tree.value.Value
    # the slice value whose length is read
    slice: destack._generated.mir.tree.value.Value
    kind: typing.Literal["sliceLength"] = "sliceLength"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


@dataclass(frozen=True, slots=True)
class InstructionDynamicType:
    """Read the concrete type id from a dynamic value."""

    # the SSA value to define with the type id
    destination: destack._generated.mir.tree.value.Value
    # the dynamic value whose concrete type is read
    dynamic: destack._generated.mir.tree.value.Value
    kind: typing.Literal["dynamicType"] = "dynamicType"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


@dataclass(frozen=True, slots=True)
class InstructionVariantTag:
    """Read the active tag from a physical tagged sum value."""

    # the SSA value to define with the active tag
    destination: destack._generated.mir.tree.value.Value
    # the variant value whose tag is read
    variant: destack._generated.mir.tree.value.Value
    kind: typing.Literal["variantTag"] = "variantTag"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


@dataclass(frozen=True, slots=True)
class InstructionVectorSplat:
    """Broadcast a scalar to all vector lanes."""

    # the SSA value to define with the vector result
    destination: destack._generated.mir.tree.value.Value
    # the scalar value to broadcast
    value: destack._generated.mir.tree.value.Value
    kind: typing.Literal["vectorSplat"] = "vectorSplat"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


@dataclass(frozen=True, slots=True)
class InstructionTensorSplat:
    """Broadcast a scalar to all tensor elements."""

    # the SSA value to define with the tensor result
    destination: destack._generated.mir.tree.value.Value
    # the scalar value to broadcast
    value: destack._generated.mir.tree.value.Value
    kind: typing.Literal["tensorSplat"] = "tensorSplat"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


@dataclass(frozen=True, slots=True)
class InstructionTensorFill:
    """Fill a tensor reference with a scalar value."""

    # the tensor reference to fill
    view: destack._generated.mir.tree.value.Value
    # the scalar value to write
    value: destack._generated.mir.tree.value.Value
    kind: typing.Literal["tensorFill"] = "tensorFill"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


@dataclass(frozen=True, slots=True)
class InstructionTensorCopy:
    """Copy elements from a source tensor reference into a destination tensor reference."""

    # the destination tensor reference
    target: destack._generated.mir.tree.value.Value
    # the source tensor reference
    source: destack._generated.mir.tree.value.Value
    kind: typing.Literal["tensorCopy"] = "tensorCopy"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


@dataclass(frozen=True, slots=True)
class InstructionTensorCast:
    """Refine a tensor type without changing its contents."""

    # the SSA value to define with the cast tensor
    destination: destack._generated.mir.tree.value.Value
    # the tensor value to cast
    tensor: destack._generated.mir.tree.value.Value
    kind: typing.Literal["tensorCast"] = "tensorCast"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


@dataclass(frozen=True, slots=True)
class InstructionFree:
    """Release unique heap storage (`free`)."""

    # the unique heap reference to free
    value: destack._generated.mir.tree.value.Value
    kind: typing.Literal["free"] = "free"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


@dataclass(frozen=True, slots=True)
class InstructionUnpin:
    """Release one heap pin (`unpin`)."""

    # the heap value to unpin
    value: destack._generated.mir.tree.value.Value
    kind: typing.Literal["unpin"] = "unpin"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


@dataclass(frozen=True, slots=True)
class InstructionAtomicFence:
    """Publish one memory fence."""

    # the fence access
    access: destack._generated.mir.tree.memory.FenceAccess
    kind: typing.Literal["atomicFence"] = "atomicFence"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


@dataclass(frozen=True, slots=True)
class InstructionAssume:
    """Assume a condition is true (UB if false)."""

    # the condition to assume
    condition: destack._generated.mir.tree.value.Value
    kind: typing.Literal["assume"] = "assume"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


@dataclass(frozen=True, slots=True)
class InstructionProfileIncrement:
    """Increment one profile counter."""

    # the counter to increment
    counter: destack._generated.mir.metadata.profile.CounterId
    kind: typing.Literal["profileIncrement"] = "profileIncrement"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


@dataclass(frozen=True, slots=True)
class InstructionProfileValue:
    """Record one profiled runtime value."""

    # the counter receiving the sampled value
    counter: destack._generated.mir.metadata.profile.CounterId
    # the sampled MIR value
    value: destack._generated.mir.tree.value.Value
    kind: typing.Literal["profileValue"] = "profileValue"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instruction(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instruction(self)


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


def encode_instruction(writer: BinaryWriter, value: Instruction) -> None:
    """Encode one Instruction."""
    if value.kind == "error":
        writer.write_unsigned(0)
    elif value.kind == "const":
        writer.write_unsigned(1)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.constant.encode_constant(writer, value.value)
    elif value.kind == "binary":
        writer.write_unsigned(2)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.operator.encode_binary_operator(
            writer, value.operator
        )
        destack._generated.mir.tree.value.encode_value(writer, value.left)
        destack._generated.mir.tree.value.encode_value(writer, value.right)
    elif value.kind == "unary":
        writer.write_unsigned(3)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.operator.encode_unary_operator(
            writer, value.operator
        )
        destack._generated.mir.tree.value.encode_value(writer, value.argument)
    elif value.kind == "cast":
        writer.write_unsigned(4)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        encode_cast_operator(writer, value.operator)
        destack._generated.mir.tree.value.encode_value(writer, value.argument)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.to_type)
    elif value.kind == "select":
        writer.write_unsigned(5)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value(writer, value.condition)
        destack._generated.mir.tree.value.encode_value(writer, value.then_value)
        destack._generated.mir.tree.value.encode_value(writer, value.else_value)
    elif value.kind == "localGet":
        writer.write_unsigned(6)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.local)
    elif value.kind == "localAddr":
        writer.write_unsigned(7)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.local)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.result_type)
    elif value.kind == "localSet":
        writer.write_unsigned(8)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.local)
        destack._generated.mir.tree.value.encode_value(writer, value.value)
    elif value.kind == "globalAddr":
        writer.write_unsigned(9)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.global_)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.result_type)
    elif value.kind == "functionAddr":
        writer.write_unsigned(10)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.function)
    elif value.kind == "functionBind":
        writer.write_unsigned(11)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.function)
        destack._generated.mir.tree.value.encode_value(writer, value.environment)
    elif value.kind == "functionPointer":
        writer.write_unsigned(12)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value(writer, value.function)
    elif value.kind == "functionEnvironment":
        writer.write_unsigned(13)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value(writer, value.function)
    elif value.kind == "functionEnvironmentCurrent":
        writer.write_unsigned(14)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
    elif value.kind == "load":
        writer.write_unsigned(15)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value(writer, value.pointer)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.result_type)
    elif value.kind == "store":
        writer.write_unsigned(16)
        destack._generated.mir.tree.value.encode_value(writer, value.pointer)
        destack._generated.mir.tree.value.encode_value(writer, value.value)
    elif value.kind == "struct":
        writer.write_unsigned(17)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.ty)
        destack._generated.mir.tree.value.encode_value_slice(writer, value.fields)
    elif value.kind == "tuple":
        writer.write_unsigned(18)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.ty)
        destack._generated.mir.tree.value.encode_value_slice(writer, value.elements)
    elif value.kind == "array":
        writer.write_unsigned(19)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.ty)
        destack._generated.mir.tree.value.encode_value_slice(writer, value.elements)
    elif value.kind == "fieldGet":
        writer.write_unsigned(20)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value(writer, value.aggregate)
        writer.write_unsigned(value.index)
    elif value.kind == "fieldSet":
        writer.write_unsigned(21)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value(writer, value.aggregate)
        writer.write_unsigned(value.index)
        destack._generated.mir.tree.value.encode_value(writer, value.value)
    elif value.kind == "fieldAddr":
        writer.write_unsigned(22)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value(writer, value.aggregate)
        writer.write_unsigned(value.index)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.result_type)
    elif value.kind == "elementAddr":
        writer.write_unsigned(23)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value(writer, value.array)
        destack._generated.mir.tree.value.encode_value(writer, value.index)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.result_type)
    elif value.kind == "sliceView":
        writer.write_unsigned(24)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value(writer, value.source)
        destack._generated.mir.tree.value.encode_value(writer, value.start)
        destack._generated.mir.tree.value.encode_value(writer, value.length)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.result_type)
    elif value.kind == "sliceLength":
        writer.write_unsigned(25)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value(writer, value.slice)
    elif value.kind == "dynamicPayload":
        writer.write_unsigned(26)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value(writer, value.dynamic)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.result_type)
    elif value.kind == "dynamicType":
        writer.write_unsigned(27)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value(writer, value.dynamic)
    elif value.kind == "variantTag":
        writer.write_unsigned(28)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value(writer, value.variant)
    elif value.kind == "variantPayload":
        writer.write_unsigned(29)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value(writer, value.variant)
        destack._generated.mir.tree.constant.encode_constant(writer, value.tag)
    elif value.kind == "vectorSplat":
        writer.write_unsigned(30)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value(writer, value.value)
    elif value.kind == "vectorExtract":
        writer.write_unsigned(31)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value(writer, value.vector)
        destack._generated.mir.tree.value.encode_value(writer, value.index)
    elif value.kind == "vectorInsert":
        writer.write_unsigned(32)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value(writer, value.vector)
        destack._generated.mir.tree.value.encode_value(writer, value.index)
        destack._generated.mir.tree.value.encode_value(writer, value.value)
    elif value.kind == "vectorShuffle":
        writer.write_unsigned(33)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value(writer, value.left)
        destack._generated.mir.tree.value.encode_value(writer, value.right)
        destack._generated.mir.tree.immediate.encode_index_slice(writer, value.mask)
    elif value.kind == "vectorSelect":
        writer.write_unsigned(34)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value(writer, value.mask)
        destack._generated.mir.tree.value.encode_value(writer, value.then_value)
        destack._generated.mir.tree.value.encode_value(writer, value.else_value)
    elif value.kind == "vectorReduce":
        writer.write_unsigned(35)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.vector.encode_vector_reduce_operator(
            writer, value.operator
        )
        destack._generated.mir.tree.value.encode_value(writer, value.vector)
    elif value.kind == "vectorCompare":
        writer.write_unsigned(36)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.operator.encode_binary_operator(
            writer, value.operator
        )
        destack._generated.mir.tree.value.encode_value(writer, value.left)
        destack._generated.mir.tree.value.encode_value(writer, value.right)
    elif value.kind == "vectorConvert":
        writer.write_unsigned(37)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.vector.encode_vector_convert_mode(
            writer, value.mode
        )
        destack._generated.mir.tree.value.encode_value(writer, value.vector)
    elif value.kind == "tensorSplat":
        writer.write_unsigned(38)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value(writer, value.value)
    elif value.kind == "tensorLoad":
        writer.write_unsigned(39)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value(writer, value.view)
        destack._generated.mir.tree.value.encode_value_slice(writer, value.indices)
    elif value.kind == "tensorExtract":
        writer.write_unsigned(40)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value(writer, value.tensor)
        destack._generated.mir.tree.value.encode_value_slice(writer, value.indices)
    elif value.kind == "tensorStore":
        writer.write_unsigned(41)
        destack._generated.mir.tree.value.encode_value(writer, value.view)
        destack._generated.mir.tree.value.encode_value_slice(writer, value.indices)
        destack._generated.mir.tree.value.encode_value(writer, value.value)
    elif value.kind == "tensorFill":
        writer.write_unsigned(42)
        destack._generated.mir.tree.value.encode_value(writer, value.view)
        destack._generated.mir.tree.value.encode_value(writer, value.value)
    elif value.kind == "tensorCopy":
        writer.write_unsigned(43)
        destack._generated.mir.tree.value.encode_value(writer, value.target)
        destack._generated.mir.tree.value.encode_value(writer, value.source)
    elif value.kind == "tensorReshape":
        writer.write_unsigned(44)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value(writer, value.tensor)
        destack._generated.mir.tree.value.encode_value_slice(writer, value.shape)
    elif value.kind == "tensorBroadcast":
        writer.write_unsigned(45)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value(writer, value.tensor)
        destack._generated.mir.tree.immediate.encode_index_slice(
            writer, value.dimensions
        )
    elif value.kind == "tensorTranspose":
        writer.write_unsigned(46)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value(writer, value.tensor)
        destack._generated.mir.tree.immediate.encode_index_slice(
            writer, value.permutation
        )
    elif value.kind == "tensorCast":
        writer.write_unsigned(47)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value(writer, value.tensor)
    elif value.kind == "tensorView":
        writer.write_unsigned(48)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value(writer, value.view)
        destack._generated.mir.tree.value.encode_value_slice(writer, value.arguments)
        writer.write_unsigned(value.offsets_count)
        writer.write_unsigned(value.sizes_count)
        writer.write_unsigned(value.strides_count)
    elif value.kind == "tensorSlice":
        writer.write_unsigned(49)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value(writer, value.tensor)
        destack._generated.mir.tree.value.encode_value_slice(writer, value.arguments)
        writer.write_unsigned(value.offsets_count)
        writer.write_unsigned(value.sizes_count)
        writer.write_unsigned(value.strides_count)
    elif value.kind == "tensorPad":
        writer.write_unsigned(50)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value(writer, value.tensor)
        destack._generated.mir.tree.value.encode_value_slice(writer, value.arguments)
        writer.write_unsigned(value.low_count)
        writer.write_unsigned(value.high_count)
        writer.write_unsigned(value.interior_count)
        destack._generated.mir.tree.value.encode_value(writer, value.value)
    elif value.kind == "tensorConcat":
        writer.write_unsigned(51)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value_slice(writer, value.tensors)
        writer.write_unsigned(value.axis)
    elif value.kind == "tensorCompare":
        writer.write_unsigned(52)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.operator.encode_binary_operator(
            writer, value.operator
        )
        destack._generated.mir.tree.value.encode_value(writer, value.left)
        destack._generated.mir.tree.value.encode_value(writer, value.right)
    elif value.kind == "tensorSelect":
        writer.write_unsigned(53)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value(writer, value.mask)
        destack._generated.mir.tree.value.encode_value(writer, value.then_value)
        destack._generated.mir.tree.value.encode_value(writer, value.else_value)
    elif value.kind == "tensorReduce":
        writer.write_unsigned(54)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.tensor.encode_tensor_reduce_operator(
            writer, value.operator
        )
        destack._generated.mir.tree.value.encode_value(writer, value.tensor)
        destack._generated.mir.tree.value.encode_value(writer, value.initial)
        destack._generated.mir.tree.immediate.encode_index_slice(writer, value.axes)
    elif value.kind == "tensorIndexReduce":
        writer.write_unsigned(55)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.tensor.encode_tensor_index_reduce_operator(
            writer, value.operator
        )
        destack._generated.mir.tree.value.encode_value(writer, value.tensor)
        writer.write_unsigned(value.axis)
        destack._generated.mir.tree.tensor.encode_tensor_index_tie_break(
            writer, value.tie_break
        )
    elif value.kind == "tensorDot":
        writer.write_unsigned(56)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value(writer, value.left)
        destack._generated.mir.tree.value.encode_value(writer, value.right)
        destack._generated.mir.tree.immediate.encode_tensor_immediate_id(
            writer, value.immediate
        )
    elif value.kind == "tensorConvolution":
        writer.write_unsigned(57)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value(writer, value.input)
        destack._generated.mir.tree.value.encode_value(writer, value.kernel)
        destack._generated.mir.tree.immediate.encode_tensor_immediate_id(
            writer, value.immediate
        )
    elif value.kind == "tensorGather":
        writer.write_unsigned(58)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value(writer, value.operand)
        destack._generated.mir.tree.value.encode_value(writer, value.indices)
        destack._generated.mir.tree.immediate.encode_tensor_immediate_id(
            writer, value.immediate
        )
    elif value.kind == "tensorScatter":
        writer.write_unsigned(59)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value(writer, value.operand)
        destack._generated.mir.tree.value.encode_value(writer, value.indices)
        destack._generated.mir.tree.value.encode_value(writer, value.updates)
        destack._generated.mir.tree.immediate.encode_tensor_immediate_id(
            writer, value.immediate
        )
        destack._generated.mir.tree.tensor.encode_tensor_scatter_mode(
            writer, value.mode
        )
    elif value.kind == "tensorConvert":
        writer.write_unsigned(60)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.tensor.encode_tensor_convert_mode(
            writer, value.mode
        )
        destack._generated.mir.tree.value.encode_value(writer, value.tensor)
    elif value.kind == "call":
        writer.write_unsigned(61)
        if value.destination is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.function)
        destack._generated.mir.tree.call.encode_call(writer, value.call)
    elif value.kind == "callVirtual":
        writer.write_unsigned(62)
        if value.destination is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value(writer, value.receiver)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.class_)
        destack._generated.mir.metadata.dispatch.encode_dispatch_slot(
            writer, value.slot
        )
        destack._generated.mir.tree.call.encode_call(writer, value.call)
    elif value.kind == "callDynamic":
        writer.write_unsigned(63)
        if value.destination is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value(writer, value.receiver)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.constraint)
        destack._generated.mir.metadata.dispatch.encode_dispatch_slot(
            writer, value.slot
        )
        destack._generated.mir.tree.call.encode_call(writer, value.call)
    elif value.kind == "callIndirect":
        writer.write_unsigned(64)
        if value.destination is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value(writer, value.callee)
        destack._generated.mir.tree.call.encode_call(writer, value.call)
    elif value.kind == "newZeroed":
        writer.write_unsigned(65)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.layout)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.result_type)
    elif value.kind == "newUninit":
        writer.write_unsigned(66)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.layout)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.result_type)
    elif value.kind == "newComplete":
        writer.write_unsigned(67)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value(writer, value.value)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.result_type)
    elif value.kind == "newSliceZeroed":
        writer.write_unsigned(68)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.element)
        destack._generated.mir.tree.value.encode_value(writer, value.length)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.result_type)
    elif value.kind == "newSliceUninit":
        writer.write_unsigned(69)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.element)
        destack._generated.mir.tree.value.encode_value(writer, value.length)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.result_type)
    elif value.kind == "free":
        writer.write_unsigned(70)
        destack._generated.mir.tree.value.encode_value(writer, value.value)
    elif value.kind == "frameAllocZeroed":
        writer.write_unsigned(71)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.layout)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.result_type)
    elif value.kind == "frameAllocUninit":
        writer.write_unsigned(72)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.layout)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.result_type)
    elif value.kind == "pin":
        writer.write_unsigned(73)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value(writer, value.value)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.result_type)
    elif value.kind == "unpin":
        writer.write_unsigned(74)
        destack._generated.mir.tree.value.encode_value(writer, value.value)
    elif value.kind == "barrierWrite":
        writer.write_unsigned(75)
        destack._generated.mir.tree.value.encode_value(writer, value.object)
        destack._generated.mir.tree.value.encode_value(writer, value.offset)
        destack._generated.mir.tree.value.encode_value(writer, value.byte_len)
    elif value.kind == "atomicLoad":
        writer.write_unsigned(76)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value(writer, value.pointer)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.result_type)
        destack._generated.mir.tree.memory.encode_atomic_access(writer, value.access)
    elif value.kind == "atomicStore":
        writer.write_unsigned(77)
        destack._generated.mir.tree.value.encode_value(writer, value.pointer)
        destack._generated.mir.tree.value.encode_value(writer, value.value)
        destack._generated.mir.tree.memory.encode_atomic_access(writer, value.access)
    elif value.kind == "atomicCompareExchange":
        writer.write_unsigned(78)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.value.encode_value(writer, value.pointer)
        destack._generated.mir.tree.value.encode_value(writer, value.expected)
        destack._generated.mir.tree.value.encode_value(writer, value.new_value)
        writer.write_bool(value.is_weak)
        destack._generated.mir.tree.memory.encode_compare_exchange_access(
            writer, value.access
        )
    elif value.kind == "atomicRmw":
        writer.write_unsigned(79)
        destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.memory.encode_atomic_rmw_operator(
            writer, value.operator
        )
        destack._generated.mir.tree.value.encode_value(writer, value.pointer)
        destack._generated.mir.tree.value.encode_value(writer, value.value)
        destack._generated.mir.tree.memory.encode_atomic_access(writer, value.access)
    elif value.kind == "atomicFence":
        writer.write_unsigned(80)
        destack._generated.mir.tree.memory.encode_fence_access(writer, value.access)
    elif value.kind == "assume":
        writer.write_unsigned(81)
        destack._generated.mir.tree.value.encode_value(writer, value.condition)
    elif value.kind == "profileIncrement":
        writer.write_unsigned(82)
        destack._generated.mir.metadata.profile.encode_counter_id(writer, value.counter)
    elif value.kind == "profileValue":
        writer.write_unsigned(83)
        destack._generated.mir.metadata.profile.encode_counter_id(writer, value.counter)
        destack._generated.mir.tree.value.encode_value(writer, value.value)
    elif value.kind == "intrinsic":
        writer.write_unsigned(84)
        if value.destination is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.mir.tree.value.encode_value(writer, value.destination)
        destack._generated.mir.tree.intrinsic.encode_intrinsic(writer, value.intrinsic)
        destack._generated.mir.tree.value.encode_value_slice(writer, value.arguments)
    else:
        raise SerdeError("unknown enum variant")


def decode_instruction(reader: BinaryReader) -> Instruction:
    """Decode one Instruction."""
    variant = reader.read_number()

    if variant == 0:
        return InstructionError()
    elif variant == 1:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        value_ = destack._generated.mir.tree.constant.decode_constant(reader)

        return InstructionConst(
            destination=destination,
            value=value_,
        )
    elif variant == 2:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        operator = destack._generated.mir.tree.operator.decode_binary_operator(reader)
        left = destack._generated.mir.tree.value.decode_value(reader)
        right = destack._generated.mir.tree.value.decode_value(reader)

        return InstructionBinary(
            destination=destination,
            operator=operator,
            left=left,
            right=right,
        )
    elif variant == 3:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        operator = destack._generated.mir.tree.operator.decode_unary_operator(reader)
        argument = destack._generated.mir.tree.value.decode_value(reader)

        return InstructionUnary(
            destination=destination,
            operator=operator,
            argument=argument,
        )
    elif variant == 4:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        operator = decode_cast_operator(reader)
        argument = destack._generated.mir.tree.value.decode_value(reader)
        to_type = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return InstructionCast(
            destination=destination,
            operator=operator,
            argument=argument,
            to_type=to_type,
        )
    elif variant == 5:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        condition = destack._generated.mir.tree.value.decode_value(reader)
        then_value = destack._generated.mir.tree.value.decode_value(reader)
        else_value = destack._generated.mir.tree.value.decode_value(reader)

        return InstructionSelect(
            destination=destination,
            condition=condition,
            then_value=then_value,
            else_value=else_value,
        )
    elif variant == 6:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        local = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return InstructionLocalGet(
            destination=destination,
            local=local,
        )
    elif variant == 7:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        local = destack._generated.mir.tree.node.decode_local_node_id(reader)
        result_type = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return InstructionLocalAddr(
            destination=destination,
            local=local,
            result_type=result_type,
        )
    elif variant == 8:
        local = destack._generated.mir.tree.node.decode_local_node_id(reader)
        value_ = destack._generated.mir.tree.value.decode_value(reader)

        return InstructionLocalSet(
            local=local,
            value=value_,
        )
    elif variant == 9:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        global_ = destack._generated.mir.tree.node.decode_local_node_id(reader)
        result_type = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return InstructionGlobalAddr(
            destination=destination,
            global_=global_,
            result_type=result_type,
        )
    elif variant == 10:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        function = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return InstructionFunctionAddr(
            destination=destination,
            function=function,
        )
    elif variant == 11:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        function = destack._generated.mir.tree.node.decode_local_node_id(reader)
        environment = destack._generated.mir.tree.value.decode_value(reader)

        return InstructionFunctionBind(
            destination=destination,
            function=function,
            environment=environment,
        )
    elif variant == 12:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        function = destack._generated.mir.tree.value.decode_value(reader)

        return InstructionFunctionPointer(
            destination=destination,
            function=function,
        )
    elif variant == 13:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        function = destack._generated.mir.tree.value.decode_value(reader)

        return InstructionFunctionEnvironment(
            destination=destination,
            function=function,
        )
    elif variant == 14:
        destination = destack._generated.mir.tree.value.decode_value(reader)

        return InstructionFunctionEnvironmentCurrent(
            destination=destination,
        )
    elif variant == 15:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        pointer = destack._generated.mir.tree.value.decode_value(reader)
        result_type = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return InstructionLoad(
            destination=destination,
            pointer=pointer,
            result_type=result_type,
        )
    elif variant == 16:
        pointer = destack._generated.mir.tree.value.decode_value(reader)
        value_ = destack._generated.mir.tree.value.decode_value(reader)

        return InstructionStore(
            pointer=pointer,
            value=value_,
        )
    elif variant == 17:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        ty = destack._generated.mir.tree.node.decode_local_node_id(reader)
        fields = destack._generated.mir.tree.value.decode_value_slice(reader)

        return InstructionStruct(
            destination=destination,
            ty=ty,
            fields=fields,
        )
    elif variant == 18:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        ty = destack._generated.mir.tree.node.decode_local_node_id(reader)
        elements = destack._generated.mir.tree.value.decode_value_slice(reader)

        return InstructionTuple(
            destination=destination,
            ty=ty,
            elements=elements,
        )
    elif variant == 19:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        ty = destack._generated.mir.tree.node.decode_local_node_id(reader)
        elements = destack._generated.mir.tree.value.decode_value_slice(reader)

        return InstructionArray(
            destination=destination,
            ty=ty,
            elements=elements,
        )
    elif variant == 20:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        aggregate = destack._generated.mir.tree.value.decode_value(reader)
        index = reader.read_number()

        return InstructionFieldGet(
            destination=destination,
            aggregate=aggregate,
            index=index,
        )
    elif variant == 21:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        aggregate = destack._generated.mir.tree.value.decode_value(reader)
        index = reader.read_number()
        value_ = destack._generated.mir.tree.value.decode_value(reader)

        return InstructionFieldSet(
            destination=destination,
            aggregate=aggregate,
            index=index,
            value=value_,
        )
    elif variant == 22:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        aggregate = destack._generated.mir.tree.value.decode_value(reader)
        index = reader.read_number()
        result_type = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return InstructionFieldAddr(
            destination=destination,
            aggregate=aggregate,
            index=index,
            result_type=result_type,
        )
    elif variant == 23:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        array = destack._generated.mir.tree.value.decode_value(reader)
        index = destack._generated.mir.tree.value.decode_value(reader)
        result_type = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return InstructionElementAddr(
            destination=destination,
            array=array,
            index=index,
            result_type=result_type,
        )
    elif variant == 24:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        source = destack._generated.mir.tree.value.decode_value(reader)
        start = destack._generated.mir.tree.value.decode_value(reader)
        length = destack._generated.mir.tree.value.decode_value(reader)
        result_type = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return InstructionSliceView(
            destination=destination,
            source=source,
            start=start,
            length=length,
            result_type=result_type,
        )
    elif variant == 25:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        slice = destack._generated.mir.tree.value.decode_value(reader)

        return InstructionSliceLength(
            destination=destination,
            slice=slice,
        )
    elif variant == 26:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        dynamic = destack._generated.mir.tree.value.decode_value(reader)
        result_type = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return InstructionDynamicPayload(
            destination=destination,
            dynamic=dynamic,
            result_type=result_type,
        )
    elif variant == 27:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        dynamic = destack._generated.mir.tree.value.decode_value(reader)

        return InstructionDynamicType(
            destination=destination,
            dynamic=dynamic,
        )
    elif variant == 28:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        variant = destack._generated.mir.tree.value.decode_value(reader)

        return InstructionVariantTag(
            destination=destination,
            variant=variant,
        )
    elif variant == 29:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        variant = destack._generated.mir.tree.value.decode_value(reader)
        tag = destack._generated.mir.tree.constant.decode_constant(reader)

        return InstructionVariantPayload(
            destination=destination,
            variant=variant,
            tag=tag,
        )
    elif variant == 30:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        value_ = destack._generated.mir.tree.value.decode_value(reader)

        return InstructionVectorSplat(
            destination=destination,
            value=value_,
        )
    elif variant == 31:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        vector = destack._generated.mir.tree.value.decode_value(reader)
        index = destack._generated.mir.tree.value.decode_value(reader)

        return InstructionVectorExtract(
            destination=destination,
            vector=vector,
            index=index,
        )
    elif variant == 32:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        vector = destack._generated.mir.tree.value.decode_value(reader)
        index = destack._generated.mir.tree.value.decode_value(reader)
        value_ = destack._generated.mir.tree.value.decode_value(reader)

        return InstructionVectorInsert(
            destination=destination,
            vector=vector,
            index=index,
            value=value_,
        )
    elif variant == 33:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        left = destack._generated.mir.tree.value.decode_value(reader)
        right = destack._generated.mir.tree.value.decode_value(reader)
        mask = destack._generated.mir.tree.immediate.decode_index_slice(reader)

        return InstructionVectorShuffle(
            destination=destination,
            left=left,
            right=right,
            mask=mask,
        )
    elif variant == 34:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        mask = destack._generated.mir.tree.value.decode_value(reader)
        then_value = destack._generated.mir.tree.value.decode_value(reader)
        else_value = destack._generated.mir.tree.value.decode_value(reader)

        return InstructionVectorSelect(
            destination=destination,
            mask=mask,
            then_value=then_value,
            else_value=else_value,
        )
    elif variant == 35:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        operator = destack._generated.mir.tree.vector.decode_vector_reduce_operator(
            reader
        )
        vector = destack._generated.mir.tree.value.decode_value(reader)

        return InstructionVectorReduce(
            destination=destination,
            operator=operator,
            vector=vector,
        )
    elif variant == 36:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        operator = destack._generated.mir.tree.operator.decode_binary_operator(reader)
        left = destack._generated.mir.tree.value.decode_value(reader)
        right = destack._generated.mir.tree.value.decode_value(reader)

        return InstructionVectorCompare(
            destination=destination,
            operator=operator,
            left=left,
            right=right,
        )
    elif variant == 37:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        mode = destack._generated.mir.tree.vector.decode_vector_convert_mode(reader)
        vector = destack._generated.mir.tree.value.decode_value(reader)

        return InstructionVectorConvert(
            destination=destination,
            mode=mode,
            vector=vector,
        )
    elif variant == 38:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        value_ = destack._generated.mir.tree.value.decode_value(reader)

        return InstructionTensorSplat(
            destination=destination,
            value=value_,
        )
    elif variant == 39:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        view = destack._generated.mir.tree.value.decode_value(reader)
        indices = destack._generated.mir.tree.value.decode_value_slice(reader)

        return InstructionTensorLoad(
            destination=destination,
            view=view,
            indices=indices,
        )
    elif variant == 40:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        tensor = destack._generated.mir.tree.value.decode_value(reader)
        indices = destack._generated.mir.tree.value.decode_value_slice(reader)

        return InstructionTensorExtract(
            destination=destination,
            tensor=tensor,
            indices=indices,
        )
    elif variant == 41:
        view = destack._generated.mir.tree.value.decode_value(reader)
        indices = destack._generated.mir.tree.value.decode_value_slice(reader)
        value_ = destack._generated.mir.tree.value.decode_value(reader)

        return InstructionTensorStore(
            view=view,
            indices=indices,
            value=value_,
        )
    elif variant == 42:
        view = destack._generated.mir.tree.value.decode_value(reader)
        value_ = destack._generated.mir.tree.value.decode_value(reader)

        return InstructionTensorFill(
            view=view,
            value=value_,
        )
    elif variant == 43:
        target = destack._generated.mir.tree.value.decode_value(reader)
        source = destack._generated.mir.tree.value.decode_value(reader)

        return InstructionTensorCopy(
            target=target,
            source=source,
        )
    elif variant == 44:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        tensor = destack._generated.mir.tree.value.decode_value(reader)
        shape = destack._generated.mir.tree.value.decode_value_slice(reader)

        return InstructionTensorReshape(
            destination=destination,
            tensor=tensor,
            shape=shape,
        )
    elif variant == 45:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        tensor = destack._generated.mir.tree.value.decode_value(reader)
        dimensions = destack._generated.mir.tree.immediate.decode_index_slice(reader)

        return InstructionTensorBroadcast(
            destination=destination,
            tensor=tensor,
            dimensions=dimensions,
        )
    elif variant == 46:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        tensor = destack._generated.mir.tree.value.decode_value(reader)
        permutation = destack._generated.mir.tree.immediate.decode_index_slice(reader)

        return InstructionTensorTranspose(
            destination=destination,
            tensor=tensor,
            permutation=permutation,
        )
    elif variant == 47:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        tensor = destack._generated.mir.tree.value.decode_value(reader)

        return InstructionTensorCast(
            destination=destination,
            tensor=tensor,
        )
    elif variant == 48:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        view = destack._generated.mir.tree.value.decode_value(reader)
        arguments = destack._generated.mir.tree.value.decode_value_slice(reader)
        offsets_count = reader.read_number()
        sizes_count = reader.read_number()
        strides_count = reader.read_number()

        return InstructionTensorView(
            destination=destination,
            view=view,
            arguments=arguments,
            offsets_count=offsets_count,
            sizes_count=sizes_count,
            strides_count=strides_count,
        )
    elif variant == 49:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        tensor = destack._generated.mir.tree.value.decode_value(reader)
        arguments = destack._generated.mir.tree.value.decode_value_slice(reader)
        offsets_count = reader.read_number()
        sizes_count = reader.read_number()
        strides_count = reader.read_number()

        return InstructionTensorSlice(
            destination=destination,
            tensor=tensor,
            arguments=arguments,
            offsets_count=offsets_count,
            sizes_count=sizes_count,
            strides_count=strides_count,
        )
    elif variant == 50:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        tensor = destack._generated.mir.tree.value.decode_value(reader)
        arguments = destack._generated.mir.tree.value.decode_value_slice(reader)
        low_count = reader.read_number()
        high_count = reader.read_number()
        interior_count = reader.read_number()
        value_ = destack._generated.mir.tree.value.decode_value(reader)

        return InstructionTensorPad(
            destination=destination,
            tensor=tensor,
            arguments=arguments,
            low_count=low_count,
            high_count=high_count,
            interior_count=interior_count,
            value=value_,
        )
    elif variant == 51:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        tensors = destack._generated.mir.tree.value.decode_value_slice(reader)
        axis = reader.read_number()

        return InstructionTensorConcat(
            destination=destination,
            tensors=tensors,
            axis=axis,
        )
    elif variant == 52:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        operator = destack._generated.mir.tree.operator.decode_binary_operator(reader)
        left = destack._generated.mir.tree.value.decode_value(reader)
        right = destack._generated.mir.tree.value.decode_value(reader)

        return InstructionTensorCompare(
            destination=destination,
            operator=operator,
            left=left,
            right=right,
        )
    elif variant == 53:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        mask = destack._generated.mir.tree.value.decode_value(reader)
        then_value = destack._generated.mir.tree.value.decode_value(reader)
        else_value = destack._generated.mir.tree.value.decode_value(reader)

        return InstructionTensorSelect(
            destination=destination,
            mask=mask,
            then_value=then_value,
            else_value=else_value,
        )
    elif variant == 54:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        operator = destack._generated.mir.tree.tensor.decode_tensor_reduce_operator(
            reader
        )
        tensor = destack._generated.mir.tree.value.decode_value(reader)
        initial = destack._generated.mir.tree.value.decode_value(reader)
        axes = destack._generated.mir.tree.immediate.decode_index_slice(reader)

        return InstructionTensorReduce(
            destination=destination,
            operator=operator,
            tensor=tensor,
            initial=initial,
            axes=axes,
        )
    elif variant == 55:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        operator = (
            destack._generated.mir.tree.tensor.decode_tensor_index_reduce_operator(
                reader
            )
        )
        tensor = destack._generated.mir.tree.value.decode_value(reader)
        axis = reader.read_number()
        tie_break = destack._generated.mir.tree.tensor.decode_tensor_index_tie_break(
            reader
        )

        return InstructionTensorIndexReduce(
            destination=destination,
            operator=operator,
            tensor=tensor,
            axis=axis,
            tie_break=tie_break,
        )
    elif variant == 56:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        left = destack._generated.mir.tree.value.decode_value(reader)
        right = destack._generated.mir.tree.value.decode_value(reader)
        immediate = destack._generated.mir.tree.immediate.decode_tensor_immediate_id(
            reader
        )

        return InstructionTensorDot(
            destination=destination,
            left=left,
            right=right,
            immediate=immediate,
        )
    elif variant == 57:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        input = destack._generated.mir.tree.value.decode_value(reader)
        kernel = destack._generated.mir.tree.value.decode_value(reader)
        immediate = destack._generated.mir.tree.immediate.decode_tensor_immediate_id(
            reader
        )

        return InstructionTensorConvolution(
            destination=destination,
            input=input,
            kernel=kernel,
            immediate=immediate,
        )
    elif variant == 58:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        operand = destack._generated.mir.tree.value.decode_value(reader)
        indices = destack._generated.mir.tree.value.decode_value(reader)
        immediate = destack._generated.mir.tree.immediate.decode_tensor_immediate_id(
            reader
        )

        return InstructionTensorGather(
            destination=destination,
            operand=operand,
            indices=indices,
            immediate=immediate,
        )
    elif variant == 59:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        operand = destack._generated.mir.tree.value.decode_value(reader)
        indices = destack._generated.mir.tree.value.decode_value(reader)
        updates = destack._generated.mir.tree.value.decode_value(reader)
        immediate = destack._generated.mir.tree.immediate.decode_tensor_immediate_id(
            reader
        )
        mode = destack._generated.mir.tree.tensor.decode_tensor_scatter_mode(reader)

        return InstructionTensorScatter(
            destination=destination,
            operand=operand,
            indices=indices,
            updates=updates,
            immediate=immediate,
            mode=mode,
        )
    elif variant == 60:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        mode = destack._generated.mir.tree.tensor.decode_tensor_convert_mode(reader)
        tensor = destack._generated.mir.tree.value.decode_value(reader)

        return InstructionTensorConvert(
            destination=destination,
            mode=mode,
            tensor=tensor,
        )
    elif variant == 61:
        destination = reader.read_option(
            lambda: destack._generated.mir.tree.value.decode_value(reader)
        )
        function = destack._generated.mir.tree.node.decode_local_node_id(reader)
        call = destack._generated.mir.tree.call.decode_call(reader)

        return InstructionCall(
            destination=destination,
            function=function,
            call=call,
        )
    elif variant == 62:
        destination = reader.read_option(
            lambda: destack._generated.mir.tree.value.decode_value(reader)
        )
        receiver = destack._generated.mir.tree.value.decode_value(reader)
        class_ = destack._generated.mir.tree.node.decode_local_node_id(reader)
        slot = destack._generated.mir.metadata.dispatch.decode_dispatch_slot(reader)
        call = destack._generated.mir.tree.call.decode_call(reader)

        return InstructionCallVirtual(
            destination=destination,
            receiver=receiver,
            class_=class_,
            slot=slot,
            call=call,
        )
    elif variant == 63:
        destination = reader.read_option(
            lambda: destack._generated.mir.tree.value.decode_value(reader)
        )
        receiver = destack._generated.mir.tree.value.decode_value(reader)
        constraint = destack._generated.mir.tree.node.decode_local_node_id(reader)
        slot = destack._generated.mir.metadata.dispatch.decode_dispatch_slot(reader)
        call = destack._generated.mir.tree.call.decode_call(reader)

        return InstructionCallDynamic(
            destination=destination,
            receiver=receiver,
            constraint=constraint,
            slot=slot,
            call=call,
        )
    elif variant == 64:
        destination = reader.read_option(
            lambda: destack._generated.mir.tree.value.decode_value(reader)
        )
        callee = destack._generated.mir.tree.value.decode_value(reader)
        call = destack._generated.mir.tree.call.decode_call(reader)

        return InstructionCallIndirect(
            destination=destination,
            callee=callee,
            call=call,
        )
    elif variant == 65:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        layout = destack._generated.mir.tree.node.decode_local_node_id(reader)
        result_type = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return InstructionNewZeroed(
            destination=destination,
            layout=layout,
            result_type=result_type,
        )
    elif variant == 66:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        layout = destack._generated.mir.tree.node.decode_local_node_id(reader)
        result_type = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return InstructionNewUninit(
            destination=destination,
            layout=layout,
            result_type=result_type,
        )
    elif variant == 67:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        value_ = destack._generated.mir.tree.value.decode_value(reader)
        result_type = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return InstructionNewComplete(
            destination=destination,
            value=value_,
            result_type=result_type,
        )
    elif variant == 68:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        element = destack._generated.mir.tree.node.decode_local_node_id(reader)
        length = destack._generated.mir.tree.value.decode_value(reader)
        result_type = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return InstructionNewSliceZeroed(
            destination=destination,
            element=element,
            length=length,
            result_type=result_type,
        )
    elif variant == 69:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        element = destack._generated.mir.tree.node.decode_local_node_id(reader)
        length = destack._generated.mir.tree.value.decode_value(reader)
        result_type = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return InstructionNewSliceUninit(
            destination=destination,
            element=element,
            length=length,
            result_type=result_type,
        )
    elif variant == 70:
        value_ = destack._generated.mir.tree.value.decode_value(reader)

        return InstructionFree(
            value=value_,
        )
    elif variant == 71:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        layout = destack._generated.mir.tree.node.decode_local_node_id(reader)
        result_type = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return InstructionFrameAllocZeroed(
            destination=destination,
            layout=layout,
            result_type=result_type,
        )
    elif variant == 72:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        layout = destack._generated.mir.tree.node.decode_local_node_id(reader)
        result_type = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return InstructionFrameAllocUninit(
            destination=destination,
            layout=layout,
            result_type=result_type,
        )
    elif variant == 73:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        value_ = destack._generated.mir.tree.value.decode_value(reader)
        result_type = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return InstructionPin(
            destination=destination,
            value=value_,
            result_type=result_type,
        )
    elif variant == 74:
        value_ = destack._generated.mir.tree.value.decode_value(reader)

        return InstructionUnpin(
            value=value_,
        )
    elif variant == 75:
        object = destack._generated.mir.tree.value.decode_value(reader)
        offset = destack._generated.mir.tree.value.decode_value(reader)
        byte_len = destack._generated.mir.tree.value.decode_value(reader)

        return InstructionBarrierWrite(
            object=object,
            offset=offset,
            byte_len=byte_len,
        )
    elif variant == 76:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        pointer = destack._generated.mir.tree.value.decode_value(reader)
        result_type = destack._generated.mir.tree.node.decode_local_node_id(reader)
        access = destack._generated.mir.tree.memory.decode_atomic_access(reader)

        return InstructionAtomicLoad(
            destination=destination,
            pointer=pointer,
            result_type=result_type,
            access=access,
        )
    elif variant == 77:
        pointer = destack._generated.mir.tree.value.decode_value(reader)
        value_ = destack._generated.mir.tree.value.decode_value(reader)
        access = destack._generated.mir.tree.memory.decode_atomic_access(reader)

        return InstructionAtomicStore(
            pointer=pointer,
            value=value_,
            access=access,
        )
    elif variant == 78:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        pointer = destack._generated.mir.tree.value.decode_value(reader)
        expected = destack._generated.mir.tree.value.decode_value(reader)
        new_value = destack._generated.mir.tree.value.decode_value(reader)
        is_weak = reader.read_bool()
        access = destack._generated.mir.tree.memory.decode_compare_exchange_access(
            reader
        )

        return InstructionAtomicCompareExchange(
            destination=destination,
            pointer=pointer,
            expected=expected,
            new_value=new_value,
            is_weak=is_weak,
            access=access,
        )
    elif variant == 79:
        destination = destack._generated.mir.tree.value.decode_value(reader)
        operator = destack._generated.mir.tree.memory.decode_atomic_rmw_operator(reader)
        pointer = destack._generated.mir.tree.value.decode_value(reader)
        value_ = destack._generated.mir.tree.value.decode_value(reader)
        access = destack._generated.mir.tree.memory.decode_atomic_access(reader)

        return InstructionAtomicRmw(
            destination=destination,
            operator=operator,
            pointer=pointer,
            value=value_,
            access=access,
        )
    elif variant == 80:
        access = destack._generated.mir.tree.memory.decode_fence_access(reader)

        return InstructionAtomicFence(
            access=access,
        )
    elif variant == 81:
        condition = destack._generated.mir.tree.value.decode_value(reader)

        return InstructionAssume(
            condition=condition,
        )
    elif variant == 82:
        counter = destack._generated.mir.metadata.profile.decode_counter_id(reader)

        return InstructionProfileIncrement(
            counter=counter,
        )
    elif variant == 83:
        counter = destack._generated.mir.metadata.profile.decode_counter_id(reader)
        value_ = destack._generated.mir.tree.value.decode_value(reader)

        return InstructionProfileValue(
            counter=counter,
            value=value_,
        )
    elif variant == 84:
        destination = reader.read_option(
            lambda: destack._generated.mir.tree.value.decode_value(reader)
        )
        intrinsic = destack._generated.mir.tree.intrinsic.decode_intrinsic(reader)
        arguments = destack._generated.mir.tree.value.decode_value_slice(reader)

        return InstructionIntrinsic(
            destination=destination,
            intrinsic=intrinsic,
            arguments=arguments,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_instruction(value: Instruction) -> Json:
    """Return one JSON value for one Instruction."""
    if value.kind == "error":
        return {
            "kind": "error",
        }
    elif value.kind == "const":
        return {
            "kind": "const",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "value": destack._generated.mir.tree.constant.to_json_constant(value.value),
        }
    elif value.kind == "binary":
        return {
            "kind": "binary",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "operator": destack._generated.mir.tree.operator.to_json_binary_operator(
                value.operator
            ),
            "left": destack._generated.mir.tree.value.to_json_value(value.left),
            "right": destack._generated.mir.tree.value.to_json_value(value.right),
        }
    elif value.kind == "unary":
        return {
            "kind": "unary",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "operator": destack._generated.mir.tree.operator.to_json_unary_operator(
                value.operator
            ),
            "argument": destack._generated.mir.tree.value.to_json_value(value.argument),
        }
    elif value.kind == "cast":
        return {
            "kind": "cast",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "operator": to_json_cast_operator(value.operator),
            "argument": destack._generated.mir.tree.value.to_json_value(value.argument),
            "toType": destack._generated.mir.tree.node.to_json_local_node_id(
                value.to_type
            ),
        }
    elif value.kind == "select":
        return {
            "kind": "select",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "condition": destack._generated.mir.tree.value.to_json_value(
                value.condition
            ),
            "thenValue": destack._generated.mir.tree.value.to_json_value(
                value.then_value
            ),
            "elseValue": destack._generated.mir.tree.value.to_json_value(
                value.else_value
            ),
        }
    elif value.kind == "localGet":
        return {
            "kind": "localGet",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "local": destack._generated.mir.tree.node.to_json_local_node_id(
                value.local
            ),
        }
    elif value.kind == "localAddr":
        return {
            "kind": "localAddr",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "local": destack._generated.mir.tree.node.to_json_local_node_id(
                value.local
            ),
            "resultType": destack._generated.mir.tree.node.to_json_local_node_id(
                value.result_type
            ),
        }
    elif value.kind == "localSet":
        return {
            "kind": "localSet",
            "local": destack._generated.mir.tree.node.to_json_local_node_id(
                value.local
            ),
            "value": destack._generated.mir.tree.value.to_json_value(value.value),
        }
    elif value.kind == "globalAddr":
        return {
            "kind": "globalAddr",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "global": destack._generated.mir.tree.node.to_json_local_node_id(
                value.global_
            ),
            "resultType": destack._generated.mir.tree.node.to_json_local_node_id(
                value.result_type
            ),
        }
    elif value.kind == "functionAddr":
        return {
            "kind": "functionAddr",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "function": destack._generated.mir.tree.node.to_json_local_node_id(
                value.function
            ),
        }
    elif value.kind == "functionBind":
        return {
            "kind": "functionBind",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "function": destack._generated.mir.tree.node.to_json_local_node_id(
                value.function
            ),
            "environment": destack._generated.mir.tree.value.to_json_value(
                value.environment
            ),
        }
    elif value.kind == "functionPointer":
        return {
            "kind": "functionPointer",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "function": destack._generated.mir.tree.value.to_json_value(value.function),
        }
    elif value.kind == "functionEnvironment":
        return {
            "kind": "functionEnvironment",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "function": destack._generated.mir.tree.value.to_json_value(value.function),
        }
    elif value.kind == "functionEnvironmentCurrent":
        return {
            "kind": "functionEnvironmentCurrent",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
        }
    elif value.kind == "load":
        return {
            "kind": "load",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "pointer": destack._generated.mir.tree.value.to_json_value(value.pointer),
            "resultType": destack._generated.mir.tree.node.to_json_local_node_id(
                value.result_type
            ),
        }
    elif value.kind == "store":
        return {
            "kind": "store",
            "pointer": destack._generated.mir.tree.value.to_json_value(value.pointer),
            "value": destack._generated.mir.tree.value.to_json_value(value.value),
        }
    elif value.kind == "struct":
        return {
            "kind": "struct",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "ty": destack._generated.mir.tree.node.to_json_local_node_id(value.ty),
            "fields": destack._generated.mir.tree.value.to_json_value_slice(
                value.fields
            ),
        }
    elif value.kind == "tuple":
        return {
            "kind": "tuple",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "ty": destack._generated.mir.tree.node.to_json_local_node_id(value.ty),
            "elements": destack._generated.mir.tree.value.to_json_value_slice(
                value.elements
            ),
        }
    elif value.kind == "array":
        return {
            "kind": "array",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "ty": destack._generated.mir.tree.node.to_json_local_node_id(value.ty),
            "elements": destack._generated.mir.tree.value.to_json_value_slice(
                value.elements
            ),
        }
    elif value.kind == "fieldGet":
        return {
            "kind": "fieldGet",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "aggregate": destack._generated.mir.tree.value.to_json_value(
                value.aggregate
            ),
            "index": value.index,
        }
    elif value.kind == "fieldSet":
        return {
            "kind": "fieldSet",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "aggregate": destack._generated.mir.tree.value.to_json_value(
                value.aggregate
            ),
            "index": value.index,
            "value": destack._generated.mir.tree.value.to_json_value(value.value),
        }
    elif value.kind == "fieldAddr":
        return {
            "kind": "fieldAddr",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "aggregate": destack._generated.mir.tree.value.to_json_value(
                value.aggregate
            ),
            "index": value.index,
            "resultType": destack._generated.mir.tree.node.to_json_local_node_id(
                value.result_type
            ),
        }
    elif value.kind == "elementAddr":
        return {
            "kind": "elementAddr",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "array": destack._generated.mir.tree.value.to_json_value(value.array),
            "index": destack._generated.mir.tree.value.to_json_value(value.index),
            "resultType": destack._generated.mir.tree.node.to_json_local_node_id(
                value.result_type
            ),
        }
    elif value.kind == "sliceView":
        return {
            "kind": "sliceView",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "source": destack._generated.mir.tree.value.to_json_value(value.source),
            "start": destack._generated.mir.tree.value.to_json_value(value.start),
            "length": destack._generated.mir.tree.value.to_json_value(value.length),
            "resultType": destack._generated.mir.tree.node.to_json_local_node_id(
                value.result_type
            ),
        }
    elif value.kind == "sliceLength":
        return {
            "kind": "sliceLength",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "slice": destack._generated.mir.tree.value.to_json_value(value.slice),
        }
    elif value.kind == "dynamicPayload":
        return {
            "kind": "dynamicPayload",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "dynamic": destack._generated.mir.tree.value.to_json_value(value.dynamic),
            "resultType": destack._generated.mir.tree.node.to_json_local_node_id(
                value.result_type
            ),
        }
    elif value.kind == "dynamicType":
        return {
            "kind": "dynamicType",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "dynamic": destack._generated.mir.tree.value.to_json_value(value.dynamic),
        }
    elif value.kind == "variantTag":
        return {
            "kind": "variantTag",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "variant": destack._generated.mir.tree.value.to_json_value(value.variant),
        }
    elif value.kind == "variantPayload":
        return {
            "kind": "variantPayload",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "variant": destack._generated.mir.tree.value.to_json_value(value.variant),
            "tag": destack._generated.mir.tree.constant.to_json_constant(value.tag),
        }
    elif value.kind == "vectorSplat":
        return {
            "kind": "vectorSplat",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "value": destack._generated.mir.tree.value.to_json_value(value.value),
        }
    elif value.kind == "vectorExtract":
        return {
            "kind": "vectorExtract",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "vector": destack._generated.mir.tree.value.to_json_value(value.vector),
            "index": destack._generated.mir.tree.value.to_json_value(value.index),
        }
    elif value.kind == "vectorInsert":
        return {
            "kind": "vectorInsert",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "vector": destack._generated.mir.tree.value.to_json_value(value.vector),
            "index": destack._generated.mir.tree.value.to_json_value(value.index),
            "value": destack._generated.mir.tree.value.to_json_value(value.value),
        }
    elif value.kind == "vectorShuffle":
        return {
            "kind": "vectorShuffle",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "left": destack._generated.mir.tree.value.to_json_value(value.left),
            "right": destack._generated.mir.tree.value.to_json_value(value.right),
            "mask": destack._generated.mir.tree.immediate.to_json_index_slice(
                value.mask
            ),
        }
    elif value.kind == "vectorSelect":
        return {
            "kind": "vectorSelect",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "mask": destack._generated.mir.tree.value.to_json_value(value.mask),
            "thenValue": destack._generated.mir.tree.value.to_json_value(
                value.then_value
            ),
            "elseValue": destack._generated.mir.tree.value.to_json_value(
                value.else_value
            ),
        }
    elif value.kind == "vectorReduce":
        return {
            "kind": "vectorReduce",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "operator": destack._generated.mir.tree.vector.to_json_vector_reduce_operator(
                value.operator
            ),
            "vector": destack._generated.mir.tree.value.to_json_value(value.vector),
        }
    elif value.kind == "vectorCompare":
        return {
            "kind": "vectorCompare",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "operator": destack._generated.mir.tree.operator.to_json_binary_operator(
                value.operator
            ),
            "left": destack._generated.mir.tree.value.to_json_value(value.left),
            "right": destack._generated.mir.tree.value.to_json_value(value.right),
        }
    elif value.kind == "vectorConvert":
        return {
            "kind": "vectorConvert",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "mode": destack._generated.mir.tree.vector.to_json_vector_convert_mode(
                value.mode
            ),
            "vector": destack._generated.mir.tree.value.to_json_value(value.vector),
        }
    elif value.kind == "tensorSplat":
        return {
            "kind": "tensorSplat",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "value": destack._generated.mir.tree.value.to_json_value(value.value),
        }
    elif value.kind == "tensorLoad":
        return {
            "kind": "tensorLoad",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "view": destack._generated.mir.tree.value.to_json_value(value.view),
            "indices": destack._generated.mir.tree.value.to_json_value_slice(
                value.indices
            ),
        }
    elif value.kind == "tensorExtract":
        return {
            "kind": "tensorExtract",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "tensor": destack._generated.mir.tree.value.to_json_value(value.tensor),
            "indices": destack._generated.mir.tree.value.to_json_value_slice(
                value.indices
            ),
        }
    elif value.kind == "tensorStore":
        return {
            "kind": "tensorStore",
            "view": destack._generated.mir.tree.value.to_json_value(value.view),
            "indices": destack._generated.mir.tree.value.to_json_value_slice(
                value.indices
            ),
            "value": destack._generated.mir.tree.value.to_json_value(value.value),
        }
    elif value.kind == "tensorFill":
        return {
            "kind": "tensorFill",
            "view": destack._generated.mir.tree.value.to_json_value(value.view),
            "value": destack._generated.mir.tree.value.to_json_value(value.value),
        }
    elif value.kind == "tensorCopy":
        return {
            "kind": "tensorCopy",
            "target": destack._generated.mir.tree.value.to_json_value(value.target),
            "source": destack._generated.mir.tree.value.to_json_value(value.source),
        }
    elif value.kind == "tensorReshape":
        return {
            "kind": "tensorReshape",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "tensor": destack._generated.mir.tree.value.to_json_value(value.tensor),
            "shape": destack._generated.mir.tree.value.to_json_value_slice(value.shape),
        }
    elif value.kind == "tensorBroadcast":
        return {
            "kind": "tensorBroadcast",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "tensor": destack._generated.mir.tree.value.to_json_value(value.tensor),
            "dimensions": destack._generated.mir.tree.immediate.to_json_index_slice(
                value.dimensions
            ),
        }
    elif value.kind == "tensorTranspose":
        return {
            "kind": "tensorTranspose",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "tensor": destack._generated.mir.tree.value.to_json_value(value.tensor),
            "permutation": destack._generated.mir.tree.immediate.to_json_index_slice(
                value.permutation
            ),
        }
    elif value.kind == "tensorCast":
        return {
            "kind": "tensorCast",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "tensor": destack._generated.mir.tree.value.to_json_value(value.tensor),
        }
    elif value.kind == "tensorView":
        return {
            "kind": "tensorView",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "view": destack._generated.mir.tree.value.to_json_value(value.view),
            "arguments": destack._generated.mir.tree.value.to_json_value_slice(
                value.arguments
            ),
            "offsetsCount": value.offsets_count,
            "sizesCount": value.sizes_count,
            "stridesCount": value.strides_count,
        }
    elif value.kind == "tensorSlice":
        return {
            "kind": "tensorSlice",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "tensor": destack._generated.mir.tree.value.to_json_value(value.tensor),
            "arguments": destack._generated.mir.tree.value.to_json_value_slice(
                value.arguments
            ),
            "offsetsCount": value.offsets_count,
            "sizesCount": value.sizes_count,
            "stridesCount": value.strides_count,
        }
    elif value.kind == "tensorPad":
        return {
            "kind": "tensorPad",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "tensor": destack._generated.mir.tree.value.to_json_value(value.tensor),
            "arguments": destack._generated.mir.tree.value.to_json_value_slice(
                value.arguments
            ),
            "lowCount": value.low_count,
            "highCount": value.high_count,
            "interiorCount": value.interior_count,
            "value": destack._generated.mir.tree.value.to_json_value(value.value),
        }
    elif value.kind == "tensorConcat":
        return {
            "kind": "tensorConcat",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "tensors": destack._generated.mir.tree.value.to_json_value_slice(
                value.tensors
            ),
            "axis": value.axis,
        }
    elif value.kind == "tensorCompare":
        return {
            "kind": "tensorCompare",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "operator": destack._generated.mir.tree.operator.to_json_binary_operator(
                value.operator
            ),
            "left": destack._generated.mir.tree.value.to_json_value(value.left),
            "right": destack._generated.mir.tree.value.to_json_value(value.right),
        }
    elif value.kind == "tensorSelect":
        return {
            "kind": "tensorSelect",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "mask": destack._generated.mir.tree.value.to_json_value(value.mask),
            "thenValue": destack._generated.mir.tree.value.to_json_value(
                value.then_value
            ),
            "elseValue": destack._generated.mir.tree.value.to_json_value(
                value.else_value
            ),
        }
    elif value.kind == "tensorReduce":
        return {
            "kind": "tensorReduce",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "operator": destack._generated.mir.tree.tensor.to_json_tensor_reduce_operator(
                value.operator
            ),
            "tensor": destack._generated.mir.tree.value.to_json_value(value.tensor),
            "initial": destack._generated.mir.tree.value.to_json_value(value.initial),
            "axes": destack._generated.mir.tree.immediate.to_json_index_slice(
                value.axes
            ),
        }
    elif value.kind == "tensorIndexReduce":
        return {
            "kind": "tensorIndexReduce",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "operator": destack._generated.mir.tree.tensor.to_json_tensor_index_reduce_operator(
                value.operator
            ),
            "tensor": destack._generated.mir.tree.value.to_json_value(value.tensor),
            "axis": value.axis,
            "tieBreak": destack._generated.mir.tree.tensor.to_json_tensor_index_tie_break(
                value.tie_break
            ),
        }
    elif value.kind == "tensorDot":
        return {
            "kind": "tensorDot",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "left": destack._generated.mir.tree.value.to_json_value(value.left),
            "right": destack._generated.mir.tree.value.to_json_value(value.right),
            "immediate": destack._generated.mir.tree.immediate.to_json_tensor_immediate_id(
                value.immediate
            ),
        }
    elif value.kind == "tensorConvolution":
        return {
            "kind": "tensorConvolution",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "input": destack._generated.mir.tree.value.to_json_value(value.input),
            "kernel": destack._generated.mir.tree.value.to_json_value(value.kernel),
            "immediate": destack._generated.mir.tree.immediate.to_json_tensor_immediate_id(
                value.immediate
            ),
        }
    elif value.kind == "tensorGather":
        return {
            "kind": "tensorGather",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "operand": destack._generated.mir.tree.value.to_json_value(value.operand),
            "indices": destack._generated.mir.tree.value.to_json_value(value.indices),
            "immediate": destack._generated.mir.tree.immediate.to_json_tensor_immediate_id(
                value.immediate
            ),
        }
    elif value.kind == "tensorScatter":
        return {
            "kind": "tensorScatter",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "operand": destack._generated.mir.tree.value.to_json_value(value.operand),
            "indices": destack._generated.mir.tree.value.to_json_value(value.indices),
            "updates": destack._generated.mir.tree.value.to_json_value(value.updates),
            "immediate": destack._generated.mir.tree.immediate.to_json_tensor_immediate_id(
                value.immediate
            ),
            "mode": destack._generated.mir.tree.tensor.to_json_tensor_scatter_mode(
                value.mode
            ),
        }
    elif value.kind == "tensorConvert":
        return {
            "kind": "tensorConvert",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "mode": destack._generated.mir.tree.tensor.to_json_tensor_convert_mode(
                value.mode
            ),
            "tensor": destack._generated.mir.tree.value.to_json_value(value.tensor),
        }
    elif value.kind == "call":
        return {
            "kind": "call",
            **(
                {}
                if value.destination is None
                else {
                    "destination": destack._generated.mir.tree.value.to_json_value(
                        value.destination
                    )
                }
            ),
            "function": destack._generated.mir.tree.node.to_json_local_node_id(
                value.function
            ),
            "call": destack._generated.mir.tree.call.to_json_call(value.call),
        }
    elif value.kind == "callVirtual":
        return {
            "kind": "callVirtual",
            **(
                {}
                if value.destination is None
                else {
                    "destination": destack._generated.mir.tree.value.to_json_value(
                        value.destination
                    )
                }
            ),
            "receiver": destack._generated.mir.tree.value.to_json_value(value.receiver),
            "class": destack._generated.mir.tree.node.to_json_local_node_id(
                value.class_
            ),
            "slot": destack._generated.mir.metadata.dispatch.to_json_dispatch_slot(
                value.slot
            ),
            "call": destack._generated.mir.tree.call.to_json_call(value.call),
        }
    elif value.kind == "callDynamic":
        return {
            "kind": "callDynamic",
            **(
                {}
                if value.destination is None
                else {
                    "destination": destack._generated.mir.tree.value.to_json_value(
                        value.destination
                    )
                }
            ),
            "receiver": destack._generated.mir.tree.value.to_json_value(value.receiver),
            "constraint": destack._generated.mir.tree.node.to_json_local_node_id(
                value.constraint
            ),
            "slot": destack._generated.mir.metadata.dispatch.to_json_dispatch_slot(
                value.slot
            ),
            "call": destack._generated.mir.tree.call.to_json_call(value.call),
        }
    elif value.kind == "callIndirect":
        return {
            "kind": "callIndirect",
            **(
                {}
                if value.destination is None
                else {
                    "destination": destack._generated.mir.tree.value.to_json_value(
                        value.destination
                    )
                }
            ),
            "callee": destack._generated.mir.tree.value.to_json_value(value.callee),
            "call": destack._generated.mir.tree.call.to_json_call(value.call),
        }
    elif value.kind == "newZeroed":
        return {
            "kind": "newZeroed",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "layout": destack._generated.mir.tree.node.to_json_local_node_id(
                value.layout
            ),
            "resultType": destack._generated.mir.tree.node.to_json_local_node_id(
                value.result_type
            ),
        }
    elif value.kind == "newUninit":
        return {
            "kind": "newUninit",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "layout": destack._generated.mir.tree.node.to_json_local_node_id(
                value.layout
            ),
            "resultType": destack._generated.mir.tree.node.to_json_local_node_id(
                value.result_type
            ),
        }
    elif value.kind == "newComplete":
        return {
            "kind": "newComplete",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "value": destack._generated.mir.tree.value.to_json_value(value.value),
            "resultType": destack._generated.mir.tree.node.to_json_local_node_id(
                value.result_type
            ),
        }
    elif value.kind == "newSliceZeroed":
        return {
            "kind": "newSliceZeroed",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "element": destack._generated.mir.tree.node.to_json_local_node_id(
                value.element
            ),
            "length": destack._generated.mir.tree.value.to_json_value(value.length),
            "resultType": destack._generated.mir.tree.node.to_json_local_node_id(
                value.result_type
            ),
        }
    elif value.kind == "newSliceUninit":
        return {
            "kind": "newSliceUninit",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "element": destack._generated.mir.tree.node.to_json_local_node_id(
                value.element
            ),
            "length": destack._generated.mir.tree.value.to_json_value(value.length),
            "resultType": destack._generated.mir.tree.node.to_json_local_node_id(
                value.result_type
            ),
        }
    elif value.kind == "free":
        return {
            "kind": "free",
            "value": destack._generated.mir.tree.value.to_json_value(value.value),
        }
    elif value.kind == "frameAllocZeroed":
        return {
            "kind": "frameAllocZeroed",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "layout": destack._generated.mir.tree.node.to_json_local_node_id(
                value.layout
            ),
            "resultType": destack._generated.mir.tree.node.to_json_local_node_id(
                value.result_type
            ),
        }
    elif value.kind == "frameAllocUninit":
        return {
            "kind": "frameAllocUninit",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "layout": destack._generated.mir.tree.node.to_json_local_node_id(
                value.layout
            ),
            "resultType": destack._generated.mir.tree.node.to_json_local_node_id(
                value.result_type
            ),
        }
    elif value.kind == "pin":
        return {
            "kind": "pin",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "value": destack._generated.mir.tree.value.to_json_value(value.value),
            "resultType": destack._generated.mir.tree.node.to_json_local_node_id(
                value.result_type
            ),
        }
    elif value.kind == "unpin":
        return {
            "kind": "unpin",
            "value": destack._generated.mir.tree.value.to_json_value(value.value),
        }
    elif value.kind == "barrierWrite":
        return {
            "kind": "barrierWrite",
            "object": destack._generated.mir.tree.value.to_json_value(value.object),
            "offset": destack._generated.mir.tree.value.to_json_value(value.offset),
            "byteLen": destack._generated.mir.tree.value.to_json_value(value.byte_len),
        }
    elif value.kind == "atomicLoad":
        return {
            "kind": "atomicLoad",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "pointer": destack._generated.mir.tree.value.to_json_value(value.pointer),
            "resultType": destack._generated.mir.tree.node.to_json_local_node_id(
                value.result_type
            ),
            "access": destack._generated.mir.tree.memory.to_json_atomic_access(
                value.access
            ),
        }
    elif value.kind == "atomicStore":
        return {
            "kind": "atomicStore",
            "pointer": destack._generated.mir.tree.value.to_json_value(value.pointer),
            "value": destack._generated.mir.tree.value.to_json_value(value.value),
            "access": destack._generated.mir.tree.memory.to_json_atomic_access(
                value.access
            ),
        }
    elif value.kind == "atomicCompareExchange":
        return {
            "kind": "atomicCompareExchange",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "pointer": destack._generated.mir.tree.value.to_json_value(value.pointer),
            "expected": destack._generated.mir.tree.value.to_json_value(value.expected),
            "newValue": destack._generated.mir.tree.value.to_json_value(
                value.new_value
            ),
            "isWeak": value.is_weak,
            "access": destack._generated.mir.tree.memory.to_json_compare_exchange_access(
                value.access
            ),
        }
    elif value.kind == "atomicRmw":
        return {
            "kind": "atomicRmw",
            "destination": destack._generated.mir.tree.value.to_json_value(
                value.destination
            ),
            "operator": destack._generated.mir.tree.memory.to_json_atomic_rmw_operator(
                value.operator
            ),
            "pointer": destack._generated.mir.tree.value.to_json_value(value.pointer),
            "value": destack._generated.mir.tree.value.to_json_value(value.value),
            "access": destack._generated.mir.tree.memory.to_json_atomic_access(
                value.access
            ),
        }
    elif value.kind == "atomicFence":
        return {
            "kind": "atomicFence",
            "access": destack._generated.mir.tree.memory.to_json_fence_access(
                value.access
            ),
        }
    elif value.kind == "assume":
        return {
            "kind": "assume",
            "condition": destack._generated.mir.tree.value.to_json_value(
                value.condition
            ),
        }
    elif value.kind == "profileIncrement":
        return {
            "kind": "profileIncrement",
            "counter": destack._generated.mir.metadata.profile.to_json_counter_id(
                value.counter
            ),
        }
    elif value.kind == "profileValue":
        return {
            "kind": "profileValue",
            "counter": destack._generated.mir.metadata.profile.to_json_counter_id(
                value.counter
            ),
            "value": destack._generated.mir.tree.value.to_json_value(value.value),
        }
    elif value.kind == "intrinsic":
        return {
            "kind": "intrinsic",
            **(
                {}
                if value.destination is None
                else {
                    "destination": destack._generated.mir.tree.value.to_json_value(
                        value.destination
                    )
                }
            ),
            "intrinsic": destack._generated.mir.tree.intrinsic.to_json_intrinsic(
                value.intrinsic
            ),
            "arguments": destack._generated.mir.tree.value.to_json_value_slice(
                value.arguments
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_instruction(value: Json) -> Instruction:
    """Return one Instruction from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "error":
        return InstructionError()
    elif kind == "const":
        return InstructionConst(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            value=destack._generated.mir.tree.constant.from_json_constant(
                json_field(object_, "value")
            ),
        )
    elif kind == "binary":
        return InstructionBinary(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            operator=destack._generated.mir.tree.operator.from_json_binary_operator(
                json_field(object_, "operator")
            ),
            left=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "left")
            ),
            right=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "right")
            ),
        )
    elif kind == "unary":
        return InstructionUnary(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            operator=destack._generated.mir.tree.operator.from_json_unary_operator(
                json_field(object_, "operator")
            ),
            argument=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "argument")
            ),
        )
    elif kind == "cast":
        return InstructionCast(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            operator=from_json_cast_operator(json_field(object_, "operator")),
            argument=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "argument")
            ),
            to_type=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "toType")
            ),
        )
    elif kind == "select":
        return InstructionSelect(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            condition=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "condition")
            ),
            then_value=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "thenValue")
            ),
            else_value=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "elseValue")
            ),
        )
    elif kind == "localGet":
        return InstructionLocalGet(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            local=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "local")
            ),
        )
    elif kind == "localAddr":
        return InstructionLocalAddr(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            local=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "local")
            ),
            result_type=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "resultType")
            ),
        )
    elif kind == "localSet":
        return InstructionLocalSet(
            local=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "local")
            ),
            value=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "value")
            ),
        )
    elif kind == "globalAddr":
        return InstructionGlobalAddr(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            global_=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "global")
            ),
            result_type=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "resultType")
            ),
        )
    elif kind == "functionAddr":
        return InstructionFunctionAddr(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            function=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "function")
            ),
        )
    elif kind == "functionBind":
        return InstructionFunctionBind(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            function=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "function")
            ),
            environment=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "environment")
            ),
        )
    elif kind == "functionPointer":
        return InstructionFunctionPointer(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            function=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "function")
            ),
        )
    elif kind == "functionEnvironment":
        return InstructionFunctionEnvironment(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            function=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "function")
            ),
        )
    elif kind == "functionEnvironmentCurrent":
        return InstructionFunctionEnvironmentCurrent(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
        )
    elif kind == "load":
        return InstructionLoad(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            pointer=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "pointer")
            ),
            result_type=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "resultType")
            ),
        )
    elif kind == "store":
        return InstructionStore(
            pointer=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "pointer")
            ),
            value=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "value")
            ),
        )
    elif kind == "struct":
        return InstructionStruct(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            ty=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "ty")
            ),
            fields=destack._generated.mir.tree.value.from_json_value_slice(
                json_field(object_, "fields")
            ),
        )
    elif kind == "tuple":
        return InstructionTuple(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            ty=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "ty")
            ),
            elements=destack._generated.mir.tree.value.from_json_value_slice(
                json_field(object_, "elements")
            ),
        )
    elif kind == "array":
        return InstructionArray(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            ty=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "ty")
            ),
            elements=destack._generated.mir.tree.value.from_json_value_slice(
                json_field(object_, "elements")
            ),
        )
    elif kind == "fieldGet":
        return InstructionFieldGet(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            aggregate=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "aggregate")
            ),
            index=json_int(json_field(object_, "index")),
        )
    elif kind == "fieldSet":
        return InstructionFieldSet(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            aggregate=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "aggregate")
            ),
            index=json_int(json_field(object_, "index")),
            value=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "value")
            ),
        )
    elif kind == "fieldAddr":
        return InstructionFieldAddr(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            aggregate=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "aggregate")
            ),
            index=json_int(json_field(object_, "index")),
            result_type=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "resultType")
            ),
        )
    elif kind == "elementAddr":
        return InstructionElementAddr(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            array=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "array")
            ),
            index=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "index")
            ),
            result_type=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "resultType")
            ),
        )
    elif kind == "sliceView":
        return InstructionSliceView(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            source=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "source")
            ),
            start=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "start")
            ),
            length=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "length")
            ),
            result_type=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "resultType")
            ),
        )
    elif kind == "sliceLength":
        return InstructionSliceLength(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            slice=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "slice")
            ),
        )
    elif kind == "dynamicPayload":
        return InstructionDynamicPayload(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            dynamic=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "dynamic")
            ),
            result_type=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "resultType")
            ),
        )
    elif kind == "dynamicType":
        return InstructionDynamicType(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            dynamic=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "dynamic")
            ),
        )
    elif kind == "variantTag":
        return InstructionVariantTag(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            variant=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "variant")
            ),
        )
    elif kind == "variantPayload":
        return InstructionVariantPayload(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            variant=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "variant")
            ),
            tag=destack._generated.mir.tree.constant.from_json_constant(
                json_field(object_, "tag")
            ),
        )
    elif kind == "vectorSplat":
        return InstructionVectorSplat(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            value=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "value")
            ),
        )
    elif kind == "vectorExtract":
        return InstructionVectorExtract(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            vector=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "vector")
            ),
            index=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "index")
            ),
        )
    elif kind == "vectorInsert":
        return InstructionVectorInsert(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            vector=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "vector")
            ),
            index=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "index")
            ),
            value=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "value")
            ),
        )
    elif kind == "vectorShuffle":
        return InstructionVectorShuffle(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            left=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "left")
            ),
            right=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "right")
            ),
            mask=destack._generated.mir.tree.immediate.from_json_index_slice(
                json_field(object_, "mask")
            ),
        )
    elif kind == "vectorSelect":
        return InstructionVectorSelect(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            mask=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "mask")
            ),
            then_value=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "thenValue")
            ),
            else_value=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "elseValue")
            ),
        )
    elif kind == "vectorReduce":
        return InstructionVectorReduce(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            operator=destack._generated.mir.tree.vector.from_json_vector_reduce_operator(
                json_field(object_, "operator")
            ),
            vector=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "vector")
            ),
        )
    elif kind == "vectorCompare":
        return InstructionVectorCompare(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            operator=destack._generated.mir.tree.operator.from_json_binary_operator(
                json_field(object_, "operator")
            ),
            left=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "left")
            ),
            right=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "right")
            ),
        )
    elif kind == "vectorConvert":
        return InstructionVectorConvert(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            mode=destack._generated.mir.tree.vector.from_json_vector_convert_mode(
                json_field(object_, "mode")
            ),
            vector=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "vector")
            ),
        )
    elif kind == "tensorSplat":
        return InstructionTensorSplat(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            value=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "value")
            ),
        )
    elif kind == "tensorLoad":
        return InstructionTensorLoad(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            view=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "view")
            ),
            indices=destack._generated.mir.tree.value.from_json_value_slice(
                json_field(object_, "indices")
            ),
        )
    elif kind == "tensorExtract":
        return InstructionTensorExtract(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            tensor=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "tensor")
            ),
            indices=destack._generated.mir.tree.value.from_json_value_slice(
                json_field(object_, "indices")
            ),
        )
    elif kind == "tensorStore":
        return InstructionTensorStore(
            view=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "view")
            ),
            indices=destack._generated.mir.tree.value.from_json_value_slice(
                json_field(object_, "indices")
            ),
            value=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "value")
            ),
        )
    elif kind == "tensorFill":
        return InstructionTensorFill(
            view=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "view")
            ),
            value=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "value")
            ),
        )
    elif kind == "tensorCopy":
        return InstructionTensorCopy(
            target=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "target")
            ),
            source=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "source")
            ),
        )
    elif kind == "tensorReshape":
        return InstructionTensorReshape(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            tensor=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "tensor")
            ),
            shape=destack._generated.mir.tree.value.from_json_value_slice(
                json_field(object_, "shape")
            ),
        )
    elif kind == "tensorBroadcast":
        return InstructionTensorBroadcast(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            tensor=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "tensor")
            ),
            dimensions=destack._generated.mir.tree.immediate.from_json_index_slice(
                json_field(object_, "dimensions")
            ),
        )
    elif kind == "tensorTranspose":
        return InstructionTensorTranspose(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            tensor=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "tensor")
            ),
            permutation=destack._generated.mir.tree.immediate.from_json_index_slice(
                json_field(object_, "permutation")
            ),
        )
    elif kind == "tensorCast":
        return InstructionTensorCast(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            tensor=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "tensor")
            ),
        )
    elif kind == "tensorView":
        return InstructionTensorView(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            view=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "view")
            ),
            arguments=destack._generated.mir.tree.value.from_json_value_slice(
                json_field(object_, "arguments")
            ),
            offsets_count=json_int(json_field(object_, "offsetsCount")),
            sizes_count=json_int(json_field(object_, "sizesCount")),
            strides_count=json_int(json_field(object_, "stridesCount")),
        )
    elif kind == "tensorSlice":
        return InstructionTensorSlice(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            tensor=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "tensor")
            ),
            arguments=destack._generated.mir.tree.value.from_json_value_slice(
                json_field(object_, "arguments")
            ),
            offsets_count=json_int(json_field(object_, "offsetsCount")),
            sizes_count=json_int(json_field(object_, "sizesCount")),
            strides_count=json_int(json_field(object_, "stridesCount")),
        )
    elif kind == "tensorPad":
        return InstructionTensorPad(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            tensor=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "tensor")
            ),
            arguments=destack._generated.mir.tree.value.from_json_value_slice(
                json_field(object_, "arguments")
            ),
            low_count=json_int(json_field(object_, "lowCount")),
            high_count=json_int(json_field(object_, "highCount")),
            interior_count=json_int(json_field(object_, "interiorCount")),
            value=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "value")
            ),
        )
    elif kind == "tensorConcat":
        return InstructionTensorConcat(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            tensors=destack._generated.mir.tree.value.from_json_value_slice(
                json_field(object_, "tensors")
            ),
            axis=json_int(json_field(object_, "axis")),
        )
    elif kind == "tensorCompare":
        return InstructionTensorCompare(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            operator=destack._generated.mir.tree.operator.from_json_binary_operator(
                json_field(object_, "operator")
            ),
            left=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "left")
            ),
            right=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "right")
            ),
        )
    elif kind == "tensorSelect":
        return InstructionTensorSelect(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            mask=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "mask")
            ),
            then_value=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "thenValue")
            ),
            else_value=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "elseValue")
            ),
        )
    elif kind == "tensorReduce":
        return InstructionTensorReduce(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            operator=destack._generated.mir.tree.tensor.from_json_tensor_reduce_operator(
                json_field(object_, "operator")
            ),
            tensor=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "tensor")
            ),
            initial=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "initial")
            ),
            axes=destack._generated.mir.tree.immediate.from_json_index_slice(
                json_field(object_, "axes")
            ),
        )
    elif kind == "tensorIndexReduce":
        return InstructionTensorIndexReduce(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            operator=destack._generated.mir.tree.tensor.from_json_tensor_index_reduce_operator(
                json_field(object_, "operator")
            ),
            tensor=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "tensor")
            ),
            axis=json_int(json_field(object_, "axis")),
            tie_break=destack._generated.mir.tree.tensor.from_json_tensor_index_tie_break(
                json_field(object_, "tieBreak")
            ),
        )
    elif kind == "tensorDot":
        return InstructionTensorDot(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            left=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "left")
            ),
            right=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "right")
            ),
            immediate=destack._generated.mir.tree.immediate.from_json_tensor_immediate_id(
                json_field(object_, "immediate")
            ),
        )
    elif kind == "tensorConvolution":
        return InstructionTensorConvolution(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            input=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "input")
            ),
            kernel=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "kernel")
            ),
            immediate=destack._generated.mir.tree.immediate.from_json_tensor_immediate_id(
                json_field(object_, "immediate")
            ),
        )
    elif kind == "tensorGather":
        return InstructionTensorGather(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            operand=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "operand")
            ),
            indices=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "indices")
            ),
            immediate=destack._generated.mir.tree.immediate.from_json_tensor_immediate_id(
                json_field(object_, "immediate")
            ),
        )
    elif kind == "tensorScatter":
        return InstructionTensorScatter(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            operand=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "operand")
            ),
            indices=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "indices")
            ),
            updates=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "updates")
            ),
            immediate=destack._generated.mir.tree.immediate.from_json_tensor_immediate_id(
                json_field(object_, "immediate")
            ),
            mode=destack._generated.mir.tree.tensor.from_json_tensor_scatter_mode(
                json_field(object_, "mode")
            ),
        )
    elif kind == "tensorConvert":
        return InstructionTensorConvert(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            mode=destack._generated.mir.tree.tensor.from_json_tensor_convert_mode(
                json_field(object_, "mode")
            ),
            tensor=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "tensor")
            ),
        )
    elif kind == "call":
        return InstructionCall(
            destination=json_optional(
                object_,
                "destination",
                lambda value: destack._generated.mir.tree.value.from_json_value(value),
            ),
            function=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "function")
            ),
            call=destack._generated.mir.tree.call.from_json_call(
                json_field(object_, "call")
            ),
        )
    elif kind == "callVirtual":
        return InstructionCallVirtual(
            destination=json_optional(
                object_,
                "destination",
                lambda value: destack._generated.mir.tree.value.from_json_value(value),
            ),
            receiver=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "receiver")
            ),
            class_=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "class")
            ),
            slot=destack._generated.mir.metadata.dispatch.from_json_dispatch_slot(
                json_field(object_, "slot")
            ),
            call=destack._generated.mir.tree.call.from_json_call(
                json_field(object_, "call")
            ),
        )
    elif kind == "callDynamic":
        return InstructionCallDynamic(
            destination=json_optional(
                object_,
                "destination",
                lambda value: destack._generated.mir.tree.value.from_json_value(value),
            ),
            receiver=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "receiver")
            ),
            constraint=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "constraint")
            ),
            slot=destack._generated.mir.metadata.dispatch.from_json_dispatch_slot(
                json_field(object_, "slot")
            ),
            call=destack._generated.mir.tree.call.from_json_call(
                json_field(object_, "call")
            ),
        )
    elif kind == "callIndirect":
        return InstructionCallIndirect(
            destination=json_optional(
                object_,
                "destination",
                lambda value: destack._generated.mir.tree.value.from_json_value(value),
            ),
            callee=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "callee")
            ),
            call=destack._generated.mir.tree.call.from_json_call(
                json_field(object_, "call")
            ),
        )
    elif kind == "newZeroed":
        return InstructionNewZeroed(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            layout=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "layout")
            ),
            result_type=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "resultType")
            ),
        )
    elif kind == "newUninit":
        return InstructionNewUninit(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            layout=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "layout")
            ),
            result_type=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "resultType")
            ),
        )
    elif kind == "newComplete":
        return InstructionNewComplete(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            value=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "value")
            ),
            result_type=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "resultType")
            ),
        )
    elif kind == "newSliceZeroed":
        return InstructionNewSliceZeroed(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            element=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "element")
            ),
            length=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "length")
            ),
            result_type=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "resultType")
            ),
        )
    elif kind == "newSliceUninit":
        return InstructionNewSliceUninit(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            element=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "element")
            ),
            length=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "length")
            ),
            result_type=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "resultType")
            ),
        )
    elif kind == "free":
        return InstructionFree(
            value=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "value")
            ),
        )
    elif kind == "frameAllocZeroed":
        return InstructionFrameAllocZeroed(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            layout=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "layout")
            ),
            result_type=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "resultType")
            ),
        )
    elif kind == "frameAllocUninit":
        return InstructionFrameAllocUninit(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            layout=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "layout")
            ),
            result_type=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "resultType")
            ),
        )
    elif kind == "pin":
        return InstructionPin(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            value=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "value")
            ),
            result_type=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "resultType")
            ),
        )
    elif kind == "unpin":
        return InstructionUnpin(
            value=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "value")
            ),
        )
    elif kind == "barrierWrite":
        return InstructionBarrierWrite(
            object=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "object")
            ),
            offset=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "offset")
            ),
            byte_len=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "byteLen")
            ),
        )
    elif kind == "atomicLoad":
        return InstructionAtomicLoad(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            pointer=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "pointer")
            ),
            result_type=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "resultType")
            ),
            access=destack._generated.mir.tree.memory.from_json_atomic_access(
                json_field(object_, "access")
            ),
        )
    elif kind == "atomicStore":
        return InstructionAtomicStore(
            pointer=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "pointer")
            ),
            value=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "value")
            ),
            access=destack._generated.mir.tree.memory.from_json_atomic_access(
                json_field(object_, "access")
            ),
        )
    elif kind == "atomicCompareExchange":
        return InstructionAtomicCompareExchange(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            pointer=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "pointer")
            ),
            expected=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "expected")
            ),
            new_value=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "newValue")
            ),
            is_weak=json_bool(json_field(object_, "isWeak")),
            access=destack._generated.mir.tree.memory.from_json_compare_exchange_access(
                json_field(object_, "access")
            ),
        )
    elif kind == "atomicRmw":
        return InstructionAtomicRmw(
            destination=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "destination")
            ),
            operator=destack._generated.mir.tree.memory.from_json_atomic_rmw_operator(
                json_field(object_, "operator")
            ),
            pointer=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "pointer")
            ),
            value=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "value")
            ),
            access=destack._generated.mir.tree.memory.from_json_atomic_access(
                json_field(object_, "access")
            ),
        )
    elif kind == "atomicFence":
        return InstructionAtomicFence(
            access=destack._generated.mir.tree.memory.from_json_fence_access(
                json_field(object_, "access")
            ),
        )
    elif kind == "assume":
        return InstructionAssume(
            condition=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "condition")
            ),
        )
    elif kind == "profileIncrement":
        return InstructionProfileIncrement(
            counter=destack._generated.mir.metadata.profile.from_json_counter_id(
                json_field(object_, "counter")
            ),
        )
    elif kind == "profileValue":
        return InstructionProfileValue(
            counter=destack._generated.mir.metadata.profile.from_json_counter_id(
                json_field(object_, "counter")
            ),
            value=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "value")
            ),
        )
    elif kind == "intrinsic":
        return InstructionIntrinsic(
            destination=json_optional(
                object_,
                "destination",
                lambda value: destack._generated.mir.tree.value.from_json_value(value),
            ),
            intrinsic=destack._generated.mir.tree.intrinsic.from_json_intrinsic(
                json_field(object_, "intrinsic")
            ),
            arguments=destack._generated.mir.tree.value.from_json_value_slice(
                json_field(object_, "arguments")
            ),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


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


def encode_cast_operator(writer: BinaryWriter, value: CastOperator) -> None:
    """Encode one CastOperator."""
    if value == "bitcast":
        writer.write_unsigned(0)
    elif value == "truncate":
        writer.write_unsigned(1)
    elif value == "zeroExtend":
        writer.write_unsigned(2)
    elif value == "signExtend":
        writer.write_unsigned(3)
    elif value == "floatToSignedInt":
        writer.write_unsigned(4)
    elif value == "floatToUnsignedInt":
        writer.write_unsigned(5)
    elif value == "floatToSignedIntSaturating":
        writer.write_unsigned(6)
    elif value == "floatToUnsignedIntSaturating":
        writer.write_unsigned(7)
    elif value == "signedIntToFloat":
        writer.write_unsigned(8)
    elif value == "unsignedIntToFloat":
        writer.write_unsigned(9)
    elif value == "floatTruncate":
        writer.write_unsigned(10)
    elif value == "floatExtend":
        writer.write_unsigned(11)
    elif value == "floatConvert":
        writer.write_unsigned(12)
    elif value == "pointerToInt":
        writer.write_unsigned(13)
    elif value == "intToPointer":
        writer.write_unsigned(14)
    else:
        raise SerdeError("unknown enum variant")


def decode_cast_operator(reader: BinaryReader) -> CastOperator:
    """Decode one CastOperator."""
    variant = reader.read_number()

    if variant == 0:
        return "bitcast"
    elif variant == 1:
        return "truncate"
    elif variant == 2:
        return "zeroExtend"
    elif variant == 3:
        return "signExtend"
    elif variant == 4:
        return "floatToSignedInt"
    elif variant == 5:
        return "floatToUnsignedInt"
    elif variant == 6:
        return "floatToSignedIntSaturating"
    elif variant == 7:
        return "floatToUnsignedIntSaturating"
    elif variant == 8:
        return "signedIntToFloat"
    elif variant == 9:
        return "unsignedIntToFloat"
    elif variant == 10:
        return "floatTruncate"
    elif variant == 11:
        return "floatExtend"
    elif variant == 12:
        return "floatConvert"
    elif variant == 13:
        return "pointerToInt"
    elif variant == 14:
        return "intToPointer"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_cast_operator(value: CastOperator) -> Json:
    """Return one JSON value for one CastOperator."""
    return value


def from_json_cast_operator(value: Json) -> CastOperator:
    """Return one CastOperator from one JSON value."""
    variant = json_string(value)

    if variant == "bitcast":
        return "bitcast"
    elif variant == "truncate":
        return "truncate"
    elif variant == "zeroExtend":
        return "zeroExtend"
    elif variant == "signExtend":
        return "signExtend"
    elif variant == "floatToSignedInt":
        return "floatToSignedInt"
    elif variant == "floatToUnsignedInt":
        return "floatToUnsignedInt"
    elif variant == "floatToSignedIntSaturating":
        return "floatToSignedIntSaturating"
    elif variant == "floatToUnsignedIntSaturating":
        return "floatToUnsignedIntSaturating"
    elif variant == "signedIntToFloat":
        return "signedIntToFloat"
    elif variant == "unsignedIntToFloat":
        return "unsignedIntToFloat"
    elif variant == "floatTruncate":
        return "floatTruncate"
    elif variant == "floatExtend":
        return "floatExtend"
    elif variant == "floatConvert":
        return "floatConvert"
    elif variant == "pointerToInt":
        return "pointerToInt"
    elif variant == "intToPointer":
        return "intToPointer"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


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
