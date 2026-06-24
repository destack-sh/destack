# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.symbol.key
import destack._generated.dir.symbol.symbol
import destack._generated.dir.tree.literal
import destack._generated.dir.tree.node
import destack._generated.dir.tree.operator
import destack._generated.dir.type.type

@dataclass(frozen=True, slots=True)
class NameResolution:
    """Target selected by lexical or path lookup."""

    # the selected symbols in declaration order
    symbols: Sequence[destack._generated.dir.symbol.symbol.GlobalSymbolId]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> NameResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> NameResolution: ...

def encode_name_resolution(writer: BinaryWriter, value: NameResolution) -> None: ...
def decode_name_resolution(reader: BinaryReader) -> NameResolution: ...
def to_json_name_resolution(value: NameResolution) -> Json: ...
def from_json_name_resolution(value: Json) -> NameResolution: ...

@dataclass(frozen=True, slots=True)
class LabelResolutionSymbol:
    """An explicit label target."""

    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    kind: typing.Literal["symbol"] = "symbol"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LabelResolutionLoop:
    """The nearest enclosing loop target."""

    kind: typing.Literal["loop"] = "loop"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LabelResolutionFunction:
    """The enclosing function target."""

    kind: typing.Literal["function"] = "function"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Target selected by a labeled transfer."""
LabelResolution: typing.TypeAlias = (
    LabelResolutionSymbol | LabelResolutionLoop | LabelResolutionFunction
)

def encode_label_resolution(writer: BinaryWriter, value: LabelResolution) -> None: ...
def decode_label_resolution(reader: BinaryReader) -> LabelResolution: ...
def to_json_label_resolution(value: LabelResolution) -> Json: ...
def from_json_label_resolution(value: Json) -> LabelResolution: ...

@dataclass(frozen=True, slots=True)
class ReceiverResolution:
    """Receiver selected by contextual lookup, such as `this` or `super`."""

    # the receiver syntax kind
    kind: ReceiverKind
    # the declaration that introduces the receiver
    owner: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the receiver type after inference
    ty: destack._generated.dir.type.type.GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ReceiverResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ReceiverResolution: ...

def encode_receiver_resolution(
    writer: BinaryWriter, value: ReceiverResolution
) -> None: ...
def decode_receiver_resolution(reader: BinaryReader) -> ReceiverResolution: ...
def to_json_receiver_resolution(value: ReceiverResolution) -> Json: ...
def from_json_receiver_resolution(value: Json) -> ReceiverResolution: ...

"""Receiver syntax resolved by contextual lookup."""
ReceiverKind: typing.TypeAlias = typing.Literal["this"] | typing.Literal["super"]

def encode_receiver_kind(writer: BinaryWriter, value: ReceiverKind) -> None: ...
def decode_receiver_kind(reader: BinaryReader) -> ReceiverKind: ...
def to_json_receiver_kind(value: ReceiverKind) -> Json: ...
def from_json_receiver_kind(value: Json) -> ReceiverKind: ...

@dataclass(frozen=True, slots=True)
class MemberResolution:
    """Receiver member selected at a usage site."""

    # the receiver type after inference
    receiver: destack._generated.dir.type.type.GlobalTypeId
    # the selected member target
    target: MemberTarget

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> MemberResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> MemberResolution: ...

def encode_member_resolution(writer: BinaryWriter, value: MemberResolution) -> None: ...
def decode_member_resolution(reader: BinaryReader) -> MemberResolution: ...
def to_json_member_resolution(value: MemberResolution) -> Json: ...
def from_json_member_resolution(value: Json) -> MemberResolution: ...

@dataclass(frozen=True, slots=True)
class MemberTargetField:
    """Structural field selected from a shape type."""

    field: destack._generated.dir.symbol.key.StaticKey
    kind: typing.Literal["field"] = "field"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class MemberTargetElement:
    """Structural element selected from a tuple type."""

    element: int
    kind: typing.Literal["element"] = "element"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class MemberTargetIndex:
    """Structural index signature selected from a shape type."""

    index: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["index"] = "index"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class MemberTargetSymbol:
    """Exactly one symbol-backed member selected at compile time."""

    symbol: MemberCandidate
    kind: typing.Literal["symbol"] = "symbol"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class MemberTargetExistential:
    """Existential symbol-backed candidates deferred to call selection."""

    existential: Sequence[MemberCandidate]
    kind: typing.Literal["existential"] = "existential"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class MemberTargetUniversal:
    """Universal symbol-backed candidates deferred to call selection."""

    universal: Sequence[MemberCandidate]
    kind: typing.Literal["universal"] = "universal"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Member target selected at a usage site."""
MemberTarget: typing.TypeAlias = (
    MemberTargetField
    | MemberTargetElement
    | MemberTargetIndex
    | MemberTargetSymbol
    | MemberTargetExistential
    | MemberTargetUniversal
)

def encode_member_target(writer: BinaryWriter, value: MemberTarget) -> None: ...
def decode_member_target(reader: BinaryReader) -> MemberTarget: ...
def to_json_member_target(value: MemberTarget) -> Json: ...
def from_json_member_target(value: Json) -> MemberTarget: ...

@dataclass(frozen=True, slots=True)
class MemberCandidate:
    """One member candidate after receiver lookup."""

    # the receiver type that selects this candidate
    receiver: destack._generated.dir.type.type.GlobalTypeId
    # the selected member symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the member type applied to the matched receiver
    ty: destack._generated.dir.type.type.GlobalTypeId
    # the generic arguments of the member symbol, empty when not statically applied
    arguments: Sequence[destack._generated.dir.type.type.GlobalTypeId]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> MemberCandidate: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> MemberCandidate: ...

