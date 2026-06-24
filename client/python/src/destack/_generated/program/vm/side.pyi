# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.mir.metadata.frame
import destack._generated.mir.tree.intrinsic
import destack._generated.mir.tree.node
import destack._generated.mir.tree.tensor
import destack._generated.mir.tree.value
import destack._generated.mir.tree.vector
import destack._generated.program.vm.atomic
import destack._generated.program.vm.function
import destack._generated.program.vm.projection
import destack._generated.program.vm.range
import destack._generated.program.vm.table
import destack._generated.program.vm.tensor
import destack._generated.program.vm.value

@dataclass(frozen=True, slots=True)
class AggregateSelect:
    """Aggregate select operation."""

    # the destination frame offset
    destination_offset: int
    # the condition cell offset
    condition_offset: int
    # the true source frame offset
    then_offset: int
    # the false source frame offset
    else_offset: int
    # the selected byte length
    byte_len: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> AggregateSelect: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> AggregateSelect: ...

def encode_aggregate_select(writer: BinaryWriter, value: AggregateSelect) -> None: ...
def decode_aggregate_select(reader: BinaryReader) -> AggregateSelect: ...
def to_json_aggregate_select(value: AggregateSelect) -> Json: ...
def from_json_aggregate_select(value: Json) -> AggregateSelect: ...

@dataclass(frozen=True, slots=True)
class AtomicCompareExchange:
    """Atomic compare exchange over one scalar value."""

    # the aggregate destination value
    destination: destack._generated.mir.tree.value.Value
    # the pointer cell offset
    pointer_offset: int
    # the expected value cell offset
    expected_offset: int
    # the replacement value cell offset
    new_value_offset: int
    # the atomic memory shape
    shape: destack._generated.program.vm.atomic.AtomicShape
    # the failure ordering
    failure_order: destack._generated.program.vm.atomic.AtomicOrder
    # whether the compare exchange is weak
    is_weak: bool

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> AtomicCompareExchange: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> AtomicCompareExchange: ...

def encode_atomic_compare_exchange(
    writer: BinaryWriter, value: AtomicCompareExchange
) -> None: ...
def decode_atomic_compare_exchange(reader: BinaryReader) -> AtomicCompareExchange: ...
def to_json_atomic_compare_exchange(value: AtomicCompareExchange) -> Json: ...
def from_json_atomic_compare_exchange(value: Json) -> AtomicCompareExchange: ...

@dataclass(frozen=True, slots=True)
class VectorSplat:
    """Vector splat operation."""

    # the destination frame offset
    dest_offset: int
    # the splatted value cell offset
    value_offset: int
    # the destination element projection
    dest_element: destack._generated.program.vm.projection.Projection
    # the destination element count
    element_count: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> VectorSplat: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> VectorSplat: ...

def encode_vector_splat(writer: BinaryWriter, value: VectorSplat) -> None: ...
def decode_vector_splat(reader: BinaryReader) -> VectorSplat: ...
def to_json_vector_splat(value: VectorSplat) -> Json: ...
def from_json_vector_splat(value: Json) -> VectorSplat: ...

@dataclass(frozen=True, slots=True)
class VectorExtract:
    """Vector extract operation."""

    # the destination cell offset
    dest_offset: int
    # the source vector frame offset
    vector_offset: int
    # the index cell offset
    index_offset: int
    # the source element projection
    vector_element: destack._generated.program.vm.projection.Projection
    # the source element count
    element_count: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> VectorExtract: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> VectorExtract: ...

def encode_vector_extract(writer: BinaryWriter, value: VectorExtract) -> None: ...
def decode_vector_extract(reader: BinaryReader) -> VectorExtract: ...
def to_json_vector_extract(value: VectorExtract) -> Json: ...
def from_json_vector_extract(value: Json) -> VectorExtract: ...

@dataclass(frozen=True, slots=True)
class VectorBinary:
    """Elementwise vector binary operation."""

    # the destination frame offset
    dest_offset: int
    # the left vector frame offset
    left_offset: int
    # the right vector frame offset
    right_offset: int
    # the destination element projection
    dest_element: destack._generated.program.vm.projection.Projection
    # the left source element projection
    left_element: destack._generated.program.vm.projection.Projection
    # the right source element projection
    right_element: destack._generated.program.vm.projection.Projection
    # the element binary kernel
    kernel: ElementBinaryKernel
    # the vector element scalar layout
    element_layout: destack._generated.program.vm.value.ScalarLayout
    # the destination element count
    element_count: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> VectorBinary: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> VectorBinary: ...

def encode_vector_binary(writer: BinaryWriter, value: VectorBinary) -> None: ...
def decode_vector_binary(reader: BinaryReader) -> VectorBinary: ...
def to_json_vector_binary(value: VectorBinary) -> Json: ...
def from_json_vector_binary(value: Json) -> VectorBinary: ...

"""Elementwise scalar binary kernel."""
ElementBinaryKernel: typing.TypeAlias = (
    typing.Literal["andBool"]
    | typing.Literal["orBool"]
    | typing.Literal["xorBool"]
    | typing.Literal["eqBool"]
    | typing.Literal["neBool"]
    | typing.Literal["addInt"]
    | typing.Literal["subInt"]
    | typing.Literal["mulInt"]
    | typing.Literal["divInt"]
    | typing.Literal["divUint"]
    | typing.Literal["remInt"]
    | typing.Literal["remUint"]
    | typing.Literal["andInt"]
    | typing.Literal["orInt"]
    | typing.Literal["xorInt"]
    | typing.Literal["shlInt"]
    | typing.Literal["shrInt"]
    | typing.Literal["shrUint"]
    | typing.Literal["eqInt"]
    | typing.Literal["neInt"]
    | typing.Literal["ltInt"]
    | typing.Literal["ltUint"]
    | typing.Literal["leInt"]
    | typing.Literal["leUint"]
    | typing.Literal["gtInt"]
    | typing.Literal["gtUint"]
    | typing.Literal["geInt"]
    | typing.Literal["geUint"]
    | typing.Literal["addF32"]
    | typing.Literal["subF32"]
    | typing.Literal["mulF32"]
    | typing.Literal["divF32"]
    | typing.Literal["addF64"]
    | typing.Literal["subF64"]
    | typing.Literal["mulF64"]
    | typing.Literal["divF64"]
    | typing.Literal["eqF32"]
    | typing.Literal["eqF64"]
    | typing.Literal["neF32"]
    | typing.Literal["neF64"]
    | typing.Literal["ltF32"]
    | typing.Literal["ltF64"]
    | typing.Literal["leF32"]
    | typing.Literal["leF64"]
    | typing.Literal["gtF32"]
    | typing.Literal["gtF64"]
    | typing.Literal["geF32"]
    | typing.Literal["geF64"]
    | typing.Literal["addFloat"]
    | typing.Literal["subFloat"]
    | typing.Literal["mulFloat"]
    | typing.Literal["divFloat"]
    | typing.Literal["eqFloat"]
    | typing.Literal["neFloat"]
    | typing.Literal["ltFloat"]
    | typing.Literal["leFloat"]
    | typing.Literal["gtFloat"]
    | typing.Literal["geFloat"]
)

def encode_element_binary_kernel(
    writer: BinaryWriter, value: ElementBinaryKernel
) -> None: ...
def decode_element_binary_kernel(reader: BinaryReader) -> ElementBinaryKernel: ...
def to_json_element_binary_kernel(value: ElementBinaryKernel) -> Json: ...
def from_json_element_binary_kernel(value: Json) -> ElementBinaryKernel: ...

