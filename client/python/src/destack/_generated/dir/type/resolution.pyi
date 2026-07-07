# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.symbol.key
import destack._generated.dir.symbol.symbol
import destack._generated.dir.table.definition
import destack._generated.dir.tree.literal
import destack._generated.dir.tree.node
import destack._generated.dir.tree.operator
import destack._generated.dir.type.generic
import destack._generated.dir.type.predicate
import destack._generated.dir.type.projection
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
class InstantiationResolution:
    """Explicit generic application selected at a usage site."""

    # the generic declaration being applied
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the complete selected generic argument bindings
    generic_arguments: Sequence[
        destack._generated.dir.type.generic.GenericArgumentBinding
    ]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> InstantiationResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> InstantiationResolution: ...

def encode_instantiation_resolution(
    writer: BinaryWriter, value: InstantiationResolution
) -> None: ...
def decode_instantiation_resolution(
    reader: BinaryReader,
) -> InstantiationResolution: ...
def to_json_instantiation_resolution(value: InstantiationResolution) -> Json: ...
def from_json_instantiation_resolution(value: Json) -> InstantiationResolution: ...

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
    declaration: destack._generated.dir.symbol.symbol.GlobalSymbolId
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
    # the projection steps
    adjustments: Sequence[destack._generated.dir.type.projection.Projection]
    # the declaration that exposed this member
    owner: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the selected member symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the member type applied to the matched receiver
    ty: destack._generated.dir.type.type.GlobalTypeId
    # the selected generic argument bindings needed by this member candidate
    generic_arguments: Sequence[
        destack._generated.dir.type.generic.GenericArgumentBinding
    ]

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
    # the callable type selected at the call site, when one exists
    callable_type: destack._generated.dir.type.type.GlobalTypeId | None
    # the dynamic parameter types after static substitutions
    parameters: Sequence[destack._generated.dir.type.type.GlobalTypeId]
    # the source arguments bound to selected parameters
    arguments: Sequence[destack._generated.dir.type.generic.ArgumentBinding]
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

    # the selected generic argument bindings, empty when not statically applied
    generic_arguments: Sequence[
        destack._generated.dir.type.generic.GenericArgumentBinding
    ]
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
    # the projection steps
    adjustments: Sequence[destack._generated.dir.type.projection.Projection]
    # the selected callable symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the selected generic argument bindings needed by this call candidate
    generic_arguments: Sequence[
        destack._generated.dir.type.generic.GenericArgumentBinding
    ]

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
class PlaceResolution:
    """Place selected by a checked expression."""

    # the expression node that designates the place
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the selected storage location
    storage: Storage
    # the value type stored in the place
    ty: destack._generated.dir.type.type.GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PlaceResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PlaceResolution: ...

def encode_place_resolution(writer: BinaryWriter, value: PlaceResolution) -> None: ...
def decode_place_resolution(reader: BinaryReader) -> PlaceResolution: ...
def to_json_place_resolution(value: PlaceResolution) -> Json: ...
def from_json_place_resolution(value: Json) -> PlaceResolution: ...