def encode_member_candidate(writer: BinaryWriter, value: MemberCandidate) -> None: ...
def decode_member_candidate(reader: BinaryReader) -> MemberCandidate: ...
def to_json_member_candidate(value: MemberCandidate) -> Json: ...
def from_json_member_candidate(value: Json) -> MemberCandidate: ...

@dataclass(frozen=True, slots=True)
class CallResolution:
    """Callable selected at a call site."""

    # the selected callable target
    target: CallTarget
    # the dynamic parameter types after static substitutions
    parameters: Sequence[destack._generated.dir.type.type.GlobalTypeId]
    # the return type after static substitutions
    return_type: destack._generated.dir.type.type.GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CallResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CallResolution: ...

def encode_call_resolution(writer: BinaryWriter, value: CallResolution) -> None: ...
def decode_call_resolution(reader: BinaryReader) -> CallResolution: ...
def to_json_call_resolution(value: CallResolution) -> Json: ...
def from_json_call_resolution(value: Json) -> CallResolution: ...

@dataclass(frozen=True, slots=True)
class CallTargetBuiltin:
    """Compiler builtin selected at a usage site."""

    builtin: BuiltinCall
    kind: typing.Literal["builtin"] = "builtin"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CallTargetExpression:
    """Callable expression without a declaration symbol."""

    # the generic arguments of the callable value, empty when not statically applied
    arguments: Sequence[destack._generated.dir.type.type.GlobalTypeId]
    kind: typing.Literal["expression"] = "expression"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CallTargetSymbol:
    """Exactly one symbol-backed callable selected at compile time."""

    symbol: CallCandidate
    kind: typing.Literal["symbol"] = "symbol"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CallTargetUniversal:
    """Universal symbol-backed callables selected at compile time."""

    universal: Sequence[CallCandidate]
    kind: typing.Literal["universal"] = "universal"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Callable target selected at a call site."""
CallTarget: typing.TypeAlias = (
    CallTargetBuiltin | CallTargetExpression | CallTargetSymbol | CallTargetUniversal
)

def encode_call_target(writer: BinaryWriter, value: CallTarget) -> None: ...
def decode_call_target(reader: BinaryReader) -> CallTarget: ...
def to_json_call_target(value: CallTarget) -> Json: ...
def from_json_call_target(value: Json) -> CallTarget: ...

@dataclass(frozen=True, slots=True)
class BuiltinCallUnaryOperator:
    """Builtin unary operator behavior."""

    # the source operator
    operator: destack._generated.dir.tree.operator.UnaryOperator
    kind: typing.Literal["unaryOperator"] = "unaryOperator"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class BuiltinCallBinaryOperator:
    """Builtin binary operator behavior."""

    # the source operator
    operator: destack._generated.dir.tree.operator.BinaryOperator
    kind: typing.Literal["binaryOperator"] = "binaryOperator"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Compiler builtin callable selected at a usage site."""
BuiltinCall: typing.TypeAlias = BuiltinCallUnaryOperator | BuiltinCallBinaryOperator

def encode_builtin_call(writer: BinaryWriter, value: BuiltinCall) -> None: ...
def decode_builtin_call(reader: BinaryReader) -> BuiltinCall: ...
def to_json_builtin_call(value: BuiltinCall) -> Json: ...
def from_json_builtin_call(value: Json) -> BuiltinCall: ...

@dataclass(frozen=True, slots=True)
class CallCandidate:
    """One callable candidate after overload selection."""

    # the receiver type that selects this candidate
    receiver: destack._generated.dir.type.type.GlobalTypeId | None
    # the selected callable symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the generic arguments of the callable symbol, empty when not statically applied
    arguments: Sequence[destack._generated.dir.type.type.GlobalTypeId]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CallCandidate: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CallCandidate: ...

def encode_call_candidate(writer: BinaryWriter, value: CallCandidate) -> None: ...
def decode_call_candidate(reader: BinaryReader) -> CallCandidate: ...
def to_json_call_candidate(value: CallCandidate) -> Json: ...
def from_json_call_candidate(value: Json) -> CallCandidate: ...

@dataclass(frozen=True, slots=True)
class ReadWriteResolution:
    """Paired accessor calls for one place read and written together."""

    # the read accessor call
    read: CallResolution
    # the write accessor call
    write: CallResolution

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ReadWriteResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ReadWriteResolution: ...

def encode_read_write_resolution(
    writer: BinaryWriter, value: ReadWriteResolution
) -> None: ...
def decode_read_write_resolution(reader: BinaryReader) -> ReadWriteResolution: ...
def to_json_read_write_resolution(value: ReadWriteResolution) -> Json: ...
def from_json_read_write_resolution(value: Json) -> ReadWriteResolution: ...

@dataclass(frozen=True, slots=True)
class ConstructResolution:
    """Construct expression selected at a usage site."""

    # the selected construct target
    target: ConstructTarget
    # the dynamic parameter types after static substitutions
    parameters: Sequence[destack._generated.dir.type.type.GlobalTypeId]
    # the return type after static substitutions
    return_type: destack._generated.dir.type.type.GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ConstructResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ConstructResolution: ...

def encode_construct_resolution(
    writer: BinaryWriter, value: ConstructResolution
) -> None: ...
def decode_construct_resolution(reader: BinaryReader) -> ConstructResolution: ...
def to_json_construct_resolution(value: ConstructResolution) -> Json: ...
def from_json_construct_resolution(value: Json) -> ConstructResolution: ...