@dataclass(frozen=True, slots=True)
class VectorUnary:
    """Elementwise vector unary operation."""

    # the destination frame offset
    dest_offset: int
    # the argument vector frame offset
    argument_offset: int
    # the destination element projection
    dest_element: destack._generated.program.vm.projection.Projection
    # the argument element projection
    argument_element: destack._generated.program.vm.projection.Projection
    # the element unary kernel
    kernel: ElementUnaryKernel
    # the vector element scalar layout
    element_layout: destack._generated.program.vm.value.ScalarLayout
    # the destination element count
    element_count: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> VectorUnary: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> VectorUnary: ...

def encode_vector_unary(writer: BinaryWriter, value: VectorUnary) -> None: ...
def decode_vector_unary(reader: BinaryReader) -> VectorUnary: ...
def to_json_vector_unary(value: VectorUnary) -> Json: ...
def from_json_vector_unary(value: Json) -> VectorUnary: ...

"""Elementwise scalar unary kernel."""
ElementUnaryKernel: typing.TypeAlias = (
    typing.Literal["notBool"]
    | typing.Literal["negInt"]
    | typing.Literal["notInt"]
    | typing.Literal["negF32"]
    | typing.Literal["negF64"]
    | typing.Literal["negFloat"]
)

def encode_element_unary_kernel(
    writer: BinaryWriter, value: ElementUnaryKernel
) -> None: ...
def decode_element_unary_kernel(reader: BinaryReader) -> ElementUnaryKernel: ...
def to_json_element_unary_kernel(value: ElementUnaryKernel) -> Json: ...
def from_json_element_unary_kernel(value: Json) -> ElementUnaryKernel: ...

@dataclass(frozen=True, slots=True)
class VectorInsert:
    """Vector insert operation."""

    # the destination frame offset
    dest_offset: int
    # the source vector frame offset
    vector_offset: int
    # the index cell offset
    index_offset: int
    # the inserted value cell offset
    value_offset: int
    # the destination element projection
    dest_element: destack._generated.program.vm.projection.Projection
    # the source element projection
    vector_element: destack._generated.program.vm.projection.Projection
    # the source vector element count
    element_count: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> VectorInsert: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> VectorInsert: ...

def encode_vector_insert(writer: BinaryWriter, value: VectorInsert) -> None: ...
def decode_vector_insert(reader: BinaryReader) -> VectorInsert: ...
def to_json_vector_insert(value: VectorInsert) -> Json: ...
def from_json_vector_insert(value: Json) -> VectorInsert: ...

@dataclass(frozen=True, slots=True)
class VectorShuffle:
    """Vector shuffle operation."""

    # the destination frame offset
    dest_offset: int
    # the left source frame offset
    left_offset: int
    # the right source frame offset
    right_offset: int
    # the pooled shuffle mask
    mask: destack._generated.program.vm.table.U32RangeId
    # the destination element projection
    dest_element: destack._generated.program.vm.projection.Projection
    # the left source element projection
    left_element: destack._generated.program.vm.projection.Projection
    # the right source element projection
    right_element: destack._generated.program.vm.projection.Projection
    # the left source element count
    left_count: int
    # the right source element count
    right_count: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> VectorShuffle: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> VectorShuffle: ...

def encode_vector_shuffle(writer: BinaryWriter, value: VectorShuffle) -> None: ...
def decode_vector_shuffle(reader: BinaryReader) -> VectorShuffle: ...
def to_json_vector_shuffle(value: VectorShuffle) -> Json: ...
def from_json_vector_shuffle(value: Json) -> VectorShuffle: ...

@dataclass(frozen=True, slots=True)
class VectorSelect:
    """Vector select operation."""

    # the destination frame offset
    dest_offset: int
    # the mask vector frame offset
    mask_offset: int
    # the true branch frame offset
    then_offset: int
    # the false branch frame offset
    else_offset: int
    # the destination element projection
    dest_element: destack._generated.program.vm.projection.Projection
    # the mask element projection
    mask_element: destack._generated.program.vm.projection.Projection
    # the true branch element projection
    then_element: destack._generated.program.vm.projection.Projection
    # the false branch element projection
    else_element: destack._generated.program.vm.projection.Projection
    # the destination element count
    element_count: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> VectorSelect: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> VectorSelect: ...

def encode_vector_select(writer: BinaryWriter, value: VectorSelect) -> None: ...
def decode_vector_select(reader: BinaryReader) -> VectorSelect: ...
def to_json_vector_select(value: VectorSelect) -> Json: ...
def from_json_vector_select(value: Json) -> VectorSelect: ...

@dataclass(frozen=True, slots=True)
class VectorReduce:
    """Vector reduction operation."""

    # the destination cell offset
    dest_offset: int
    # the source vector frame offset
    vector_offset: int
    # the reduction kernel
    kernel: destack._generated.mir.tree.vector.VectorReduceOperator
    # the source element projection
    vector_element: destack._generated.program.vm.projection.Projection
    # the vector element scalar layout
    element_layout: destack._generated.program.vm.value.ScalarLayout
    # the source vector element count
    element_count: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> VectorReduce: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> VectorReduce: ...

def encode_vector_reduce(writer: BinaryWriter, value: VectorReduce) -> None: ...
def decode_vector_reduce(reader: BinaryReader) -> VectorReduce: ...
def to_json_vector_reduce(value: VectorReduce) -> Json: ...
def from_json_vector_reduce(value: Json) -> VectorReduce: ...

@dataclass(frozen=True, slots=True)
class VectorConvert:
    """Vector conversion operation."""

    # the destination frame offset
    dest_offset: int
    # the source vector frame offset
    vector_offset: int
    # the conversion kernel
    mode: destack._generated.mir.tree.vector.VectorConvertMode
    # the destination element projection
    dest_element: destack._generated.program.vm.projection.Projection
    # the source element projection
    source_element: destack._generated.program.vm.projection.Projection
    # the destination element scalar layout
    dest_layout: destack._generated.program.vm.value.ScalarLayout
    # the source element scalar layout
    source_layout: destack._generated.program.vm.value.ScalarLayout
    # the destination element count
    element_count: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> VectorConvert: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> VectorConvert: ...

def encode_vector_convert(writer: BinaryWriter, value: VectorConvert) -> None: ...
def decode_vector_convert(reader: BinaryReader) -> VectorConvert: ...
def to_json_vector_convert(value: VectorConvert) -> Json: ...
def from_json_vector_convert(value: Json) -> VectorConvert: ...

@dataclass(frozen=True, slots=True)
class FunctionBind:
    """Function bind operation."""

    # the environment cell layout
    environment: destack._generated.program.vm.value.CellLayout

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionBind: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FunctionBind: ...

def encode_function_bind(writer: BinaryWriter, value: FunctionBind) -> None: ...
def decode_function_bind(reader: BinaryReader) -> FunctionBind: ...
def to_json_function_bind(value: FunctionBind) -> Json: ...
def from_json_function_bind(value: Json) -> FunctionBind: ...

@dataclass(frozen=True, slots=True)
class Call:
    """Direct function call."""

    # the callee function index
    function: int
    # the resolved call target
    target: destack._generated.program.vm.function.CallTarget
    # the pooled argument range
    arguments: destack._generated.program.vm.range.ArgumentRange
    # the argument frame moves
    moves: destack._generated.program.vm.range.MoveRange

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Call: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Call: ...

def encode_call(writer: BinaryWriter, value: Call) -> None: ...
def decode_call(reader: BinaryReader) -> Call: ...
def to_json_call(value: Call) -> Json: ...
def from_json_call(value: Json) -> Call: ...