@dataclass(frozen=True, slots=True)
class StorageBinding:
    """Local or imported value binding."""

    # the selected binding symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    kind: typing.Literal["binding"] = "binding"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class StorageField:
    """Structural or nominal field storage."""

    # the receiver type
    receiver: destack._generated.dir.type.type.GlobalTypeId
    # the selected field
    field: destack._generated.dir.type.projection.ProjectionField
    kind: typing.Literal["field"] = "field"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class StorageProperty:
    """Accessor-backed property storage."""

    # the selected getter member, when the source operator reads first
    read: MemberResolution | None
    # the selected setter member
    write: MemberResolution
    kind: typing.Literal["property"] = "property"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class StorageSubscript:
    """Subscript-selected storage."""

    # the source node providing the subscript key
    index: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the selected subscript operation, when the source operator reads first
    read: destack._generated.dir.type.projection.SubscriptOperation | None
    # the selected write operation
    write: destack._generated.dir.type.projection.SubscriptOperation
    kind: typing.Literal["subscript"] = "subscript"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class StorageDereference:
    """Dereferenced storage."""

    # the selected dereference operation, when the source operator reads first
    read: destack._generated.dir.type.projection.DereferenceOperation | None
    # the selected write operation
    write: destack._generated.dir.type.projection.DereferenceOperation
    kind: typing.Literal["dereference"] = "dereference"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Writable storage location selected by a place expression."""
Storage: typing.TypeAlias = (
    StorageBinding
    | StorageField
    | StorageProperty
    | StorageSubscript
    | StorageDereference
)

def encode_storage(writer: BinaryWriter, value: Storage) -> None: ...
def decode_storage(reader: BinaryReader) -> Storage: ...
def to_json_storage(value: Storage) -> Json: ...
def from_json_storage(value: Json) -> Storage: ...

@dataclass(frozen=True, slots=True)
class GuardResolutionIs:
    """`is` guard, like `value is T`."""

    is_: IsGuardResolution
    kind: typing.Literal["is"] = "is"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class GuardResolutionInstanceOf:
    """`instanceof` guard, like `value instanceof User`."""

    instance_of: InstanceOfGuardResolution
    kind: typing.Literal["instanceOf"] = "instanceOf"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class GuardResolutionIn:
    """`in` guard, like `"name" in value`."""

    in_: InGuardResolution
    kind: typing.Literal["in"] = "in"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Guard expression selected during checking."""
GuardResolution: typing.TypeAlias = (
    GuardResolutionIs | GuardResolutionInstanceOf | GuardResolutionIn
)

def encode_guard_resolution(writer: BinaryWriter, value: GuardResolution) -> None: ...
def decode_guard_resolution(reader: BinaryReader) -> GuardResolution: ...
def to_json_guard_resolution(value: GuardResolution) -> Json: ...
def from_json_guard_resolution(value: Json) -> GuardResolution: ...

@dataclass(frozen=True, slots=True)
class IsGuardResolution:
    """`is` guard selected during checking."""

    # the tested value type
    value_type: destack._generated.dir.type.type.GlobalTypeId
    # the tested target type
    target_type: destack._generated.dir.type.type.GlobalTypeId
    # the executable predicate
    predicate: destack._generated.dir.type.predicate.Predicate

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> IsGuardResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> IsGuardResolution: ...

def encode_is_guard_resolution(
    writer: BinaryWriter, value: IsGuardResolution
) -> None: ...
def decode_is_guard_resolution(reader: BinaryReader) -> IsGuardResolution: ...
def to_json_is_guard_resolution(value: IsGuardResolution) -> Json: ...
def from_json_is_guard_resolution(value: Json) -> IsGuardResolution: ...

@dataclass(frozen=True, slots=True)
class InstanceOfGuardResolution:
    """`instanceof` guard selected during checking."""

    # the tested value type
    value_type: destack._generated.dir.type.type.GlobalTypeId
    # the selected right-hand-side declaration
    target: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the selected instance type tested at runtime
    target_type: destack._generated.dir.type.type.GlobalTypeId
    # the executable predicate
    predicate: destack._generated.dir.type.predicate.Predicate

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> InstanceOfGuardResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> InstanceOfGuardResolution: ...

def encode_instance_of_guard_resolution(
    writer: BinaryWriter, value: InstanceOfGuardResolution
) -> None: ...
def decode_instance_of_guard_resolution(
    reader: BinaryReader,
) -> InstanceOfGuardResolution: ...
def to_json_instance_of_guard_resolution(value: InstanceOfGuardResolution) -> Json: ...
def from_json_instance_of_guard_resolution(
    value: Json,
) -> InstanceOfGuardResolution: ...

@dataclass(frozen=True, slots=True)
class InGuardResolution:
    """`in` guard selected during checking."""

    # the tested key type
    key_type: destack._generated.dir.type.type.GlobalTypeId
    # the tested receiver type
    receiver_type: destack._generated.dir.type.type.GlobalTypeId
    # the executable predicate
    predicate: destack._generated.dir.type.predicate.Predicate

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> InGuardResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> InGuardResolution: ...