@dataclass(frozen=True, slots=True)
class ConstructTargetClass:
    """Class construction selected at compile time."""

    class_: ClassConstructCandidate
    kind: typing.Literal["class"] = "class"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ConstructTargetNewtype:
    """Newtype wrapper constructor selected at compile time."""

    newtype: NewtypeConstructCandidate
    kind: typing.Literal["newtype"] = "newtype"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Construct target selected at a usage site."""
ConstructTarget: typing.TypeAlias = ConstructTargetClass | ConstructTargetNewtype

def encode_construct_target(writer: BinaryWriter, value: ConstructTarget) -> None: ...
def decode_construct_target(reader: BinaryReader) -> ConstructTarget: ...
def to_json_construct_target(value: ConstructTarget) -> Json: ...
def from_json_construct_target(value: Json) -> ConstructTarget: ...

@dataclass(frozen=True, slots=True)
class ClassConstructCandidate:
    """One class construction candidate after overload selection."""

    # the selected class symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the selected explicit constructor symbol, when declared
    constructor: destack._generated.dir.symbol.symbol.GlobalSymbolId | None
    # the generic arguments of the class symbol, empty when not statically applied
    arguments: Sequence[destack._generated.dir.type.type.GlobalTypeId]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ClassConstructCandidate: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ClassConstructCandidate: ...

def encode_class_construct_candidate(
    writer: BinaryWriter, value: ClassConstructCandidate
) -> None: ...
def decode_class_construct_candidate(
    reader: BinaryReader,
) -> ClassConstructCandidate: ...
def to_json_class_construct_candidate(value: ClassConstructCandidate) -> Json: ...
def from_json_class_construct_candidate(value: Json) -> ClassConstructCandidate: ...

@dataclass(frozen=True, slots=True)
class NewtypeConstructCandidate:
    """One newtype construction candidate after overload selection."""

    # the selected newtype symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the generic arguments of the newtype symbol, empty when not statically applied
    arguments: Sequence[destack._generated.dir.type.type.GlobalTypeId]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> NewtypeConstructCandidate: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> NewtypeConstructCandidate: ...

def encode_newtype_construct_candidate(
    writer: BinaryWriter, value: NewtypeConstructCandidate
) -> None: ...
def decode_newtype_construct_candidate(
    reader: BinaryReader,
) -> NewtypeConstructCandidate: ...
def to_json_newtype_construct_candidate(value: NewtypeConstructCandidate) -> Json: ...
def from_json_newtype_construct_candidate(value: Json) -> NewtypeConstructCandidate: ...

@dataclass(frozen=True, slots=True)
class PatternResolutionWildcard:
    """Pattern that accepts the input without binding, like `_`."""

    kind: typing.Literal["wildcard"] = "wildcard"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternResolutionBinding:
    """Pattern that binds a symbol, like `value`."""

    binding: PatternBindingResolution
    kind: typing.Literal["binding"] = "binding"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternResolutionLiteral:
    """Pattern that accepts one static literal value, like `"ok"` or `0`."""

    literal: PatternLiteralResolution
    kind: typing.Literal["literal"] = "literal"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternResolutionRange:
    """Pattern that accepts one scalar interval, like `0..10` or `..=255`."""

    range: PatternRangeResolution
    kind: typing.Literal["range"] = "range"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternResolutionTuple:
    """Pattern that destructures a tuple-shaped input, like `(x, y)`."""

    tuple: PatternTupleResolution
    kind: typing.Literal["tuple"] = "tuple"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternResolutionSequence:
    """Pattern that destructures an ordered collection, like `[head, ...tail]`."""

    sequence: PatternSequenceResolution
    kind: typing.Literal["sequence"] = "sequence"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternResolutionShape:
    """Pattern that destructures a structural input, like `{ kind: "ok", value }`."""

    shape: PatternShapeResolution
    kind: typing.Literal["shape"] = "shape"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternResolutionNominal:
    """Pattern that destructures a symbol-backed nominal input, like `Point { x, y }`."""

    nominal: PatternNominalResolution
    kind: typing.Literal["nominal"] = "nominal"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternResolutionNewtype:
    """Pattern that unwraps a symbol-backed newtype input, like `UserId(value)`."""

    newtype: PatternNewtypeResolution
    kind: typing.Literal["newtype"] = "newtype"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternResolutionVariant:
    """Pattern that selects a symbol-backed variant input, like `State.Ready`."""

    variant: PatternVariantResolution
    kind: typing.Literal["variant"] = "variant"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternResolutionUnion:
    """Pattern that accepts one of several alternatives, like `0 | 1 | 2`."""

    union: PatternUnionResolution
    kind: typing.Literal["union"] = "union"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternResolutionBorrow:
    """Pattern that borrows the input before matching, like `&readonly value`."""

    borrow: PatternBorrowResolution
    kind: typing.Literal["borrow"] = "borrow"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternResolutionMove:
    """Pattern that moves the input before matching, like `^value`."""

    move_file: PatternMoveResolution
    kind: typing.Literal["move"] = "move"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternResolutionDereference:
    """Pattern that dereferences the input before matching, like `*Point { x, y }`."""

    dereference: PatternDereferenceResolution
    kind: typing.Literal["dereference"] = "dereference"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Pattern meaning selected during checking."""
PatternResolution: typing.TypeAlias = (
    PatternResolutionWildcard
    | PatternResolutionBinding
    | PatternResolutionLiteral
    | PatternResolutionRange
    | PatternResolutionTuple
    | PatternResolutionSequence
    | PatternResolutionShape
    | PatternResolutionNominal
    | PatternResolutionNewtype
    | PatternResolutionVariant
    | PatternResolutionUnion
    | PatternResolutionBorrow
    | PatternResolutionMove
    | PatternResolutionDereference
)

def encode_pattern_resolution(
    writer: BinaryWriter, value: PatternResolution
) -> None: ...
def decode_pattern_resolution(reader: BinaryReader) -> PatternResolution: ...
def to_json_pattern_resolution(value: PatternResolution) -> Json: ...
def from_json_pattern_resolution(value: Json) -> PatternResolution: ...