@dataclass(frozen=True, slots=True)
class CallBranch:
    """Function call terminator with an explicit continuation."""

    # the callee function index
    function: int
    # the resolved call target
    target: destack._generated.program.vm.function.CallTarget
    # the pooled argument range
    arguments: destack._generated.program.vm.range.ArgumentRange
    # the continuation frame state
    target_state: destack._generated.mir.metadata.frame.FrameStateId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CallBranch: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CallBranch: ...

def encode_call_branch(writer: BinaryWriter, value: CallBranch) -> None: ...
def decode_call_branch(reader: BinaryReader) -> CallBranch: ...
def to_json_call_branch(value: CallBranch) -> Json: ...
def from_json_call_branch(value: Json) -> CallBranch: ...

@dataclass(frozen=True, slots=True)
class CallVirtual:
    """Class method call."""

    # the receiver cell offset
    receiver_offset: int
    # the dispatch table field projection
    table_field: destack._generated.program.vm.table.ProjectionId
    # the dispatch table slot
    slot: int
    # the pooled argument range
    arguments: destack._generated.program.vm.range.ArgumentRange

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CallVirtual: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CallVirtual: ...

def encode_call_virtual(writer: BinaryWriter, value: CallVirtual) -> None: ...
def decode_call_virtual(reader: BinaryReader) -> CallVirtual: ...
def to_json_call_virtual(value: CallVirtual) -> Json: ...
def from_json_call_virtual(value: Json) -> CallVirtual: ...

@dataclass(frozen=True, slots=True)
class CallVirtualBranch:
    """Class method call terminator with an explicit continuation."""

    # the receiver cell offset
    receiver_offset: int
    # the dispatch table field projection
    table_field: destack._generated.program.vm.table.ProjectionId
    # the dispatch table slot
    slot: int
    # the pooled argument range
    arguments: destack._generated.program.vm.range.ArgumentRange
    # the continuation frame state
    target_state: destack._generated.mir.metadata.frame.FrameStateId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CallVirtualBranch: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CallVirtualBranch: ...

def encode_call_virtual_branch(
    writer: BinaryWriter, value: CallVirtualBranch
) -> None: ...
def decode_call_virtual_branch(reader: BinaryReader) -> CallVirtualBranch: ...
def to_json_call_virtual_branch(value: CallVirtualBranch) -> Json: ...
def from_json_call_virtual_branch(value: Json) -> CallVirtualBranch: ...

@dataclass(frozen=True, slots=True)
class CallDynamic:
    """Dynamic method call."""

    # the receiver cell offset
    receiver_offset: int
    # the dynamic table field projection
    table_field: destack._generated.program.vm.table.ProjectionId
    # the dynamic table slot
    slot: int
    # the pooled argument range
    arguments: destack._generated.program.vm.range.ArgumentRange

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CallDynamic: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CallDynamic: ...

def encode_call_dynamic(writer: BinaryWriter, value: CallDynamic) -> None: ...
def decode_call_dynamic(reader: BinaryReader) -> CallDynamic: ...
def to_json_call_dynamic(value: CallDynamic) -> Json: ...
def from_json_call_dynamic(value: Json) -> CallDynamic: ...

@dataclass(frozen=True, slots=True)
class CallDynamicBranch:
    """Dynamic method call terminator with an explicit continuation."""

    # the receiver cell offset
    receiver_offset: int
    # the dynamic table field projection
    table_field: destack._generated.program.vm.table.ProjectionId
    # the dynamic table slot
    slot: int
    # the pooled argument range
    arguments: destack._generated.program.vm.range.ArgumentRange
    # the continuation frame state
    target_state: destack._generated.mir.metadata.frame.FrameStateId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CallDynamicBranch: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CallDynamicBranch: ...

def encode_call_dynamic_branch(
    writer: BinaryWriter, value: CallDynamicBranch
) -> None: ...
def decode_call_dynamic_branch(reader: BinaryReader) -> CallDynamicBranch: ...
def to_json_call_dynamic_branch(value: CallDynamicBranch) -> Json: ...
def from_json_call_dynamic_branch(value: Json) -> CallDynamicBranch: ...

@dataclass(frozen=True, slots=True)
class IndirectCall:
    """Indirect function call."""

    # the callee cell offset
    callee_offset: int
    # the expected function signature
    signature: destack._generated.mir.tree.node.LocalNodeId
    # the pooled argument range
    arguments: destack._generated.program.vm.range.ArgumentRange

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> IndirectCall: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> IndirectCall: ...

def encode_indirect_call(writer: BinaryWriter, value: IndirectCall) -> None: ...
def decode_indirect_call(reader: BinaryReader) -> IndirectCall: ...
def to_json_indirect_call(value: IndirectCall) -> Json: ...
def from_json_indirect_call(value: Json) -> IndirectCall: ...

@dataclass(frozen=True, slots=True)
class IndirectCallBranch:
    """Indirect call terminator with an explicit continuation."""

    # the callee cell offset
    callee_offset: int
    # the expected function signature
    signature: destack._generated.mir.tree.node.LocalNodeId
    # the pooled argument range
    arguments: destack._generated.program.vm.range.ArgumentRange
    # the continuation frame state
    target_state: destack._generated.mir.metadata.frame.FrameStateId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> IndirectCallBranch: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> IndirectCallBranch: ...

def encode_indirect_call_branch(
    writer: BinaryWriter, value: IndirectCallBranch
) -> None: ...
def decode_indirect_call_branch(reader: BinaryReader) -> IndirectCallBranch: ...
def to_json_indirect_call_branch(value: IndirectCallBranch) -> Json: ...
def from_json_indirect_call_branch(value: Json) -> IndirectCallBranch: ...

@dataclass(frozen=True, slots=True)
class TensorLoad:
    """Load a tensor element from a view."""

    # the destination scalar
    dest_offset: int
    # the source tensor view
    view_offset: int
    # the index frame offsets
    indices: destack._generated.program.vm.table.U32RangeId
    # the tensor view layout
    view_layout: destack._generated.program.vm.table.TensorLayoutId
    # the tensor element projection
    element: destack._generated.program.vm.table.ProjectionId
    # the tensor view backing memory
    address: destack._generated.program.vm.tensor.TensorAddress

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorLoad: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TensorLoad: ...

def encode_tensor_load(writer: BinaryWriter, value: TensorLoad) -> None: ...
def decode_tensor_load(reader: BinaryReader) -> TensorLoad: ...
def to_json_tensor_load(value: TensorLoad) -> Json: ...
def from_json_tensor_load(value: Json) -> TensorLoad: ...

@dataclass(frozen=True, slots=True)
class TensorExtract:
    """Extract a tensor element from a tensor value."""

    # the destination scalar frame offset
    dest_offset: int
    # the source tensor frame offset
    tensor_offset: int
    # the index frame offsets
    indices: destack._generated.program.vm.table.U32RangeId
    # the source tensor layout
    tensor_layout: destack._generated.program.vm.table.TensorLayoutId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorExtract: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TensorExtract: ...

def encode_tensor_extract(writer: BinaryWriter, value: TensorExtract) -> None: ...
def decode_tensor_extract(reader: BinaryReader) -> TensorExtract: ...
def to_json_tensor_extract(value: TensorExtract) -> Json: ...
def from_json_tensor_extract(value: Json) -> TensorExtract: ...