def encode_in_guard_resolution(
    writer: BinaryWriter, value: InGuardResolution
) -> None: ...
def decode_in_guard_resolution(reader: BinaryReader) -> InGuardResolution: ...
def to_json_in_guard_resolution(value: InGuardResolution) -> Json: ...
def from_json_in_guard_resolution(value: Json) -> InGuardResolution: ...

@dataclass(frozen=True, slots=True)
class ConstructResolution:
    """Construct expression selected at a usage site."""

    # the selected construct target
    target: ConstructTarget
    # the dynamic parameter types after static substitutions
    parameters: Sequence[destack._generated.dir.type.type.GlobalTypeId]
    # the source arguments bound to selected parameters
    arguments: Sequence[destack._generated.dir.type.generic.ArgumentBinding]
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

@dataclass(frozen=True, slots=True)
class ConstructTargetVariant:
    """Tagged union variant constructor selected at compile time."""

    variant: VariantConstructCandidate
    kind: typing.Literal["variant"] = "variant"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Construct target selected at a usage site."""
ConstructTarget: typing.TypeAlias = (
    ConstructTargetClass | ConstructTargetNewtype | ConstructTargetVariant
)

def encode_construct_target(writer: BinaryWriter, value: ConstructTarget) -> None: ...
def decode_construct_target(reader: BinaryReader) -> ConstructTarget: ...
def to_json_construct_target(value: ConstructTarget) -> Json: ...
def from_json_construct_target(value: Json) -> ConstructTarget: ...

@dataclass(frozen=True, slots=True)
class ClassConstructCandidate:
    """One class construction candidate after overload selection."""

    # the selected class symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the selected class constructor
    constructor: destack._generated.dir.table.definition.ClassConstructor
    # the selected generic argument bindings for the class symbol
    generic_arguments: Sequence[
        destack._generated.dir.type.generic.GenericArgumentBinding
    ]

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
    # the selected generic argument bindings for the newtype symbol
    generic_arguments: Sequence[
        destack._generated.dir.type.generic.GenericArgumentBinding
    ]

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
class VariantConstructCandidate:
    """One tagged variant construction candidate after checking."""

    # the selected tagged case
    case: destack._generated.dir.type.projection.VariantCase
    # the selected generic argument bindings for the owner symbol
    generic_arguments: Sequence[
        destack._generated.dir.type.generic.GenericArgumentBinding
    ]
    # the discriminant value injected by the constructor
    discriminant: destack._generated.dir.tree.literal.ScalarLiteral

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> VariantConstructCandidate: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> VariantConstructCandidate: ...

def encode_variant_construct_candidate(
    writer: BinaryWriter, value: VariantConstructCandidate
) -> None: ...
def decode_variant_construct_candidate(
    reader: BinaryReader,
) -> VariantConstructCandidate: ...
def to_json_variant_construct_candidate(value: VariantConstructCandidate) -> Json: ...
def from_json_variant_construct_candidate(value: Json) -> VariantConstructCandidate: ...

@dataclass(frozen=True, slots=True)
class PatternResolutionIgnore:
    """Pattern that accepts the input without binding, like `_`."""

    kind: typing.Literal["ignore"] = "ignore"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternResolutionBind:
    """Pattern that binds a symbol, like `value`."""

    bind: PatternBindingResolution
    kind: typing.Literal["bind"] = "bind"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternResolutionMust:
    """Pattern that requires a successful nested match, like `value!`."""

    must: PatternMustResolution
    kind: typing.Literal["must"] = "must"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternResolutionDefault:
    """Pattern that uses a default value when the selected value is undefined."""

    default: PatternDefaultResolution
    kind: typing.Literal["default"] = "default"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternResolutionTest:
    """Pattern that tests one executable predicate."""

    test: PatternPredicateResolution
    kind: typing.Literal["test"] = "test"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternResolutionProject:
    """Pattern that projects the input before matching, like `*Point { x, y }`."""

    project: PatternProjectionResolution
    kind: typing.Literal["project"] = "project"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternResolutionDestructure:
    """Pattern that destructures projected child values."""

    destructure: PatternDestructureResolution
    kind: typing.Literal["destructure"] = "destructure"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternResolutionOr:
    """Pattern that accepts one of several branches, like `0 | 1 | 2`."""

    or_: PatternOrResolution
    kind: typing.Literal["or"] = "or"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Pattern meaning selected during checking."""