@dataclass(frozen=True, slots=True)
class PatternBindingResolution:
    """Symbol binding introduced by one pattern."""

    # the bound symbol, when the binding has a user-visible name
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId | None
    # the nested pattern matched after binding
    pattern: destack._generated.dir.tree.node.GlobalNodeIdAny | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternBindingResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PatternBindingResolution: ...

def encode_pattern_binding_resolution(
    writer: BinaryWriter, value: PatternBindingResolution
) -> None: ...
def decode_pattern_binding_resolution(
    reader: BinaryReader,
) -> PatternBindingResolution: ...
def to_json_pattern_binding_resolution(value: PatternBindingResolution) -> Json: ...
def from_json_pattern_binding_resolution(value: Json) -> PatternBindingResolution: ...

@dataclass(frozen=True, slots=True)
class PatternLiteralResolution:
    """Static literal selected by one pattern."""

    # the committed literal value
    value: destack._generated.dir.tree.literal.ScalarLiteral

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternLiteralResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PatternLiteralResolution: ...

def encode_pattern_literal_resolution(
    writer: BinaryWriter, value: PatternLiteralResolution
) -> None: ...
def decode_pattern_literal_resolution(
    reader: BinaryReader,
) -> PatternLiteralResolution: ...
def to_json_pattern_literal_resolution(value: PatternLiteralResolution) -> Json: ...
def from_json_pattern_literal_resolution(value: Json) -> PatternLiteralResolution: ...

@dataclass(frozen=True, slots=True)
class PatternRangeResolution:
    """Scalar range selected by one pattern."""

    # the scalar domain constrained by the range
    domain: destack._generated.dir.type.type.GlobalTypeId
    # the optional committed lower bound
    start: destack._generated.dir.tree.literal.ScalarLiteral | None
    # the optional committed upper bound
    end: destack._generated.dir.tree.literal.ScalarLiteral | None
    # whether the upper bound is inclusive
    end_bound: destack._generated.dir.tree.operator.RangeEnd

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternRangeResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PatternRangeResolution: ...

def encode_pattern_range_resolution(
    writer: BinaryWriter, value: PatternRangeResolution
) -> None: ...
def decode_pattern_range_resolution(reader: BinaryReader) -> PatternRangeResolution: ...
def to_json_pattern_range_resolution(value: PatternRangeResolution) -> Json: ...
def from_json_pattern_range_resolution(value: Json) -> PatternRangeResolution: ...

@dataclass(frozen=True, slots=True)
class PatternTupleResolution:
    """Tuple fields selected by one pattern."""

    # the tuple field mapping in source order
    fields: Sequence[PatternFieldResolution]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternTupleResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PatternTupleResolution: ...

def encode_pattern_tuple_resolution(
    writer: BinaryWriter, value: PatternTupleResolution
) -> None: ...
def decode_pattern_tuple_resolution(reader: BinaryReader) -> PatternTupleResolution: ...
def to_json_pattern_tuple_resolution(value: PatternTupleResolution) -> Json: ...
def from_json_pattern_tuple_resolution(value: Json) -> PatternTupleResolution: ...

@dataclass(frozen=True, slots=True)
class PatternFieldResolution:
    """One destructured pattern field."""

    # the source node that introduces the field
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the selected field target
    target: PatternFieldTarget
    # the nested pattern matched for the field
    pattern: destack._generated.dir.tree.node.GlobalNodeIdAny | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternFieldResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PatternFieldResolution: ...

def encode_pattern_field_resolution(
    writer: BinaryWriter, value: PatternFieldResolution
) -> None: ...
def decode_pattern_field_resolution(reader: BinaryReader) -> PatternFieldResolution: ...
def to_json_pattern_field_resolution(value: PatternFieldResolution) -> Json: ...
def from_json_pattern_field_resolution(value: Json) -> PatternFieldResolution: ...

@dataclass(frozen=True, slots=True)
class PatternFieldTargetKey:
    """Named or symbolic field target."""

    key: destack._generated.dir.symbol.key.StaticKey
    kind: typing.Literal["key"] = "key"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternFieldTargetIndex:
    """Positional field target."""

    index: int
    kind: typing.Literal["index"] = "index"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Field target selected by one destructuring pattern."""
PatternFieldTarget: typing.TypeAlias = PatternFieldTargetKey | PatternFieldTargetIndex

def encode_pattern_field_target(
    writer: BinaryWriter, value: PatternFieldTarget
) -> None: ...
def decode_pattern_field_target(reader: BinaryReader) -> PatternFieldTarget: ...
def to_json_pattern_field_target(value: PatternFieldTarget) -> Json: ...
def from_json_pattern_field_target(value: Json) -> PatternFieldTarget: ...

@dataclass(frozen=True, slots=True)
class PatternSequenceResolutionArray:
    """Dynamically sized array pattern, like `[head, ...tail]` over `T[]`."""

    # the fixed prefix and suffix fields
    fields: Sequence[PatternFieldResolution]
    # the rest field, when present
    rest: PatternRestResolution | None
    kind: typing.Literal["array"] = "array"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternSequenceResolutionSlice:
    """Borrowed slice pattern, like `[head, ...tail]` over `[T]`."""

    # the fixed prefix and suffix fields
    fields: Sequence[PatternFieldResolution]
    # the rest field, when present
    rest: PatternRestResolution | None
    kind: typing.Literal["slice"] = "slice"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternSequenceResolutionFixedArray:
    """Fixed-size array pattern, like `[a, b, c]` over `[T; 3]`."""

    # the fixed element fields
    fields: Sequence[PatternFieldResolution]
    # the committed array length singleton
    length: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["fixedArray"] = "fixedArray"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Ordered collection selected by one pattern."""
PatternSequenceResolution: typing.TypeAlias = (
    PatternSequenceResolutionArray
    | PatternSequenceResolutionSlice
    | PatternSequenceResolutionFixedArray
)