@dataclass(frozen=True, slots=True)
class TensorBinary:
    """Elementwise tensor binary operation."""

    # the destination tensor frame offset
    dest_offset: int
    # the left tensor frame offset
    left_offset: int
    # the right tensor frame offset
    right_offset: int
    # the left tensor layout
    left_layout: destack._generated.program.vm.table.TensorLayoutId
    # the right tensor layout
    right_layout: destack._generated.program.vm.table.TensorLayoutId
    # the destination tensor layout
    dest_layout: destack._generated.program.vm.table.TensorLayoutId
    # the element binary kernel
    kernel: ElementBinaryKernel
    # the tensor element scalar layout
    element_layout: destack._generated.program.vm.value.ScalarLayout

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorBinary: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TensorBinary: ...

def encode_tensor_binary(writer: BinaryWriter, value: TensorBinary) -> None: ...
def decode_tensor_binary(reader: BinaryReader) -> TensorBinary: ...
def to_json_tensor_binary(value: TensorBinary) -> Json: ...
def from_json_tensor_binary(value: Json) -> TensorBinary: ...

@dataclass(frozen=True, slots=True)
class TensorContiguousBinary:
    """Contiguous elementwise tensor binary operation."""

    # the destination tensor frame offset
    dest_offset: int
    # the left tensor frame offset
    left_offset: int
    # the right tensor frame offset
    right_offset: int
    # the destination tensor layout
    dest_layout: destack._generated.program.vm.table.TensorLayoutId
    # the tensor element scalar layout
    element_layout: destack._generated.program.vm.value.ScalarLayout
    # the contiguous binary kernel selected during lowering
    kernel: ElementBinaryKernel

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorContiguousBinary: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TensorContiguousBinary: ...

def encode_tensor_contiguous_binary(
    writer: BinaryWriter, value: TensorContiguousBinary
) -> None: ...
def decode_tensor_contiguous_binary(reader: BinaryReader) -> TensorContiguousBinary: ...
def to_json_tensor_contiguous_binary(value: TensorContiguousBinary) -> Json: ...
def from_json_tensor_contiguous_binary(value: Json) -> TensorContiguousBinary: ...

@dataclass(frozen=True, slots=True)
class TensorUnary:
    """Elementwise tensor unary operation."""

    # the destination tensor frame offset
    dest_offset: int
    # the source tensor frame offset
    argument_offset: int
    # the argument tensor layout
    argument_layout: destack._generated.program.vm.table.TensorLayoutId
    # the tensor element scalar layout
    element_layout: destack._generated.program.vm.value.ScalarLayout
    # the destination tensor layout
    dest_layout: destack._generated.program.vm.table.TensorLayoutId
    # the element unary kernel
    kernel: ElementUnaryKernel

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorUnary: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TensorUnary: ...

def encode_tensor_unary(writer: BinaryWriter, value: TensorUnary) -> None: ...
def decode_tensor_unary(reader: BinaryReader) -> TensorUnary: ...
def to_json_tensor_unary(value: TensorUnary) -> Json: ...
def from_json_tensor_unary(value: Json) -> TensorUnary: ...

@dataclass(frozen=True, slots=True)
class TensorContiguousUnary:
    """Contiguous elementwise tensor unary operation."""

    # the destination tensor frame offset
    dest_offset: int
    # the source tensor frame offset
    argument_offset: int
    # the destination tensor layout
    dest_layout: destack._generated.program.vm.table.TensorLayoutId
    # the tensor element scalar layout
    element_layout: destack._generated.program.vm.value.ScalarLayout
    # the contiguous unary kernel selected during lowering
    kernel: ElementUnaryKernel

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorContiguousUnary: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TensorContiguousUnary: ...

def encode_tensor_contiguous_unary(
    writer: BinaryWriter, value: TensorContiguousUnary
) -> None: ...
def decode_tensor_contiguous_unary(reader: BinaryReader) -> TensorContiguousUnary: ...
def to_json_tensor_contiguous_unary(value: TensorContiguousUnary) -> Json: ...
def from_json_tensor_contiguous_unary(value: Json) -> TensorContiguousUnary: ...

@dataclass(frozen=True, slots=True)
class TensorStore:
    """Store a tensor element into a view."""

    # the destination tensor view
    view_offset: int
    # the index frame offsets
    indices: destack._generated.program.vm.table.U32RangeId
    # the stored scalar
    value_offset: int
    # the tensor view layout
    view_layout: destack._generated.program.vm.table.TensorLayoutId
    # the tensor element projection
    element: destack._generated.program.vm.table.ProjectionId
    # the tensor view backing memory
    address: destack._generated.program.vm.tensor.TensorAddress

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorStore: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TensorStore: ...

def encode_tensor_store(writer: BinaryWriter, value: TensorStore) -> None: ...
def decode_tensor_store(reader: BinaryReader) -> TensorStore: ...
def to_json_tensor_store(value: TensorStore) -> Json: ...
def from_json_tensor_store(value: Json) -> TensorStore: ...

@dataclass(frozen=True, slots=True)
class TensorFill:
    """Fill a tensor reference with a scalar value."""

    # the filled tensor view
    view_offset: int
    # the scalar fill value
    value_offset: int
    # the tensor view layout
    view_layout: destack._generated.program.vm.table.TensorLayoutId
    # the tensor element projection
    element: destack._generated.program.vm.table.ProjectionId
    # the tensor view backing memory
    address: destack._generated.program.vm.tensor.TensorAddress

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorFill: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TensorFill: ...

def encode_tensor_fill(writer: BinaryWriter, value: TensorFill) -> None: ...
def decode_tensor_fill(reader: BinaryReader) -> TensorFill: ...
def to_json_tensor_fill(value: TensorFill) -> Json: ...
def from_json_tensor_fill(value: Json) -> TensorFill: ...

@dataclass(frozen=True, slots=True)
class TensorCopy:
    """Copy elements between tensor references."""

    # the target tensor view
    target_offset: int
    # the source tensor view
    source_offset: int
    # the target tensor view layout
    target_layout: destack._generated.program.vm.table.TensorLayoutId
    # the source tensor view layout
    source_layout: destack._generated.program.vm.table.TensorLayoutId
    # the target element projection
    target_element: destack._generated.program.vm.table.ProjectionId
    # the source element projection
    source_element: destack._generated.program.vm.table.ProjectionId
    # the target tensor backing memory
    target_address: destack._generated.program.vm.tensor.TensorAddress
    # the source tensor backing memory
    source_address: destack._generated.program.vm.tensor.TensorAddress

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorCopy: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TensorCopy: ...

def encode_tensor_copy(writer: BinaryWriter, value: TensorCopy) -> None: ...
def decode_tensor_copy(reader: BinaryReader) -> TensorCopy: ...
def to_json_tensor_copy(value: TensorCopy) -> Json: ...
def from_json_tensor_copy(value: Json) -> TensorCopy: ...

@dataclass(frozen=True, slots=True)
class TensorViewCast:
    """Cast a dense pointer into a tensor view descriptor."""

    # the destination tensor view
    dest_offset: int
    # the MIR pointer cell
    pointer_offset: int
    # the tensor view layout
    view_layout: destack._generated.program.vm.table.TensorLayoutId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorViewCast: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TensorViewCast: ...

def encode_tensor_view_cast(writer: BinaryWriter, value: TensorViewCast) -> None: ...
def decode_tensor_view_cast(reader: BinaryReader) -> TensorViewCast: ...
def to_json_tensor_view_cast(value: TensorViewCast) -> Json: ...
def from_json_tensor_view_cast(value: Json) -> TensorViewCast: ...