PatternResolution: typing.TypeAlias = (
    PatternResolutionIgnore
    | PatternResolutionBind
    | PatternResolutionMust
    | PatternResolutionDefault
    | PatternResolutionTest
    | PatternResolutionProject
    | PatternResolutionDestructure
    | PatternResolutionOr
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
class PatternMustResolution:
    """Required nested pattern selected during checking."""

    # the nested pattern that must match
    pattern: destack._generated.dir.tree.node.GlobalNodeIdAny

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternMustResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PatternMustResolution: ...

def encode_pattern_must_resolution(
    writer: BinaryWriter, value: PatternMustResolution
) -> None: ...
def decode_pattern_must_resolution(reader: BinaryReader) -> PatternMustResolution: ...
def to_json_pattern_must_resolution(value: PatternMustResolution) -> Json: ...
def from_json_pattern_must_resolution(value: Json) -> PatternMustResolution: ...

@dataclass(frozen=True, slots=True)
class PatternDefaultResolution:
    """Defaulted nested pattern selected during checking."""

    # the nested pattern
    pattern: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the default expression
    value: destack._generated.dir.tree.node.GlobalNodeIdAny

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternDefaultResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PatternDefaultResolution: ...

def encode_pattern_default_resolution(
    writer: BinaryWriter, value: PatternDefaultResolution
) -> None: ...
def decode_pattern_default_resolution(
    reader: BinaryReader,
) -> PatternDefaultResolution: ...
def to_json_pattern_default_resolution(value: PatternDefaultResolution) -> Json: ...
def from_json_pattern_default_resolution(value: Json) -> PatternDefaultResolution: ...

@dataclass(frozen=True, slots=True)
class PatternPredicateResolution:
    """Executable predicate selected by one pattern."""

    # the executable predicate
    predicate: destack._generated.dir.type.predicate.Predicate

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternPredicateResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PatternPredicateResolution: ...

def encode_pattern_predicate_resolution(
    writer: BinaryWriter, value: PatternPredicateResolution
) -> None: ...
def decode_pattern_predicate_resolution(
    reader: BinaryReader,
) -> PatternPredicateResolution: ...
def to_json_pattern_predicate_resolution(value: PatternPredicateResolution) -> Json: ...
def from_json_pattern_predicate_resolution(
    value: Json,
) -> PatternPredicateResolution: ...

@dataclass(frozen=True, slots=True)
class PatternProjectionResolution:
    """Projection selected by one pattern."""

    # the selected projection
    projection: destack._generated.dir.type.projection.Projection
    # the pattern matched after projection
    pattern: destack._generated.dir.tree.node.GlobalNodeIdAny | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternProjectionResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PatternProjectionResolution: ...

def encode_pattern_projection_resolution(
    writer: BinaryWriter, value: PatternProjectionResolution
) -> None: ...
def decode_pattern_projection_resolution(
    reader: BinaryReader,
) -> PatternProjectionResolution: ...
def to_json_pattern_projection_resolution(
    value: PatternProjectionResolution,
) -> Json: ...
def from_json_pattern_projection_resolution(
    value: Json,
) -> PatternProjectionResolution: ...

@dataclass(frozen=True, slots=True)
class PatternDestructureResolutionTuple:
    """Tuple-shaped destructuring, like `(x, y)`."""

    tuple: PatternTupleDestructureResolution
    kind: typing.Literal["tuple"] = "tuple"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternDestructureResolutionObject:
    """Object-shaped destructuring, like `{ name }`."""

    object: PatternObjectDestructureResolution
    kind: typing.Literal["object"] = "object"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternDestructureResolutionNominal:
    """Symbol-backed nominal destructuring, like `Point { x, y }`."""

    nominal: PatternNominalDestructureResolution
    kind: typing.Literal["nominal"] = "nominal"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternDestructureResolutionSequence:
    """Sequence destructuring, like `[head, ...tail]`."""

    sequence: PatternSequenceDestructureResolution
    kind: typing.Literal["sequence"] = "sequence"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PatternDestructureResolutionVariant:
    """Tagged variant destructuring, like `Status.Ok(value)`."""

    variant: PatternVariantDestructureResolution
    kind: typing.Literal["variant"] = "variant"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Destructuring selected by one pattern."""
PatternDestructureResolution: typing.TypeAlias = (
    PatternDestructureResolutionTuple
    | PatternDestructureResolutionObject
    | PatternDestructureResolutionNominal
    | PatternDestructureResolutionSequence
    | PatternDestructureResolutionVariant
)

def encode_pattern_destructure_resolution(
    writer: BinaryWriter, value: PatternDestructureResolution
) -> None: ...
def decode_pattern_destructure_resolution(
    reader: BinaryReader,
) -> PatternDestructureResolution: ...
def to_json_pattern_destructure_resolution(
    value: PatternDestructureResolution,
) -> Json: ...
def from_json_pattern_destructure_resolution(
    value: Json,
) -> PatternDestructureResolution: ...

@dataclass(frozen=True, slots=True)
class PatternTupleDestructureResolution:
    """Tuple destructuring selected by one pattern."""

    # the tuple fields in source order
    fields: Sequence[PatternFieldResolution]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternTupleDestructureResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PatternTupleDestructureResolution: ...

def encode_pattern_tuple_destructure_resolution(
    writer: BinaryWriter, value: PatternTupleDestructureResolution
) -> None: ...
def decode_pattern_tuple_destructure_resolution(
    reader: BinaryReader,
) -> PatternTupleDestructureResolution: ...
def to_json_pattern_tuple_destructure_resolution(
    value: PatternTupleDestructureResolution,
) -> Json: ...
def from_json_pattern_tuple_destructure_resolution(
    value: Json,
) -> PatternTupleDestructureResolution: ...

@dataclass(frozen=True, slots=True)
class PatternFieldResolution:
    """One destructured pattern field."""

    # the source node that introduces the field
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the selected field projection
    projection: destack._generated.dir.type.projection.Projection
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
class PatternObjectDestructureResolution:
    """Object destructuring selected by one pattern."""

    # the object fields in source order
    fields: Sequence[PatternFieldResolution]
    # the rest field, when present
    rest: PatternFieldResolution | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternObjectDestructureResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PatternObjectDestructureResolution: ...

def encode_pattern_object_destructure_resolution(
    writer: BinaryWriter, value: PatternObjectDestructureResolution
) -> None: ...
def decode_pattern_object_destructure_resolution(
    reader: BinaryReader,
) -> PatternObjectDestructureResolution: ...
def to_json_pattern_object_destructure_resolution(
    value: PatternObjectDestructureResolution,
) -> Json: ...
def from_json_pattern_object_destructure_resolution(
    value: Json,
) -> PatternObjectDestructureResolution: ...

@dataclass(frozen=True, slots=True)
class PatternNominalDestructureResolution:
    """Nominal destructuring selected by one pattern."""

    # the selected nominal symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the selected generic argument bindings for the nominal symbol
    generic_arguments: Sequence[
        destack._generated.dir.type.generic.GenericArgumentBinding
    ]
    # the nominal fields in source order
    fields: Sequence[PatternFieldResolution]
    # the rest field, when present
    rest: PatternFieldResolution | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternNominalDestructureResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PatternNominalDestructureResolution: ...

def encode_pattern_nominal_destructure_resolution(
    writer: BinaryWriter, value: PatternNominalDestructureResolution
) -> None: ...
def decode_pattern_nominal_destructure_resolution(
    reader: BinaryReader,
) -> PatternNominalDestructureResolution: ...
def to_json_pattern_nominal_destructure_resolution(
    value: PatternNominalDestructureResolution,
) -> Json: ...
def from_json_pattern_nominal_destructure_resolution(
    value: Json,
) -> PatternNominalDestructureResolution: ...

@dataclass(frozen=True, slots=True)
class PatternSequenceDestructureResolution:
    """Sequence destructuring selected by one pattern."""

    # the arity requirement introduced by the pattern
    arity: PatternSequenceArity
    # the fixed fields in source order
    fields: Sequence[PatternFieldResolution]
    # the rest field, when present
    rest: PatternFieldResolution | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternSequenceDestructureResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PatternSequenceDestructureResolution: ...

def encode_pattern_sequence_destructure_resolution(
    writer: BinaryWriter, value: PatternSequenceDestructureResolution
) -> None: ...
def decode_pattern_sequence_destructure_resolution(
    reader: BinaryReader,
) -> PatternSequenceDestructureResolution: ...
def to_json_pattern_sequence_destructure_resolution(
    value: PatternSequenceDestructureResolution,
) -> Json: ...
def from_json_pattern_sequence_destructure_resolution(
    value: Json,
) -> PatternSequenceDestructureResolution: ...

@dataclass(frozen=True, slots=True)
class PatternSequenceArity:
    """Arity requirement introduced by one sequence pattern."""

    # the minimum accepted source length
    minimum: int
    # the maximum accepted source length, when bounded
    maximum: int | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternSequenceArity: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PatternSequenceArity: ...

def encode_pattern_sequence_arity(
    writer: BinaryWriter, value: PatternSequenceArity
) -> None: ...
def decode_pattern_sequence_arity(reader: BinaryReader) -> PatternSequenceArity: ...
def to_json_pattern_sequence_arity(value: PatternSequenceArity) -> Json: ...
def from_json_pattern_sequence_arity(value: Json) -> PatternSequenceArity: ...

@dataclass(frozen=True, slots=True)
class PatternVariantDestructureResolution:
    """Tagged variant destructuring selected by one pattern."""

    # the selected variant predicate
    predicate: destack._generated.dir.type.predicate.Predicate
    # the selected variant payload projection
    projection: destack._generated.dir.type.projection.Projection
    # the payload fields in source order
    fields: Sequence[PatternFieldResolution]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternVariantDestructureResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PatternVariantDestructureResolution: ...

def encode_pattern_variant_destructure_resolution(
    writer: BinaryWriter, value: PatternVariantDestructureResolution
) -> None: ...
def decode_pattern_variant_destructure_resolution(
    reader: BinaryReader,
) -> PatternVariantDestructureResolution: ...
def to_json_pattern_variant_destructure_resolution(
    value: PatternVariantDestructureResolution,
) -> Json: ...
def from_json_pattern_variant_destructure_resolution(
    value: Json,
) -> PatternVariantDestructureResolution: ...

@dataclass(frozen=True, slots=True)
class PatternOrResolution:
    """Or-pattern branches selected during checking."""

    # the branch pattern nodes
    patterns: Sequence[destack._generated.dir.tree.node.GlobalNodeIdAny]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternOrResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PatternOrResolution: ...

def encode_pattern_or_resolution(
    writer: BinaryWriter, value: PatternOrResolution
) -> None: ...
def decode_pattern_or_resolution(reader: BinaryReader) -> PatternOrResolution: ...
def to_json_pattern_or_resolution(value: PatternOrResolution) -> Json: ...
def from_json_pattern_or_resolution(value: Json) -> PatternOrResolution: ...

@dataclass(frozen=True, slots=True)
class AssignPatternResolutionPlace:
    """Direct writable place target, like `value` or `object.field`."""

    place: PlaceResolution
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
class AssignPatternResolutionTuple:
    """Tuple destructuring target, like `(x, y)` or `(x,)`."""

    tuple: AssignPatternTupleResolution
    kind: typing.Literal["tuple"] = "tuple"

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
    | AssignPatternResolutionTuple
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

    # the sequence arity required by the assignment target
    arity: PatternSequenceArity
    # the fixed fields in source order
    fields: Sequence[AssignPatternFieldResolution]
    # the rest target, when present
    rest: AssignPatternFieldResolution | None

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
    # the selected field projection
    projection: destack._generated.dir.type.projection.Projection
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
class AssignPatternTupleResolution:
    """Tuple assignment destructuring selected during checking."""

    # the projected tuple fields in source order
    fields: Sequence[AssignPatternFieldResolution]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> AssignPatternTupleResolution: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> AssignPatternTupleResolution: ...

def encode_assign_pattern_tuple_resolution(
    writer: BinaryWriter, value: AssignPatternTupleResolution
) -> None: ...
def decode_assign_pattern_tuple_resolution(
    reader: BinaryReader,
) -> AssignPatternTupleResolution: ...
def to_json_assign_pattern_tuple_resolution(
    value: AssignPatternTupleResolution,
) -> Json: ...
def from_json_assign_pattern_tuple_resolution(
    value: Json,
) -> AssignPatternTupleResolution: ...

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

@dataclass(frozen=True, slots=True)
class AssignPatternRestResolution:
    """Rest field selected by one assignment destructuring pattern."""

    # the source node that introduces the rest field
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the materialized rest projection
    projection: destack._generated.dir.type.projection.Projection
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

__all__ = [
    "NameResolution",
    "encode_name_resolution",
    "decode_name_resolution",
    "to_json_name_resolution",
    "from_json_name_resolution",
    "InstantiationResolution",
    "encode_instantiation_resolution",
    "decode_instantiation_resolution",
    "to_json_instantiation_resolution",
    "from_json_instantiation_resolution",
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
    "PlaceResolution",
    "encode_place_resolution",
    "decode_place_resolution",
    "to_json_place_resolution",
    "from_json_place_resolution",
    "Storage",
    "encode_storage",
    "decode_storage",
    "to_json_storage",
    "from_json_storage",
    "StorageBinding",
    "StorageField",
    "StorageProperty",
    "StorageSubscript",
    "StorageDereference",
    "GuardResolution",
    "encode_guard_resolution",
    "decode_guard_resolution",
    "to_json_guard_resolution",
    "from_json_guard_resolution",
    "GuardResolutionIs",
    "GuardResolutionInstanceOf",
    "GuardResolutionIn",
    "IsGuardResolution",
    "encode_is_guard_resolution",
    "decode_is_guard_resolution",
    "to_json_is_guard_resolution",
    "from_json_is_guard_resolution",
    "InstanceOfGuardResolution",
    "encode_instance_of_guard_resolution",
    "decode_instance_of_guard_resolution",
    "to_json_instance_of_guard_resolution",
    "from_json_instance_of_guard_resolution",
    "InGuardResolution",
    "encode_in_guard_resolution",
    "decode_in_guard_resolution",
    "to_json_in_guard_resolution",
    "from_json_in_guard_resolution",
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
    "ConstructTargetVariant",
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
    "VariantConstructCandidate",
    "encode_variant_construct_candidate",
    "decode_variant_construct_candidate",
    "to_json_variant_construct_candidate",
    "from_json_variant_construct_candidate",
    "PatternResolution",
    "encode_pattern_resolution",
    "decode_pattern_resolution",
    "to_json_pattern_resolution",
    "from_json_pattern_resolution",
    "PatternResolutionIgnore",
    "PatternResolutionBind",
    "PatternResolutionMust",
    "PatternResolutionDefault",
    "PatternResolutionTest",
    "PatternResolutionProject",
    "PatternResolutionDestructure",
    "PatternResolutionOr",
    "PatternBindingResolution",
    "encode_pattern_binding_resolution",
    "decode_pattern_binding_resolution",
    "to_json_pattern_binding_resolution",
    "from_json_pattern_binding_resolution",
    "PatternMustResolution",
    "encode_pattern_must_resolution",
    "decode_pattern_must_resolution",
    "to_json_pattern_must_resolution",
    "from_json_pattern_must_resolution",
    "PatternDefaultResolution",
    "encode_pattern_default_resolution",
    "decode_pattern_default_resolution",
    "to_json_pattern_default_resolution",
    "from_json_pattern_default_resolution",
    "PatternPredicateResolution",
    "encode_pattern_predicate_resolution",
    "decode_pattern_predicate_resolution",
    "to_json_pattern_predicate_resolution",
    "from_json_pattern_predicate_resolution",
    "PatternProjectionResolution",
    "encode_pattern_projection_resolution",
    "decode_pattern_projection_resolution",
    "to_json_pattern_projection_resolution",
    "from_json_pattern_projection_resolution",
    "PatternDestructureResolution",
    "encode_pattern_destructure_resolution",
    "decode_pattern_destructure_resolution",
    "to_json_pattern_destructure_resolution",
    "from_json_pattern_destructure_resolution",
    "PatternDestructureResolutionTuple",
    "PatternDestructureResolutionObject",
    "PatternDestructureResolutionNominal",
    "PatternDestructureResolutionSequence",
    "PatternDestructureResolutionVariant",
    "PatternTupleDestructureResolution",
    "encode_pattern_tuple_destructure_resolution",
    "decode_pattern_tuple_destructure_resolution",
    "to_json_pattern_tuple_destructure_resolution",
    "from_json_pattern_tuple_destructure_resolution",
    "PatternFieldResolution",
    "encode_pattern_field_resolution",
    "decode_pattern_field_resolution",
    "to_json_pattern_field_resolution",
    "from_json_pattern_field_resolution",
    "PatternObjectDestructureResolution",
    "encode_pattern_object_destructure_resolution",
    "decode_pattern_object_destructure_resolution",
    "to_json_pattern_object_destructure_resolution",
    "from_json_pattern_object_destructure_resolution",
    "PatternNominalDestructureResolution",
    "encode_pattern_nominal_destructure_resolution",
    "decode_pattern_nominal_destructure_resolution",
    "to_json_pattern_nominal_destructure_resolution",
    "from_json_pattern_nominal_destructure_resolution",
    "PatternSequenceDestructureResolution",
    "encode_pattern_sequence_destructure_resolution",
    "decode_pattern_sequence_destructure_resolution",
    "to_json_pattern_sequence_destructure_resolution",
    "from_json_pattern_sequence_destructure_resolution",
    "PatternSequenceArity",
    "encode_pattern_sequence_arity",
    "decode_pattern_sequence_arity",
    "to_json_pattern_sequence_arity",
    "from_json_pattern_sequence_arity",
    "PatternVariantDestructureResolution",
    "encode_pattern_variant_destructure_resolution",
    "decode_pattern_variant_destructure_resolution",
    "to_json_pattern_variant_destructure_resolution",
    "from_json_pattern_variant_destructure_resolution",
    "PatternOrResolution",
    "encode_pattern_or_resolution",
    "decode_pattern_or_resolution",
    "to_json_pattern_or_resolution",
    "from_json_pattern_or_resolution",
    "AssignPatternResolution",
    "encode_assign_pattern_resolution",
    "decode_assign_pattern_resolution",
    "to_json_assign_pattern_resolution",
    "from_json_assign_pattern_resolution",
    "AssignPatternResolutionPlace",
    "AssignPatternResolutionDefault",
    "AssignPatternResolutionSequence",
    "AssignPatternResolutionTuple",
    "AssignPatternResolutionObject",
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
    "AssignPatternTupleResolution",
    "encode_assign_pattern_tuple_resolution",
    "decode_assign_pattern_tuple_resolution",
    "to_json_assign_pattern_tuple_resolution",
    "from_json_assign_pattern_tuple_resolution",
    "AssignPatternObjectResolution",
    "encode_assign_pattern_object_resolution",
    "decode_assign_pattern_object_resolution",
    "to_json_assign_pattern_object_resolution",
    "from_json_assign_pattern_object_resolution",
    "AssignPatternRestResolution",
    "encode_assign_pattern_rest_resolution",
    "decode_assign_pattern_rest_resolution",
    "to_json_assign_pattern_rest_resolution",
    "from_json_assign_pattern_rest_resolution",
]