def encode_pattern_sequence_resolution(
    writer: BinaryWriter, value: PatternSequenceResolution
) -> None: ...
def decode_pattern_sequence_resolution(
    reader: BinaryReader,
) -> PatternSequenceResolution: ...
def to_json_pattern_sequence_resolution(value: PatternSequenceResolution) -> Json: ...
def from_json_pattern_sequence_resolution(value: Json) -> PatternSequenceResolution: ...

@dataclass(frozen=True, slots=True)
class PatternRestResolution:
    """Rest field selected by one ordered pattern."""

    # the source node that introduces the rest field
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the nested pattern matched for the rest field
    pattern: destack._generated.dir.tree.node.GlobalNodeIdAny | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternRestResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PatternRestResolution: ...

def encode_pattern_rest_resolution(
    writer: BinaryWriter, value: PatternRestResolution
) -> None: ...
def decode_pattern_rest_resolution(reader: BinaryReader) -> PatternRestResolution: ...
def to_json_pattern_rest_resolution(value: PatternRestResolution) -> Json: ...
def from_json_pattern_rest_resolution(value: Json) -> PatternRestResolution: ...

@dataclass(frozen=True, slots=True)
class PatternShapeResolution:
    """Structural fields selected by one pattern."""

    # the structural field mapping in source order
    fields: Sequence[PatternFieldResolution]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternShapeResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PatternShapeResolution: ...

def encode_pattern_shape_resolution(
    writer: BinaryWriter, value: PatternShapeResolution
) -> None: ...
def decode_pattern_shape_resolution(reader: BinaryReader) -> PatternShapeResolution: ...
def to_json_pattern_shape_resolution(value: PatternShapeResolution) -> Json: ...
def from_json_pattern_shape_resolution(value: Json) -> PatternShapeResolution: ...

@dataclass(frozen=True, slots=True)
class PatternNominalResolution:
    """Symbol-backed nominal pattern selected during checking."""

    # the selected nominal symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the generic arguments of the nominal symbol, empty when not statically applied
    arguments: Sequence[destack._generated.dir.type.type.GlobalTypeId]
    # the nominal field mapping in source order
    fields: Sequence[PatternFieldResolution]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternNominalResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PatternNominalResolution: ...

def encode_pattern_nominal_resolution(
    writer: BinaryWriter, value: PatternNominalResolution
) -> None: ...
def decode_pattern_nominal_resolution(
    reader: BinaryReader,
) -> PatternNominalResolution: ...
def to_json_pattern_nominal_resolution(value: PatternNominalResolution) -> Json: ...
def from_json_pattern_nominal_resolution(value: Json) -> PatternNominalResolution: ...

@dataclass(frozen=True, slots=True)
class PatternNewtypeResolution:
    """Symbol-backed newtype pattern selected during checking."""

    # the selected newtype symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the generic arguments of the newtype symbol, empty when not statically applied
    arguments: Sequence[destack._generated.dir.type.type.GlobalTypeId]
    # the wrapped value pattern
    value: destack._generated.dir.tree.node.GlobalNodeIdAny | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternNewtypeResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PatternNewtypeResolution: ...

def encode_pattern_newtype_resolution(
    writer: BinaryWriter, value: PatternNewtypeResolution
) -> None: ...
def decode_pattern_newtype_resolution(
    reader: BinaryReader,
) -> PatternNewtypeResolution: ...
def to_json_pattern_newtype_resolution(value: PatternNewtypeResolution) -> Json: ...
def from_json_pattern_newtype_resolution(value: Json) -> PatternNewtypeResolution: ...

@dataclass(frozen=True, slots=True)
class PatternVariantResolution:
    """Symbol-backed variant pattern selected during checking."""

    # the selected variant family symbol
    owner: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the selected variant symbol
    variant: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the generic arguments of the variant symbol, empty when not statically applied
    arguments: Sequence[destack._generated.dir.type.type.GlobalTypeId]
    # the discriminant value
    discriminant: destack._generated.dir.tree.literal.ScalarLiteral
    # the variant field mapping in source order
    fields: Sequence[PatternFieldResolution]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternVariantResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PatternVariantResolution: ...

def encode_pattern_variant_resolution(
    writer: BinaryWriter, value: PatternVariantResolution
) -> None: ...
def decode_pattern_variant_resolution(
    reader: BinaryReader,
) -> PatternVariantResolution: ...
def to_json_pattern_variant_resolution(value: PatternVariantResolution) -> Json: ...
def from_json_pattern_variant_resolution(value: Json) -> PatternVariantResolution: ...

@dataclass(frozen=True, slots=True)
class PatternUnionResolution:
    """Alternative patterns selected during checking."""

    # the alternative pattern nodes
    alternatives: Sequence[destack._generated.dir.tree.node.GlobalNodeIdAny]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternUnionResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PatternUnionResolution: ...

def encode_pattern_union_resolution(
    writer: BinaryWriter, value: PatternUnionResolution
) -> None: ...
def decode_pattern_union_resolution(reader: BinaryReader) -> PatternUnionResolution: ...
def to_json_pattern_union_resolution(value: PatternUnionResolution) -> Json: ...
def from_json_pattern_union_resolution(value: Json) -> PatternUnionResolution: ...

@dataclass(frozen=True, slots=True)
class PatternBorrowResolution:
    """Borrow operation selected by one pattern."""

    # the requested borrow access, if source explicit
    access: destack._generated.dir.type.type.Access | None
    # the pattern matched through the borrow
    pattern: destack._generated.dir.tree.node.GlobalNodeIdAny

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternBorrowResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PatternBorrowResolution: ...