@dataclass(frozen=True, slots=True)
class TensorReshape:
    """Reshape a tensor into a new shape."""

    # the destination tensor frame offset
    dest_offset: int
    # the source tensor frame offset
    tensor_offset: int
    # the destination shape values
    shape: destack._generated.program.vm.table.U32RangeId
    # the source tensor layout
    source_layout: destack._generated.program.vm.table.TensorLayoutId
    # the destination tensor layout
    dest_layout: destack._generated.program.vm.table.TensorLayoutId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorReshape: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TensorReshape: ...

def encode_tensor_reshape(writer: BinaryWriter, value: TensorReshape) -> None: ...
def decode_tensor_reshape(reader: BinaryReader) -> TensorReshape: ...
def to_json_tensor_reshape(value: TensorReshape) -> Json: ...
def from_json_tensor_reshape(value: Json) -> TensorReshape: ...

@dataclass(frozen=True, slots=True)
class TensorBroadcast:
    """Broadcast a tensor into a larger shape."""

    # the destination tensor frame offset
    dest_offset: int
    # the source tensor frame offset
    tensor_offset: int
    # the broadcast dimension mapping
    dimensions: destack._generated.program.vm.table.U32RangeId
    # the source tensor layout
    source_layout: destack._generated.program.vm.table.TensorLayoutId
    # the destination tensor layout
    dest_layout: destack._generated.program.vm.table.TensorLayoutId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorBroadcast: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TensorBroadcast: ...

def encode_tensor_broadcast(writer: BinaryWriter, value: TensorBroadcast) -> None: ...
def decode_tensor_broadcast(reader: BinaryReader) -> TensorBroadcast: ...
def to_json_tensor_broadcast(value: TensorBroadcast) -> Json: ...
def from_json_tensor_broadcast(value: Json) -> TensorBroadcast: ...

@dataclass(frozen=True, slots=True)
class TensorTranspose:
    """Permute tensor dimensions."""

    # the destination tensor frame offset
    dest_offset: int
    # the source tensor frame offset
    tensor_offset: int
    # the dimension permutation
    permutation: destack._generated.program.vm.table.U32RangeId
    # the source tensor layout
    source_layout: destack._generated.program.vm.table.TensorLayoutId
    # the destination tensor layout
    dest_layout: destack._generated.program.vm.table.TensorLayoutId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorTranspose: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TensorTranspose: ...

def encode_tensor_transpose(writer: BinaryWriter, value: TensorTranspose) -> None: ...
def decode_tensor_transpose(reader: BinaryReader) -> TensorTranspose: ...
def to_json_tensor_transpose(value: TensorTranspose) -> Json: ...
def from_json_tensor_transpose(value: Json) -> TensorTranspose: ...

@dataclass(frozen=True, slots=True)
class TensorSlice:
    """Slice a tensor by offsets, sizes, and strides."""

    # the destination tensor frame offset
    dest_offset: int
    # the source tensor frame offset
    tensor_offset: int
    # the packed offset, size, and stride values
    arguments: destack._generated.program.vm.table.U32RangeId
    # the number of offset values
    offsets_count: int
    # the number of size values
    sizes_count: int
    # the number of stride values
    strides_count: int
    # the source tensor layout
    source_layout: destack._generated.program.vm.table.TensorLayoutId
    # the destination tensor layout
    dest_layout: destack._generated.program.vm.table.TensorLayoutId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorSlice: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TensorSlice: ...

def encode_tensor_slice(writer: BinaryWriter, value: TensorSlice) -> None: ...
def decode_tensor_slice(reader: BinaryReader) -> TensorSlice: ...
def to_json_tensor_slice(value: TensorSlice) -> Json: ...
def from_json_tensor_slice(value: Json) -> TensorSlice: ...

@dataclass(frozen=True, slots=True)
class TensorPad:
    """Pad a tensor with low, high, and interior padding."""

    # the destination tensor frame offset
    dest_offset: int
    # the source tensor frame offset
    tensor_offset: int
    # the packed low, high, and interior padding values
    arguments: destack._generated.program.vm.table.U32RangeId
    # the number of low padding values
    low_count: int
    # the number of high padding values
    high_count: int
    # the number of interior padding values
    interior_count: int
    # the padding value cell offset
    value_offset: int
    # the source tensor layout
    source_layout: destack._generated.program.vm.table.TensorLayoutId
    # the destination tensor layout
    dest_layout: destack._generated.program.vm.table.TensorLayoutId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorPad: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TensorPad: ...

def encode_tensor_pad(writer: BinaryWriter, value: TensorPad) -> None: ...
def decode_tensor_pad(reader: BinaryReader) -> TensorPad: ...
def to_json_tensor_pad(value: TensorPad) -> Json: ...
def from_json_tensor_pad(value: Json) -> TensorPad: ...

@dataclass(frozen=True, slots=True)
class TensorConcat:
    """Concatenate tensors along a dimension."""

    # the destination tensor frame offset
    dest_offset: int
    # the source tensor frame offsets
    tensors: destack._generated.program.vm.table.U32RangeId
    # the source tensor layouts
    tensor_layouts: destack._generated.program.vm.table.U32RangeId
    # the concatenation axis
    axis: int
    # the destination tensor layout
    dest_layout: destack._generated.program.vm.table.TensorLayoutId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorConcat: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TensorConcat: ...

def encode_tensor_concat(writer: BinaryWriter, value: TensorConcat) -> None: ...
def decode_tensor_concat(reader: BinaryReader) -> TensorConcat: ...
def to_json_tensor_concat(value: TensorConcat) -> Json: ...
def from_json_tensor_concat(value: Json) -> TensorConcat: ...

@dataclass(frozen=True, slots=True)
class TensorReduce:
    """Reduce a tensor along axes."""

    # the destination tensor frame offset
    dest_offset: int
    # the source tensor frame offset
    tensor_offset: int
    # the initial value cell offset
    initial_offset: int
    # the reduced axis range
    axes: destack._generated.program.vm.table.U32RangeId
    # the source tensor layout
    source_layout: destack._generated.program.vm.table.TensorLayoutId
    # the destination tensor layout
    dest_layout: destack._generated.program.vm.table.TensorLayoutId
    # the reduction kernel
    kernel: destack._generated.mir.tree.tensor.TensorReduceOperator

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorReduce: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TensorReduce: ...

def encode_tensor_reduce(writer: BinaryWriter, value: TensorReduce) -> None: ...
def decode_tensor_reduce(reader: BinaryReader) -> TensorReduce: ...
def to_json_tensor_reduce(value: TensorReduce) -> Json: ...
def from_json_tensor_reduce(value: Json) -> TensorReduce: ...

@dataclass(frozen=True, slots=True)
class TensorIndexReduce:
    """Reduce a tensor along one axis and return selected source indices."""

    # the destination tensor frame offset
    dest_offset: int
    # the source tensor frame offset
    tensor_offset: int
    # the reduced axis
    axis: int
    # the source tensor layout
    source_layout: destack._generated.program.vm.table.TensorLayoutId
    # the destination tensor layout
    dest_layout: destack._generated.program.vm.table.TensorLayoutId
    # the index reduction kernel
    kernel: destack._generated.mir.tree.tensor.TensorIndexReduceOperator
    # the behavior for equal selected values
    tie_break: destack._generated.mir.tree.tensor.TensorIndexTieBreak

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorIndexReduce: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TensorIndexReduce: ...

def encode_tensor_index_reduce(
    writer: BinaryWriter, value: TensorIndexReduce
) -> None: ...
def decode_tensor_index_reduce(reader: BinaryReader) -> TensorIndexReduce: ...
def to_json_tensor_index_reduce(value: TensorIndexReduce) -> Json: ...
def from_json_tensor_index_reduce(value: Json) -> TensorIndexReduce: ...

@dataclass(frozen=True, slots=True)
class TensorDot:
    """Dot product of two tensors."""

    # the destination tensor frame offset
    dest_offset: int
    # the left tensor frame offset
    left_offset: int
    # the right tensor frame offset
    right_offset: int
    # the dot dimension numbers
    dimensions: destack._generated.program.vm.table.TensorDotId
    # the left tensor layout
    left_layout: destack._generated.program.vm.table.TensorLayoutId
    # the right tensor layout
    right_layout: destack._generated.program.vm.table.TensorLayoutId
    # the destination tensor layout
    dest_layout: destack._generated.program.vm.table.TensorLayoutId
    # the tensor element scalar layout
    element_layout: destack._generated.program.vm.value.ScalarLayout

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorDot: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TensorDot: ...

def encode_tensor_dot(writer: BinaryWriter, value: TensorDot) -> None: ...
def decode_tensor_dot(reader: BinaryReader) -> TensorDot: ...
def to_json_tensor_dot(value: TensorDot) -> Json: ...
def from_json_tensor_dot(value: Json) -> TensorDot: ...

@dataclass(frozen=True, slots=True)
class TensorConvolution:
    """Convolution between an input tensor and a kernel tensor."""

    # the destination tensor frame offset
    dest_offset: int
    # the input tensor frame offset
    input_offset: int
    # the kernel tensor frame offset
    kernel_offset: int
    # the convolution dimension numbers
    dimensions: destack._generated.program.vm.table.TensorConvolutionId
    # the convolution window descriptor
    window: destack._generated.program.vm.table.TensorWindowId
    # the feature group count
    feature_group_count: int
    # the batch group count
    batch_group_count: int
    # the input tensor layout
    input_layout: destack._generated.program.vm.table.TensorLayoutId
    # the kernel tensor layout
    kernel_layout: destack._generated.program.vm.table.TensorLayoutId
    # the destination tensor layout
    dest_layout: destack._generated.program.vm.table.TensorLayoutId
    # the tensor element scalar layout
    element_layout: destack._generated.program.vm.value.ScalarLayout

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorConvolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TensorConvolution: ...

def encode_tensor_convolution(
    writer: BinaryWriter, value: TensorConvolution
) -> None: ...
def decode_tensor_convolution(reader: BinaryReader) -> TensorConvolution: ...
def to_json_tensor_convolution(value: TensorConvolution) -> Json: ...
def from_json_tensor_convolution(value: Json) -> TensorConvolution: ...

@dataclass(frozen=True, slots=True)
class TensorGather:
    """Gather slices from a tensor based on indices."""

    # the destination tensor frame offset
    dest_offset: int
    # the source tensor frame offset
    source_offset: int
    # the indices tensor frame offset
    indices_offset: int
    # the gather dimension numbers
    dimensions: destack._generated.program.vm.table.TensorGatherId
    # the gathered slice sizes
    slice_sizes: destack._generated.program.vm.table.U32RangeId
    # the source tensor layout
    source_layout: destack._generated.program.vm.table.TensorLayoutId
    # the indices tensor layout
    indices_layout: destack._generated.program.vm.table.TensorLayoutId
    # the destination tensor layout
    dest_layout: destack._generated.program.vm.table.TensorLayoutId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorGather: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TensorGather: ...

def encode_tensor_gather(writer: BinaryWriter, value: TensorGather) -> None: ...
def decode_tensor_gather(reader: BinaryReader) -> TensorGather: ...
def to_json_tensor_gather(value: TensorGather) -> Json: ...
def from_json_tensor_gather(value: Json) -> TensorGather: ...

@dataclass(frozen=True, slots=True)
class TensorScatter:
    """Scatter updates into a tensor based on indices."""

    # the destination tensor frame offset
    dest_offset: int
    # the source tensor frame offset
    source_offset: int
    # the indices tensor frame offset
    indices_offset: int
    # the updates tensor frame offset
    updates_offset: int
    # the scatter dimension numbers
    dimensions: destack._generated.program.vm.table.TensorScatterId
    # the source tensor layout
    source_layout: destack._generated.program.vm.table.TensorLayoutId
    # the indices tensor layout
    indices_layout: destack._generated.program.vm.table.TensorLayoutId
    # the updates tensor layout
    updates_layout: destack._generated.program.vm.table.TensorLayoutId
    # the destination tensor layout
    dest_layout: destack._generated.program.vm.table.TensorLayoutId
    # the tensor element scalar layout
    element_layout: destack._generated.program.vm.value.ScalarLayout
    # the scatter kernel
    mode: destack._generated.mir.tree.tensor.TensorScatterMode

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorScatter: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TensorScatter: ...

def encode_tensor_scatter(writer: BinaryWriter, value: TensorScatter) -> None: ...
def decode_tensor_scatter(reader: BinaryReader) -> TensorScatter: ...
def to_json_tensor_scatter(value: TensorScatter) -> Json: ...
def from_json_tensor_scatter(value: Json) -> TensorScatter: ...

@dataclass(frozen=True, slots=True)
class TensorSelect:
    """Select tensor elements based on a boolean mask."""

    # the destination tensor frame offset
    dest_offset: int
    # the mask tensor frame offset
    mask_offset: int
    # the true branch tensor frame offset
    then_offset: int
    # the false branch tensor frame offset
    else_offset: int
    # the mask tensor layout
    mask_layout: destack._generated.program.vm.table.TensorLayoutId
    # the true branch tensor layout
    then_layout: destack._generated.program.vm.table.TensorLayoutId
    # the false branch tensor layout
    else_layout: destack._generated.program.vm.table.TensorLayoutId
    # the destination tensor layout
    dest_layout: destack._generated.program.vm.table.TensorLayoutId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorSelect: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TensorSelect: ...

def encode_tensor_select(writer: BinaryWriter, value: TensorSelect) -> None: ...
def decode_tensor_select(reader: BinaryReader) -> TensorSelect: ...
def to_json_tensor_select(value: TensorSelect) -> Json: ...
def from_json_tensor_select(value: Json) -> TensorSelect: ...

@dataclass(frozen=True, slots=True)
class TensorConvert:
    """Convert a tensor element type."""

    # the destination tensor frame offset
    dest_offset: int
    # the source tensor frame offset
    tensor_offset: int
    # the source tensor layout
    source_layout: destack._generated.program.vm.table.TensorLayoutId
    # the destination tensor layout
    dest_layout: destack._generated.program.vm.table.TensorLayoutId
    # the source tensor scalar layout
    source_scalar: destack._generated.program.vm.value.ScalarLayout
    # the destination tensor scalar layout
    dest_scalar: destack._generated.program.vm.value.ScalarLayout
    # the conversion kernel
    mode: destack._generated.mir.tree.tensor.TensorConvertMode

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorConvert: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TensorConvert: ...

def encode_tensor_convert(writer: BinaryWriter, value: TensorConvert) -> None: ...
def decode_tensor_convert(reader: BinaryReader) -> TensorConvert: ...
def to_json_tensor_convert(value: TensorConvert) -> Json: ...
def from_json_tensor_convert(value: Json) -> TensorConvert: ...

@dataclass(frozen=True, slots=True)
class TensorView:
    """Create a view into a tensor reference."""

    # the destination tensor view
    dest_offset: int
    # the source tensor view
    view_offset: int
    # the offset, size, and stride values
    arguments: destack._generated.program.vm.table.U32RangeId
    # the number of offset values
    offsets_count: int
    # the number of size values
    sizes_count: int
    # the number of stride values
    strides_count: int
    # the source tensor view layout
    source_layout: destack._generated.program.vm.table.TensorLayoutId
    # the destination tensor view layout
    dest_layout: destack._generated.program.vm.table.TensorLayoutId
    # the tensor element projection
    element: destack._generated.program.vm.table.ProjectionId
    # the tensor view backing memory
    address: destack._generated.program.vm.tensor.TensorAddress

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorView: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TensorView: ...