def encode_pattern_borrow_resolution(
    writer: BinaryWriter, value: PatternBorrowResolution
) -> None: ...
def decode_pattern_borrow_resolution(
    reader: BinaryReader,
) -> PatternBorrowResolution: ...
def to_json_pattern_borrow_resolution(value: PatternBorrowResolution) -> Json: ...
def from_json_pattern_borrow_resolution(value: Json) -> PatternBorrowResolution: ...

@dataclass(frozen=True, slots=True)
class PatternMoveResolution:
    """Move operation selected by one pattern."""

    # the requested move access, if source explicit
    access: destack._generated.dir.type.type.Access | None
    # the pattern matched after moving
    pattern: destack._generated.dir.tree.node.GlobalNodeIdAny

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternMoveResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PatternMoveResolution: ...

def encode_pattern_move_resolution(
    writer: BinaryWriter, value: PatternMoveResolution
) -> None: ...
def decode_pattern_move_resolution(reader: BinaryReader) -> PatternMoveResolution: ...
def to_json_pattern_move_resolution(value: PatternMoveResolution) -> Json: ...
def from_json_pattern_move_resolution(value: Json) -> PatternMoveResolution: ...

@dataclass(frozen=True, slots=True)
class PatternDereferenceResolution:
    """Dereference operation selected by one pattern."""

    # the pattern matched through the dereference
    pattern: destack._generated.dir.tree.node.GlobalNodeIdAny

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternDereferenceResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PatternDereferenceResolution: ...

def encode_pattern_dereference_resolution(
    writer: BinaryWriter, value: PatternDereferenceResolution
) -> None: ...
def decode_pattern_dereference_resolution(
    reader: BinaryReader,
) -> PatternDereferenceResolution: ...
def to_json_pattern_dereference_resolution(
    value: PatternDereferenceResolution,
) -> Json: ...
def from_json_pattern_dereference_resolution(
    value: Json,
) -> PatternDereferenceResolution: ...