def encode_tensor_view(writer: BinaryWriter, value: TensorView) -> None: ...
def decode_tensor_view(reader: BinaryReader) -> TensorView: ...
def to_json_tensor_view(value: TensorView) -> Json: ...
def from_json_tensor_view(value: Json) -> TensorView: ...

@dataclass(frozen=True, slots=True)
class Intrinsic:
    """Intrinsic call."""

    # the intrinsic kernel
    kernel: destack._generated.mir.tree.intrinsic.Intrinsic
    # the destination shape
    dest: IntrinsicDest
    # the pooled argument range
    arguments: destack._generated.program.vm.range.ArgumentRange
    # the lowered shape for each argument
    layouts: tuple[
        destack._generated.program.vm.value.ValueShape,
        destack._generated.program.vm.value.ValueShape,
        destack._generated.program.vm.value.ValueShape,
        destack._generated.program.vm.value.ValueShape,
        destack._generated.program.vm.value.ValueShape,
        destack._generated.program.vm.value.ValueShape,
        destack._generated.program.vm.value.ValueShape,
        destack._generated.program.vm.value.ValueShape,
        destack._generated.program.vm.value.ValueShape,
        destack._generated.program.vm.value.ValueShape,
        destack._generated.program.vm.value.ValueShape,
        destack._generated.program.vm.value.ValueShape,
        destack._generated.program.vm.value.ValueShape,
        destack._generated.program.vm.value.ValueShape,
        destack._generated.program.vm.value.ValueShape,
        destack._generated.program.vm.value.ValueShape,
    ]
    # the argument count
    layout_count: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Intrinsic: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Intrinsic: ...

def encode_intrinsic(writer: BinaryWriter, value: Intrinsic) -> None: ...
def decode_intrinsic(reader: BinaryReader) -> Intrinsic: ...
def to_json_intrinsic(value: Intrinsic) -> Json: ...
def from_json_intrinsic(value: Json) -> Intrinsic: ...