@dataclass(frozen=True, slots=True)
class AssignPatternResolutionPlace:
    """Direct writable place target, like `value` or `object.field`."""

    place: AssignPatternPlaceResolution
    kind: typing.Literal["place"] = "place"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class AssignPatternResolutionDefault:
    """Defaulted assignment target, like `value = fallback`."""

    default: AssignPatternDefaultResolution
    kind: typing.Literal["default"] = "default"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class AssignPatternResolutionSequence:
    """Ordered destructuring target, like `[head, ...tail]`."""

    sequence: AssignPatternSequenceResolution
    kind: typing.Literal["sequence"] = "sequence"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class AssignPatternResolutionObject:
    """Object destructuring target, like `{ name, age: years }`."""

    object: AssignPatternObjectResolution
    kind: typing.Literal["object"] = "object"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Assignment target meaning selected during checking."""
AssignPatternResolution: typing.TypeAlias = (
    AssignPatternResolutionPlace
    | AssignPatternResolutionDefault
    | AssignPatternResolutionSequence
    | AssignPatternResolutionObject
)

def encode_assign_pattern_resolution(
    writer: BinaryWriter, value: AssignPatternResolution
) -> None: ...
def decode_assign_pattern_resolution(
    reader: BinaryReader,
) -> AssignPatternResolution: ...
def to_json_assign_pattern_resolution(value: AssignPatternResolution) -> Json: ...
def from_json_assign_pattern_resolution(value: Json) -> AssignPatternResolution: ...

@dataclass(frozen=True, slots=True)
class AssignPatternPlaceResolution:
    """Direct assignment place selected during checking."""

    # the expression node that designates the writable place
    target: destack._generated.dir.tree.node.GlobalNodeIdAny

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> AssignPatternPlaceResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> AssignPatternPlaceResolution: ...

def encode_assign_pattern_place_resolution(
    writer: BinaryWriter, value: AssignPatternPlaceResolution
) -> None: ...
def decode_assign_pattern_place_resolution(
    reader: BinaryReader,
) -> AssignPatternPlaceResolution: ...
def to_json_assign_pattern_place_resolution(
    value: AssignPatternPlaceResolution,
) -> Json: ...
def from_json_assign_pattern_place_resolution(
    value: Json,
) -> AssignPatternPlaceResolution: ...

@dataclass(frozen=True, slots=True)
class AssignPatternDefaultResolution:
    """Defaulted assignment target selected during checking."""

    # the nested assignment target
    pattern: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the fallback expression
    value: destack._generated.dir.tree.node.GlobalNodeIdAny

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> AssignPatternDefaultResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> AssignPatternDefaultResolution: ...

def encode_assign_pattern_default_resolution(
    writer: BinaryWriter, value: AssignPatternDefaultResolution
) -> None: ...
def decode_assign_pattern_default_resolution(
    reader: BinaryReader,
) -> AssignPatternDefaultResolution: ...
def to_json_assign_pattern_default_resolution(
    value: AssignPatternDefaultResolution,
) -> Json: ...
def from_json_assign_pattern_default_resolution(
    value: Json,
) -> AssignPatternDefaultResolution: ...

@dataclass(frozen=True, slots=True)
class AssignPatternSequenceResolution:
    """Ordered assignment destructuring selected during checking."""

    # the fixed fields in source order
    fields: Sequence[AssignPatternFieldResolution]
    # the rest target, when present
    rest: AssignPatternRestResolution | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> AssignPatternSequenceResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> AssignPatternSequenceResolution: ...

def encode_assign_pattern_sequence_resolution(
    writer: BinaryWriter, value: AssignPatternSequenceResolution
) -> None: ...
def decode_assign_pattern_sequence_resolution(
    reader: BinaryReader,
) -> AssignPatternSequenceResolution: ...
def to_json_assign_pattern_sequence_resolution(
    value: AssignPatternSequenceResolution,
) -> Json: ...
def from_json_assign_pattern_sequence_resolution(
    value: Json,
) -> AssignPatternSequenceResolution: ...

@dataclass(frozen=True, slots=True)
class AssignPatternFieldResolution:
    """One destructured assignment field."""

    # the source node that introduces the field
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the selected field target
    target: PatternFieldTarget
    # the nested assignment target
    pattern: destack._generated.dir.tree.node.GlobalNodeIdAny | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> AssignPatternFieldResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> AssignPatternFieldResolution: ...

def encode_assign_pattern_field_resolution(
    writer: BinaryWriter, value: AssignPatternFieldResolution
) -> None: ...
def decode_assign_pattern_field_resolution(
    reader: BinaryReader,
) -> AssignPatternFieldResolution: ...
def to_json_assign_pattern_field_resolution(
    value: AssignPatternFieldResolution,
) -> Json: ...
def from_json_assign_pattern_field_resolution(
    value: Json,
) -> AssignPatternFieldResolution: ...

@dataclass(frozen=True, slots=True)
class AssignPatternRestResolution:
    """Rest field selected by one assignment destructuring pattern."""

    # the source node that introduces the rest field
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the nested assignment target
    pattern: destack._generated.dir.tree.node.GlobalNodeIdAny | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> AssignPatternRestResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> AssignPatternRestResolution: ...

def encode_assign_pattern_rest_resolution(
    writer: BinaryWriter, value: AssignPatternRestResolution
) -> None: ...
def decode_assign_pattern_rest_resolution(
    reader: BinaryReader,
) -> AssignPatternRestResolution: ...
def to_json_assign_pattern_rest_resolution(
    value: AssignPatternRestResolution,
) -> Json: ...
def from_json_assign_pattern_rest_resolution(
    value: Json,
) -> AssignPatternRestResolution: ...

@dataclass(frozen=True, slots=True)
class AssignPatternObjectResolution:
    """Object assignment destructuring selected during checking."""

    # the named fields in source order
    fields: Sequence[AssignPatternFieldResolution]
    # the rest target, when present
    rest: AssignPatternRestResolution | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> AssignPatternObjectResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> AssignPatternObjectResolution: ...

def encode_assign_pattern_object_resolution(
    writer: BinaryWriter, value: AssignPatternObjectResolution
) -> None: ...
def decode_assign_pattern_object_resolution(
    reader: BinaryReader,
) -> AssignPatternObjectResolution: ...
def to_json_assign_pattern_object_resolution(
    value: AssignPatternObjectResolution,
) -> Json: ...
def from_json_assign_pattern_object_resolution(
    value: Json,
) -> AssignPatternObjectResolution: ...

__all__ = [
    "NameResolution",
    "encode_name_resolution",
    "decode_name_resolution",
    "to_json_name_resolution",
    "from_json_name_resolution",
    "LabelResolution",
    "encode_label_resolution",
    "decode_label_resolution",
    "to_json_label_resolution",
    "from_json_label_resolution",
    "LabelResolutionSymbol",
    "LabelResolutionLoop",
    "LabelResolutionFunction",
    "ReceiverResolution",
    "encode_receiver_resolution",
    "decode_receiver_resolution",
    "to_json_receiver_resolution",
    "from_json_receiver_resolution",
    "ReceiverKind",
    "encode_receiver_kind",
    "decode_receiver_kind",
    "to_json_receiver_kind",
    "from_json_receiver_kind",
    "MemberResolution",
    "encode_member_resolution",
    "decode_member_resolution",
    "to_json_member_resolution",
    "from_json_member_resolution",
    "MemberTarget",
    "encode_member_target",
    "decode_member_target",
    "to_json_member_target",
    "from_json_member_target",
    "MemberTargetField",
    "MemberTargetElement",
    "MemberTargetIndex",
    "MemberTargetSymbol",
    "MemberTargetExistential",
    "MemberTargetUniversal",
    "MemberCandidate",
    "encode_member_candidate",
    "decode_member_candidate",
    "to_json_member_candidate",
    "from_json_member_candidate",
    "CallResolution",
    "encode_call_resolution",
    "decode_call_resolution",
    "to_json_call_resolution",
    "from_json_call_resolution",
    "CallTarget",
    "encode_call_target",
    "decode_call_target",
    "to_json_call_target",
    "from_json_call_target",
    "CallTargetBuiltin",
    "CallTargetExpression",
    "CallTargetSymbol",
    "CallTargetUniversal",
    "BuiltinCall",
    "encode_builtin_call",
    "decode_builtin_call",
    "to_json_builtin_call",
    "from_json_builtin_call",
    "BuiltinCallUnaryOperator",
    "BuiltinCallBinaryOperator",
    "CallCandidate",
    "encode_call_candidate",
    "decode_call_candidate",
    "to_json_call_candidate",
    "from_json_call_candidate",
    "ReadWriteResolution",
    "encode_read_write_resolution",
    "decode_read_write_resolution",
    "to_json_read_write_resolution",
    "from_json_read_write_resolution",
    "ConstructResolution",
    "encode_construct_resolution",
    "decode_construct_resolution",
    "to_json_construct_resolution",
    "from_json_construct_resolution",
    "ConstructTarget",
    "encode_construct_target",
    "decode_construct_target",
    "to_json_construct_target",
    "from_json_construct_target",
    "ConstructTargetClass",
    "ConstructTargetNewtype",
    "ClassConstructCandidate",
    "encode_class_construct_candidate",
    "decode_class_construct_candidate",
    "to_json_class_construct_candidate",
    "from_json_class_construct_candidate",
    "NewtypeConstructCandidate",
    "encode_newtype_construct_candidate",
    "decode_newtype_construct_candidate",
    "to_json_newtype_construct_candidate",
    "from_json_newtype_construct_candidate",
    "PatternResolution",
    "encode_pattern_resolution",
    "decode_pattern_resolution",
    "to_json_pattern_resolution",
    "from_json_pattern_resolution",
    "PatternResolutionWildcard",
    "PatternResolutionBinding",
    "PatternResolutionLiteral",
    "PatternResolutionRange",
    "PatternResolutionTuple",
    "PatternResolutionSequence",
    "PatternResolutionShape",
    "PatternResolutionNominal",
    "PatternResolutionNewtype",
    "PatternResolutionVariant",
    "PatternResolutionUnion",
    "PatternResolutionBorrow",
    "PatternResolutionMove",
    "PatternResolutionDereference",
    "PatternBindingResolution",
    "encode_pattern_binding_resolution",
    "decode_pattern_binding_resolution",
    "to_json_pattern_binding_resolution",
    "from_json_pattern_binding_resolution",
    "PatternLiteralResolution",
    "encode_pattern_literal_resolution",
    "decode_pattern_literal_resolution",
    "to_json_pattern_literal_resolution",
    "from_json_pattern_literal_resolution",
    "PatternRangeResolution",
    "encode_pattern_range_resolution",
    "decode_pattern_range_resolution",
    "to_json_pattern_range_resolution",
    "from_json_pattern_range_resolution",
    "PatternTupleResolution",
    "encode_pattern_tuple_resolution",
    "decode_pattern_tuple_resolution",
    "to_json_pattern_tuple_resolution",
    "from_json_pattern_tuple_resolution",
    "PatternFieldResolution",
    "encode_pattern_field_resolution",
    "decode_pattern_field_resolution",
    "to_json_pattern_field_resolution",
    "from_json_pattern_field_resolution",
    "PatternFieldTarget",
    "encode_pattern_field_target",
    "decode_pattern_field_target",
    "to_json_pattern_field_target",
    "from_json_pattern_field_target",
    "PatternFieldTargetKey",
    "PatternFieldTargetIndex",
    "PatternSequenceResolution",
    "encode_pattern_sequence_resolution",
    "decode_pattern_sequence_resolution",
    "to_json_pattern_sequence_resolution",
    "from_json_pattern_sequence_resolution",
    "PatternSequenceResolutionArray",
    "PatternSequenceResolutionSlice",
    "PatternSequenceResolutionFixedArray",
    "PatternRestResolution",
    "encode_pattern_rest_resolution",
    "decode_pattern_rest_resolution",
    "to_json_pattern_rest_resolution",
    "from_json_pattern_rest_resolution",
    "PatternShapeResolution",
    "encode_pattern_shape_resolution",
    "decode_pattern_shape_resolution",
    "to_json_pattern_shape_resolution",
    "from_json_pattern_shape_resolution",
    "PatternNominalResolution",
    "encode_pattern_nominal_resolution",
    "decode_pattern_nominal_resolution",
    "to_json_pattern_nominal_resolution",
    "from_json_pattern_nominal_resolution",
    "PatternNewtypeResolution",
    "encode_pattern_newtype_resolution",
    "decode_pattern_newtype_resolution",
    "to_json_pattern_newtype_resolution",
    "from_json_pattern_newtype_resolution",
    "PatternVariantResolution",
    "encode_pattern_variant_resolution",
    "decode_pattern_variant_resolution",
    "to_json_pattern_variant_resolution",
    "from_json_pattern_variant_resolution",
    "PatternUnionResolution",
    "encode_pattern_union_resolution",
    "decode_pattern_union_resolution",
    "to_json_pattern_union_resolution",
    "from_json_pattern_union_resolution",
    "PatternBorrowResolution",
    "encode_pattern_borrow_resolution",
    "decode_pattern_borrow_resolution",
    "to_json_pattern_borrow_resolution",
    "from_json_pattern_borrow_resolution",
    "PatternMoveResolution",
    "encode_pattern_move_resolution",
    "decode_pattern_move_resolution",
    "to_json_pattern_move_resolution",
    "from_json_pattern_move_resolution",
    "PatternDereferenceResolution",
    "encode_pattern_dereference_resolution",
    "decode_pattern_dereference_resolution",
    "to_json_pattern_dereference_resolution",
    "from_json_pattern_dereference_resolution",
    "AssignPatternResolution",
    "encode_assign_pattern_resolution",
    "decode_assign_pattern_resolution",
    "to_json_assign_pattern_resolution",
    "from_json_assign_pattern_resolution",
    "AssignPatternResolutionPlace",
    "AssignPatternResolutionDefault",
    "AssignPatternResolutionSequence",
    "AssignPatternResolutionObject",
    "AssignPatternPlaceResolution",
    "encode_assign_pattern_place_resolution",
    "decode_assign_pattern_place_resolution",
    "to_json_assign_pattern_place_resolution",
    "from_json_assign_pattern_place_resolution",
    "AssignPatternDefaultResolution",
    "encode_assign_pattern_default_resolution",
    "decode_assign_pattern_default_resolution",
    "to_json_assign_pattern_default_resolution",
    "from_json_assign_pattern_default_resolution",
    "AssignPatternSequenceResolution",
    "encode_assign_pattern_sequence_resolution",
    "decode_assign_pattern_sequence_resolution",
    "to_json_assign_pattern_sequence_resolution",
    "from_json_assign_pattern_sequence_resolution",
    "AssignPatternFieldResolution",
    "encode_assign_pattern_field_resolution",
    "decode_assign_pattern_field_resolution",
    "to_json_assign_pattern_field_resolution",
    "from_json_assign_pattern_field_resolution",
    "AssignPatternRestResolution",
    "encode_assign_pattern_rest_resolution",
    "decode_assign_pattern_rest_resolution",
    "to_json_assign_pattern_rest_resolution",
    "from_json_assign_pattern_rest_resolution",
    "AssignPatternObjectResolution",
    "encode_assign_pattern_object_resolution",
    "decode_assign_pattern_object_resolution",
    "to_json_assign_pattern_object_resolution",
    "from_json_assign_pattern_object_resolution",
]