@dataclass(frozen=True, slots=True)
class IntrinsicDestNone:
    """No destination."""

    kind: typing.Literal["none"] = "none"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class IntrinsicDestCell:
    """Cell destination frame offset."""

    cell: int
    kind: typing.Literal["cell"] = "cell"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class IntrinsicDestFrame:
    """Frame destination value."""

    frame: destack._generated.mir.tree.value.Value
    kind: typing.Literal["frame"] = "frame"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Intrinsic destination."""
IntrinsicDest: typing.TypeAlias = (
    IntrinsicDestNone | IntrinsicDestCell | IntrinsicDestFrame
)

def encode_intrinsic_dest(writer: BinaryWriter, value: IntrinsicDest) -> None: ...
def decode_intrinsic_dest(reader: BinaryReader) -> IntrinsicDest: ...
def to_json_intrinsic_dest(value: IntrinsicDest) -> Json: ...
def from_json_intrinsic_dest(value: Json) -> IntrinsicDest: ...

@dataclass(frozen=True, slots=True)
class TailCall:
    """Tail call to a function."""

    # the callee function index
    function: int
    # the resolved call target
    target: destack._generated.program.vm.function.CallTarget
    # the argument frame moves
    moves: destack._generated.program.vm.range.MoveRange

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TailCall: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TailCall: ...

def encode_tail_call(writer: BinaryWriter, value: TailCall) -> None: ...
def decode_tail_call(reader: BinaryReader) -> TailCall: ...
def to_json_tail_call(value: TailCall) -> Json: ...
def from_json_tail_call(value: Json) -> TailCall: ...

@dataclass(frozen=True, slots=True)
class TailCallVirtual:
    """Class tail call."""

    # the receiver cell offset
    receiver_offset: int
    # the dispatch table field projection
    table_field: destack._generated.program.vm.table.ProjectionId
    # the dispatch table slot
    slot: int
    # the pooled argument range
    arguments: destack._generated.program.vm.range.ArgumentRange

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TailCallVirtual: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TailCallVirtual: ...

def encode_tail_call_virtual(writer: BinaryWriter, value: TailCallVirtual) -> None: ...
def decode_tail_call_virtual(reader: BinaryReader) -> TailCallVirtual: ...
def to_json_tail_call_virtual(value: TailCallVirtual) -> Json: ...
def from_json_tail_call_virtual(value: Json) -> TailCallVirtual: ...

@dataclass(frozen=True, slots=True)
class TailCallDynamic:
    """Dynamic tail call."""

    # the receiver cell offset
    receiver_offset: int
    # the dynamic table field projection
    table_field: destack._generated.program.vm.table.ProjectionId
    # the dynamic table slot
    slot: int
    # the pooled argument range
    arguments: destack._generated.program.vm.range.ArgumentRange

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TailCallDynamic: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TailCallDynamic: ...

def encode_tail_call_dynamic(writer: BinaryWriter, value: TailCallDynamic) -> None: ...
def decode_tail_call_dynamic(reader: BinaryReader) -> TailCallDynamic: ...
def to_json_tail_call_dynamic(value: TailCallDynamic) -> Json: ...
def from_json_tail_call_dynamic(value: Json) -> TailCallDynamic: ...

@dataclass(frozen=True, slots=True)
class IndirectTailCall:
    """Indirect tail call."""

    # the callee cell offset
    callee_offset: int
    # the expected function signature
    signature: destack._generated.mir.tree.node.LocalNodeId
    # the pooled argument range
    arguments: destack._generated.program.vm.range.ArgumentRange

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> IndirectTailCall: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> IndirectTailCall: ...

def encode_indirect_tail_call(
    writer: BinaryWriter, value: IndirectTailCall
) -> None: ...
def decode_indirect_tail_call(reader: BinaryReader) -> IndirectTailCall: ...
def to_json_indirect_tail_call(value: IndirectTailCall) -> Json: ...
def from_json_indirect_tail_call(value: Json) -> IndirectTailCall: ...

__all__ = [
    "AggregateSelect",
    "encode_aggregate_select",
    "decode_aggregate_select",
    "to_json_aggregate_select",
    "from_json_aggregate_select",
    "AtomicCompareExchange",
    "encode_atomic_compare_exchange",
    "decode_atomic_compare_exchange",
    "to_json_atomic_compare_exchange",
    "from_json_atomic_compare_exchange",
    "VectorSplat",
    "encode_vector_splat",
    "decode_vector_splat",
    "to_json_vector_splat",
    "from_json_vector_splat",
    "VectorExtract",
    "encode_vector_extract",
    "decode_vector_extract",
    "to_json_vector_extract",
    "from_json_vector_extract",
    "VectorBinary",
    "encode_vector_binary",
    "decode_vector_binary",
    "to_json_vector_binary",
    "from_json_vector_binary",
    "ElementBinaryKernel",
    "encode_element_binary_kernel",
    "decode_element_binary_kernel",
    "to_json_element_binary_kernel",
    "from_json_element_binary_kernel",
    "VectorUnary",
    "encode_vector_unary",
    "decode_vector_unary",
    "to_json_vector_unary",
    "from_json_vector_unary",
    "ElementUnaryKernel",
    "encode_element_unary_kernel",
    "decode_element_unary_kernel",
    "to_json_element_unary_kernel",
    "from_json_element_unary_kernel",
    "VectorInsert",
    "encode_vector_insert",
    "decode_vector_insert",
    "to_json_vector_insert",
    "from_json_vector_insert",
    "VectorShuffle",
    "encode_vector_shuffle",
    "decode_vector_shuffle",
    "to_json_vector_shuffle",
    "from_json_vector_shuffle",
    "VectorSelect",
    "encode_vector_select",
    "decode_vector_select",
    "to_json_vector_select",
    "from_json_vector_select",
    "VectorReduce",
    "encode_vector_reduce",
    "decode_vector_reduce",
    "to_json_vector_reduce",
    "from_json_vector_reduce",
    "VectorConvert",
    "encode_vector_convert",
    "decode_vector_convert",
    "to_json_vector_convert",
    "from_json_vector_convert",
    "FunctionBind",
    "encode_function_bind",
    "decode_function_bind",
    "to_json_function_bind",
    "from_json_function_bind",
    "Call",
    "encode_call",
    "decode_call",
    "to_json_call",
    "from_json_call",
    "CallBranch",
    "encode_call_branch",
    "decode_call_branch",
    "to_json_call_branch",
    "from_json_call_branch",
    "CallVirtual",
    "encode_call_virtual",
    "decode_call_virtual",
    "to_json_call_virtual",
    "from_json_call_virtual",
    "CallVirtualBranch",
    "encode_call_virtual_branch",
    "decode_call_virtual_branch",
    "to_json_call_virtual_branch",
    "from_json_call_virtual_branch",
    "CallDynamic",
    "encode_call_dynamic",
    "decode_call_dynamic",
    "to_json_call_dynamic",
    "from_json_call_dynamic",
    "CallDynamicBranch",
    "encode_call_dynamic_branch",
    "decode_call_dynamic_branch",
    "to_json_call_dynamic_branch",
    "from_json_call_dynamic_branch",
    "IndirectCall",
    "encode_indirect_call",
    "decode_indirect_call",
    "to_json_indirect_call",
    "from_json_indirect_call",
    "IndirectCallBranch",
    "encode_indirect_call_branch",
    "decode_indirect_call_branch",
    "to_json_indirect_call_branch",
    "from_json_indirect_call_branch",
    "TensorLoad",
    "encode_tensor_load",
    "decode_tensor_load",
    "to_json_tensor_load",
    "from_json_tensor_load",
    "TensorExtract",
    "encode_tensor_extract",
    "decode_tensor_extract",
    "to_json_tensor_extract",
    "from_json_tensor_extract",
    "TensorBinary",
    "encode_tensor_binary",
    "decode_tensor_binary",
    "to_json_tensor_binary",
    "from_json_tensor_binary",
    "TensorContiguousBinary",
    "encode_tensor_contiguous_binary",
    "decode_tensor_contiguous_binary",
    "to_json_tensor_contiguous_binary",
    "from_json_tensor_contiguous_binary",
    "TensorUnary",
    "encode_tensor_unary",
    "decode_tensor_unary",
    "to_json_tensor_unary",
    "from_json_tensor_unary",
    "TensorContiguousUnary",
    "encode_tensor_contiguous_unary",
    "decode_tensor_contiguous_unary",
    "to_json_tensor_contiguous_unary",
    "from_json_tensor_contiguous_unary",
    "TensorStore",
    "encode_tensor_store",
    "decode_tensor_store",
    "to_json_tensor_store",
    "from_json_tensor_store",
    "TensorFill",
    "encode_tensor_fill",
    "decode_tensor_fill",
    "to_json_tensor_fill",
    "from_json_tensor_fill",
    "TensorCopy",
    "encode_tensor_copy",
    "decode_tensor_copy",
    "to_json_tensor_copy",
    "from_json_tensor_copy",
    "TensorViewCast",
    "encode_tensor_view_cast",
    "decode_tensor_view_cast",
    "to_json_tensor_view_cast",
    "from_json_tensor_view_cast",
    "TensorReshape",
    "encode_tensor_reshape",
    "decode_tensor_reshape",
    "to_json_tensor_reshape",
    "from_json_tensor_reshape",
    "TensorBroadcast",
    "encode_tensor_broadcast",
    "decode_tensor_broadcast",
    "to_json_tensor_broadcast",
    "from_json_tensor_broadcast",
    "TensorTranspose",
    "encode_tensor_transpose",
    "decode_tensor_transpose",
    "to_json_tensor_transpose",
    "from_json_tensor_transpose",
    "TensorSlice",
    "encode_tensor_slice",
    "decode_tensor_slice",
    "to_json_tensor_slice",
    "from_json_tensor_slice",
    "TensorPad",
    "encode_tensor_pad",
    "decode_tensor_pad",
    "to_json_tensor_pad",
    "from_json_tensor_pad",
    "TensorConcat",
    "encode_tensor_concat",
    "decode_tensor_concat",
    "to_json_tensor_concat",
    "from_json_tensor_concat",
    "TensorReduce",
    "encode_tensor_reduce",
    "decode_tensor_reduce",
    "to_json_tensor_reduce",
    "from_json_tensor_reduce",
    "TensorIndexReduce",
    "encode_tensor_index_reduce",
    "decode_tensor_index_reduce",
    "to_json_tensor_index_reduce",
    "from_json_tensor_index_reduce",
    "TensorDot",
    "encode_tensor_dot",
    "decode_tensor_dot",
    "to_json_tensor_dot",
    "from_json_tensor_dot",
    "TensorConvolution",
    "encode_tensor_convolution",
    "decode_tensor_convolution",
    "to_json_tensor_convolution",
    "from_json_tensor_convolution",
    "TensorGather",
    "encode_tensor_gather",
    "decode_tensor_gather",
    "to_json_tensor_gather",
    "from_json_tensor_gather",
    "TensorScatter",
    "encode_tensor_scatter",
    "decode_tensor_scatter",
    "to_json_tensor_scatter",
    "from_json_tensor_scatter",
    "TensorSelect",
    "encode_tensor_select",
    "decode_tensor_select",
    "to_json_tensor_select",
    "from_json_tensor_select",
    "TensorConvert",
    "encode_tensor_convert",
    "decode_tensor_convert",
    "to_json_tensor_convert",
    "from_json_tensor_convert",
    "TensorView",
    "encode_tensor_view",
    "decode_tensor_view",
    "to_json_tensor_view",
    "from_json_tensor_view",
    "Intrinsic",
    "encode_intrinsic",
    "decode_intrinsic",
    "to_json_intrinsic",
    "from_json_intrinsic",
    "IntrinsicDest",
    "encode_intrinsic_dest",
    "decode_intrinsic_dest",
    "to_json_intrinsic_dest",
    "from_json_intrinsic_dest",
    "IntrinsicDestNone",
    "IntrinsicDestCell",
    "IntrinsicDestFrame",
    "TailCall",
    "encode_tail_call",
    "decode_tail_call",
    "to_json_tail_call",
    "from_json_tail_call",
    "TailCallVirtual",
    "encode_tail_call_virtual",
    "decode_tail_call_virtual",
    "to_json_tail_call_virtual",
    "from_json_tail_call_virtual",
    "TailCallDynamic",
    "encode_tail_call_dynamic",
    "decode_tail_call_dynamic",
    "to_json_tail_call_dynamic",
    "from_json_tail_call_dynamic",
    "IndirectTailCall",
    "encode_indirect_tail_call",
    "decode_indirect_tail_call",
    "to_json_indirect_tail_call",
    "from_json_indirect_tail_call",
]
