# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.string
import destack._generated.dir.symbol.key
import destack._generated.dir.symbol.symbol
import destack._generated.dir.tree.literal
import destack._generated.dir.tree.node
import destack._generated.dir.tree.static
import destack._generated.dir.tree.type
import destack._generated.dir.type.generic
import destack._generated.dir.type.primitive
import destack._generated.source.file.model.module

"""One open inference variable inside a checked component."""
TypeVariableId: typing.TypeAlias = int

def encode_type_variable_id(writer: BinaryWriter, value: TypeVariableId) -> None: ...
def decode_type_variable_id(reader: BinaryReader) -> TypeVariableId: ...
def to_json_type_variable_id(value: TypeVariableId) -> Json: ...
def from_json_type_variable_id(value: Json) -> TypeVariableId: ...

@dataclass(frozen=True, slots=True)
class MemoryLiteralAccess:
    """Memory access singleton, like `"readonly"` or `"exclusive"`."""

    access: Access
    kind: typing.Literal["access"] = "access"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class MemoryLiteralSpace:
    """Storage space singleton, like `"local"` or `"shared"`."""

    space: Space
    kind: typing.Literal["space"] = "space"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class MemoryLiteralPlace:
    """Placement singleton, like `"ambient"` or a concrete space."""

    place: Place
    kind: typing.Literal["place"] = "place"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class MemoryLiteralLifetime:
    """Lifetime singleton, like `"static"` or a lifetime parameter."""

    lifetime: Lifetime
    kind: typing.Literal["lifetime"] = "lifetime"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Singleton type of one normalized memory value."""
MemoryLiteral: typing.TypeAlias = (
    MemoryLiteralAccess
    | MemoryLiteralSpace
    | MemoryLiteralPlace
    | MemoryLiteralLifetime
)

def encode_memory_literal(writer: BinaryWriter, value: MemoryLiteral) -> None: ...
def decode_memory_literal(reader: BinaryReader) -> MemoryLiteral: ...
def to_json_memory_literal(value: MemoryLiteral) -> Json: ...
def from_json_memory_literal(value: Json) -> MemoryLiteral: ...

"""Normalized memory access value."""
Access: typing.TypeAlias = (
    typing.Literal["readonly"] | typing.Literal["mutable"] | typing.Literal["exclusive"]
)

def encode_access(writer: BinaryWriter, value: Access) -> None: ...
def decode_access(reader: BinaryReader) -> Access: ...
def to_json_access(value: Access) -> Json: ...
def from_json_access(value: Json) -> Access: ...

"""Normalized storage space value."""
Space: typing.TypeAlias = (
    typing.Literal["local"]
    | typing.Literal["shared"]
    | typing.Literal["static"]
    | typing.Literal["frame"]
)

def encode_space(writer: BinaryWriter, value: Space) -> None: ...
def decode_space(reader: BinaryReader) -> Space: ...
def to_json_space(value: Space) -> Json: ...
def from_json_space(value: Json) -> Space: ...

@dataclass(frozen=True, slots=True)
class PlaceAmbient:
    """Ambient placement."""

    kind: typing.Literal["ambient"] = "ambient"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PlaceSpace:
    """Concrete storage space."""

    space: Space
    kind: typing.Literal["space"] = "space"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Normalized memory placement value."""
Place: typing.TypeAlias = PlaceAmbient | PlaceSpace

def encode_place(writer: BinaryWriter, value: Place) -> None: ...
def decode_place(reader: BinaryReader) -> Place: ...
def to_json_place(value: Place) -> Json: ...
def from_json_place(value: Json) -> Place: ...

@dataclass(frozen=True, slots=True)
class LifetimeStatic:
    """Static lifetime."""

    kind: typing.Literal["static"] = "static"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LifetimeFrame:
    """The enclosing frame's lifetime."""

    kind: typing.Literal["frame"] = "frame"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LifetimeSymbol:
    """Symbolic lifetime parameter or associated constant."""

    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    kind: typing.Literal["symbol"] = "symbol"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Normalized lifetime value."""
Lifetime: typing.TypeAlias = LifetimeStatic | LifetimeFrame | LifetimeSymbol

def encode_lifetime(writer: BinaryWriter, value: Lifetime) -> None: ...
def decode_lifetime(reader: BinaryReader) -> Lifetime: ...
def to_json_lifetime(value: Lifetime) -> Json: ...
def from_json_lifetime(value: Json) -> Lifetime: ...

@dataclass(frozen=True, slots=True)
class TypeReference:
    """One written reference to a type declaration before application."""

    # the referenced declaration symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeReference: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TypeReference: ...

def encode_type_reference(writer: BinaryWriter, value: TypeReference) -> None: ...
def decode_type_reference(reader: BinaryReader) -> TypeReference: ...
def to_json_type_reference(value: TypeReference) -> Json: ...
def from_json_type_reference(value: Json) -> TypeReference: ...

@dataclass(frozen=True, slots=True)
class GenericInstance:
    """One declaration applied to its complete positional arguments."""

    # the referenced declaration symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the complete positional argument list in declaration order
    arguments: TypeListId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> GenericInstance: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> GenericInstance: ...

def encode_generic_instance(writer: BinaryWriter, value: GenericInstance) -> None: ...
def decode_generic_instance(reader: BinaryReader) -> GenericInstance: ...
def to_json_generic_instance(value: GenericInstance) -> Json: ...
def from_json_generic_instance(value: Json) -> GenericInstance: ...

@dataclass(frozen=True, slots=True)
class TypeListId:
    """One interned list inside the owning module's type storage."""

    # the first element of the list
    start: int
    # the number of elements in the list
    count: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeListId: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TypeListId: ...

def encode_type_list_id(writer: BinaryWriter, value: TypeListId) -> None: ...
def decode_type_list_id(reader: BinaryReader) -> TypeListId: ...
def to_json_type_list_id(value: TypeListId) -> Json: ...
def from_json_type_list_id(value: Json) -> TypeListId: ...

@dataclass(frozen=True, slots=True)
class MemberType:
    """Member type selected from an owner type."""

    # the owner type
    owner: GlobalTypeId
    # the selected member key
    key: destack._generated.dir.symbol.key.StaticKey
    # the complete positional argument list applied to the member
    arguments: TypeListId
    # the declaring scope qualifying the projection
    qualifier: GlobalTypeId | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> MemberType: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> MemberType: ...

def encode_member_type(writer: BinaryWriter, value: MemberType) -> None: ...
def decode_member_type(reader: BinaryReader) -> MemberType: ...
def to_json_member_type(value: MemberType) -> Json: ...
def from_json_member_type(value: Json) -> MemberType: ...

@dataclass(frozen=True, slots=True)
class GlobalTypeId:
    """Global type id across modules."""

    # the module id of the global type
    module_id: destack._generated.source.file.model.module.ModuleId
    # the local id of the global type
    local_id: LocalTypeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> GlobalTypeId: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> GlobalTypeId: ...

def encode_global_type_id(writer: BinaryWriter, value: GlobalTypeId) -> None: ...
def decode_global_type_id(reader: BinaryReader) -> GlobalTypeId: ...
def to_json_global_type_id(value: GlobalTypeId) -> Json: ...
def from_json_global_type_id(value: Json) -> GlobalTypeId: ...

"""Unique identifier for a local type."""
LocalTypeId: typing.TypeAlias = int

def encode_local_type_id(writer: BinaryWriter, value: LocalTypeId) -> None: ...
def decode_local_type_id(reader: BinaryReader) -> LocalTypeId: ...
def to_json_local_type_id(value: LocalTypeId) -> Json: ...
def from_json_local_type_id(value: Json) -> LocalTypeId: ...

@dataclass(frozen=True, slots=True)
class EnumMemberType:
    """Singleton type of one enum member."""

    # the enum declaration instance
    owner: GlobalTypeId
    # the selected enum variant symbol
    member: destack._generated.dir.symbol.symbol.GlobalSymbolId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> EnumMemberType: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> EnumMemberType: ...

def encode_enum_member_type(writer: BinaryWriter, value: EnumMemberType) -> None: ...
def decode_enum_member_type(reader: BinaryReader) -> EnumMemberType: ...
def to_json_enum_member_type(value: EnumMemberType) -> Json: ...
def from_json_enum_member_type(value: Json) -> EnumMemberType: ...

@dataclass(frozen=True, slots=True)
class FormType:
    """Canonical memory or access form."""

    # the form constructor
    form: Form
    # the type carried by the form
    value: GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FormType: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FormType: ...

def encode_form_type(writer: BinaryWriter, value: FormType) -> None: ...
def decode_form_type(reader: BinaryReader) -> FormType: ...
def to_json_form_type(value: FormType) -> Json: ...
def from_json_form_type(value: Json) -> FormType: ...

@dataclass(frozen=True, slots=True)
class FormManaged:
    """Automatically managed runtime value, the unqualified `User`."""

    kind: typing.Literal["managed"] = "managed"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class FormOwned:
    """Owned value, like `^User`."""

    kind: typing.Literal["owned"] = "owned"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class FormBorrowed:
    """Borrowed value, like `&User`, `&readonly User`, or `&exclusive User`."""

    # the solved borrow lifetime singleton
    lifetime: GlobalTypeId
    # the solved borrow access singleton
    access: GlobalTypeId
    kind: typing.Literal["borrowed"] = "borrowed"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class FormRaw:
    """Raw pointer value, like `*User`."""

    kind: typing.Literal["raw"] = "raw"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class FormPlaced:
    """Placed value, like `local User` or `shared User`."""

    # the solved concrete or ambient place singleton
    place: GlobalTypeId
    kind: typing.Literal["placed"] = "placed"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class FormReadonly:
    """Readonly view, like `readonly User`."""

    kind: typing.Literal["readonly"] = "readonly"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Canonical memory or access form constructor."""
Form: typing.TypeAlias = (
    FormManaged | FormOwned | FormBorrowed | FormRaw | FormPlaced | FormReadonly
)

def encode_form(writer: BinaryWriter, value: Form) -> None: ...
def decode_form(reader: BinaryReader) -> Form: ...
def to_json_form(value: Form) -> Json: ...
def from_json_form(value: Json) -> Form: ...

@dataclass(frozen=True, slots=True)
class DynamicType:
    """Explicit runtime `Dynamic<T>` representation."""

    # the `Dynamic<T>` constraint
    constraint: GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DynamicType: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DynamicType: ...

def encode_dynamic_type(writer: BinaryWriter, value: DynamicType) -> None: ...
def decode_dynamic_type(reader: BinaryReader) -> DynamicType: ...
def to_json_dynamic_type(value: DynamicType) -> Json: ...
def from_json_dynamic_type(value: Json) -> DynamicType: ...

@dataclass(frozen=True, slots=True)
class TypeOperationStringMapping:
    """Compiler-known string mapping type, like `Uppercase<S>`."""

    # the string mapping operation
    mapping: StringMapping
    # the mapped string type
    target: GlobalTypeId
    kind: typing.Literal["stringMapping"] = "stringMapping"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeOperationConditional:
    """Conditional type expression, like `T extends string ? A : B`."""

    conditional: ConditionalType
    kind: typing.Literal["conditional"] = "conditional"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeOperationNarrow:
    """Runtime guard narrowing, like the true or false branch of `value is T`."""

    narrow: NarrowType
    kind: typing.Literal["narrow"] = "narrow"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeOperationMapped:
    """Mapped type expression, like `{ [K in keyof T]: T[K] }`."""

    mapped: MappedType
    kind: typing.Literal["mapped"] = "mapped"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeOperationIndex:
    """Indexed access type expression, like `User["name"]`."""

    index: IndexType
    kind: typing.Literal["index"] = "index"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeOperationTemplateLiteral:
    """Template literal type expression, like `` `get${Name}` ``."""

    template_literal: TemplateLiteralType
    kind: typing.Literal["templateLiteral"] = "templateLiteral"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeOperationInfer:
    """Type infer binding in a conditional type pattern, like `infer E`."""

    infer: InferType
    kind: typing.Literal["infer"] = "infer"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeOperationTypeOf:
    """Type query expression, like `typeof value`."""

    type_of: TypeOfType
    kind: typing.Literal["typeOf"] = "typeOf"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeOperationKeyOf:
    """`keyof T`."""

    key_of: UnaryType
    kind: typing.Literal["keyOf"] = "keyOf"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeOperationNoInfer:
    """Inference blocker like `NoInfer<T>`."""

    no_infer: UnaryType
    kind: typing.Literal["noInfer"] = "noInfer"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeOperationAwaited:
    """Awaited value type, like `Awaited<Promise<T>>`."""

    awaited: UnaryType
    kind: typing.Literal["awaited"] = "awaited"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeOperationTryOutput:
    """Try success projection like `value?` continuing evaluation."""

    # the tried value type
    value: GlobalTypeId
    kind: typing.Literal["tryOutput"] = "tryOutput"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeOperationTryResidual:
    """Try failure projection like `value?` propagating its residual."""

    # the tried value type
    value: GlobalTypeId
    kind: typing.Literal["tryResidual"] = "tryResidual"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeOperationStaticBinary:
    """Static binary operation like `N * 2` or `Mode == "inline"`."""

    static_binary: StaticBinaryType
    kind: typing.Literal["staticBinary"] = "staticBinary"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeOperationStaticUnary:
    """Static unary operation like `!Wide`."""

    static_unary: StaticUnaryType
    kind: typing.Literal["staticUnary"] = "staticUnary"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Type-level operation preserved by check."""
TypeOperation: typing.TypeAlias = (
    TypeOperationStringMapping
    | TypeOperationConditional
    | TypeOperationNarrow
    | TypeOperationMapped
    | TypeOperationIndex
    | TypeOperationTemplateLiteral
    | TypeOperationInfer
    | TypeOperationTypeOf
    | TypeOperationKeyOf
    | TypeOperationNoInfer
    | TypeOperationAwaited
    | TypeOperationTryOutput
    | TypeOperationTryResidual
    | TypeOperationStaticBinary
    | TypeOperationStaticUnary
)

def encode_type_operation(writer: BinaryWriter, value: TypeOperation) -> None: ...
def decode_type_operation(reader: BinaryReader) -> TypeOperation: ...
def to_json_type_operation(value: TypeOperation) -> Json: ...
def from_json_type_operation(value: Json) -> TypeOperation: ...

"""Compiler-provided string mapping."""
StringMapping: typing.TypeAlias = (
    typing.Literal["uppercase"]
    | typing.Literal["lowercase"]
    | typing.Literal["capitalize"]
    | typing.Literal["uncapitalize"]
)

def encode_string_mapping(writer: BinaryWriter, value: StringMapping) -> None: ...
def decode_string_mapping(reader: BinaryReader) -> StringMapping: ...
def to_json_string_mapping(value: StringMapping) -> Json: ...
def from_json_string_mapping(value: Json) -> StringMapping: ...

@dataclass(frozen=True, slots=True)
class ConditionalType:
    """A conditional type."""

    # the left operand
    left: GlobalTypeId
    # the right operand
    right: GlobalTypeId
    # the type selected when the condition holds
    then_type: GlobalTypeId
    # the type selected when the condition does not hold
    else_type: GlobalTypeId
    # whether the conditional distributes over union-valued left operands
    is_distributive: bool

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ConditionalType: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ConditionalType: ...

def encode_conditional_type(writer: BinaryWriter, value: ConditionalType) -> None: ...
def decode_conditional_type(reader: BinaryReader) -> ConditionalType: ...
def to_json_conditional_type(value: ConditionalType) -> Json: ...
def from_json_conditional_type(value: Json) -> ConditionalType: ...

@dataclass(frozen=True, slots=True)
class NarrowType:
    """A runtime guard narrowing applied to one source type."""

    # the source type being narrowed
    source: GlobalTypeId
    # the runtime-tested target type
    target: GlobalTypeId
    # whether matching arms are kept or removed
    is_positive: bool

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> NarrowType: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> NarrowType: ...

def encode_narrow_type(writer: BinaryWriter, value: NarrowType) -> None: ...
def decode_narrow_type(reader: BinaryReader) -> NarrowType: ...
def to_json_narrow_type(value: NarrowType) -> Json: ...
def from_json_narrow_type(value: Json) -> NarrowType: ...

@dataclass(frozen=True, slots=True)
class MappedType:
    """A mapped type."""

    # the mapped parameter
    parameter: MappedTypeParameter
    # the mapped modifiers
    modifiers: MappedTypeModifiers
    # the mapped value type
    value: GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> MappedType: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> MappedType: ...

def encode_mapped_type(writer: BinaryWriter, value: MappedType) -> None: ...
def decode_mapped_type(reader: BinaryReader) -> MappedType: ...
def to_json_mapped_type(value: MappedType) -> Json: ...
def from_json_mapped_type(value: Json) -> MappedType: ...

@dataclass(frozen=True, slots=True)
class MappedTypeParameter:
    """A mapped-type parameter."""

    # the parameter name like `K`
    name: destack._generated.core.string.StringId
    # the binder's generic parameter
    parameter: destack._generated.dir.type.generic.GlobalGenericParameterId
    # the constraint type like `keyof T`
    constraint: GlobalTypeId
    # the optional key remap like `as Foo<K>`
    key_remap: GlobalTypeId | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> MappedTypeParameter: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> MappedTypeParameter: ...

def encode_mapped_type_parameter(
    writer: BinaryWriter, value: MappedTypeParameter
) -> None: ...
def decode_mapped_type_parameter(reader: BinaryReader) -> MappedTypeParameter: ...
def to_json_mapped_type_parameter(value: MappedTypeParameter) -> Json: ...
def from_json_mapped_type_parameter(value: Json) -> MappedTypeParameter: ...

@dataclass(frozen=True, slots=True)
class MappedTypeModifiers:
    """Mapped-type modifiers."""

    # the readonly modifier
    readonly: destack._generated.dir.tree.type.MappedTypeModifier
    # the optional modifier
    optional: destack._generated.dir.tree.type.MappedTypeModifier

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> MappedTypeModifiers: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> MappedTypeModifiers: ...

def encode_mapped_type_modifiers(
    writer: BinaryWriter, value: MappedTypeModifiers
) -> None: ...
def decode_mapped_type_modifiers(reader: BinaryReader) -> MappedTypeModifiers: ...
def to_json_mapped_type_modifiers(value: MappedTypeModifiers) -> Json: ...
def from_json_mapped_type_modifiers(value: Json) -> MappedTypeModifiers: ...

@dataclass(frozen=True, slots=True)
class IndexType:
    """Indexed access type."""

    # the indexed type
    left: GlobalTypeId
    # the index type
    index: GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> IndexType: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> IndexType: ...

def encode_index_type(writer: BinaryWriter, value: IndexType) -> None: ...
def decode_index_type(reader: BinaryReader) -> IndexType: ...
def to_json_index_type(value: IndexType) -> Json: ...
def from_json_index_type(value: Json) -> IndexType: ...

@dataclass(frozen=True, slots=True)
class TemplateLiteralType:
    """A template literal type."""

    # the literal string segment list
    strings: TypeListId
    # the interpolated type span list
    spans: TypeListId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TemplateLiteralType: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TemplateLiteralType: ...

def encode_template_literal_type(
    writer: BinaryWriter, value: TemplateLiteralType
) -> None: ...
def decode_template_literal_type(reader: BinaryReader) -> TemplateLiteralType: ...
def to_json_template_literal_type(value: TemplateLiteralType) -> Json: ...
def from_json_template_literal_type(value: Json) -> TemplateLiteralType: ...

@dataclass(frozen=True, slots=True)
class InferType:
    """An infer binding inside a conditional type pattern."""

    # the inferred binding name
    name: destack._generated.core.string.StringId | None
    # the inferred binding symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId | None
    # the optional inferred constraint
    constraint: GlobalTypeId | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> InferType: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> InferType: ...

def encode_infer_type(writer: BinaryWriter, value: InferType) -> None: ...
def decode_infer_type(reader: BinaryReader) -> InferType: ...
def to_json_infer_type(value: InferType) -> Json: ...
def from_json_infer_type(value: Json) -> InferType: ...

@dataclass(frozen=True, slots=True)
class TypeOfType:
    """Type query expression, like `typeof value`."""

    # the queried value reference
    value: destack._generated.dir.tree.node.GlobalNodeIdAny

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeOfType: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TypeOfType: ...

def encode_type_of_type(writer: BinaryWriter, value: TypeOfType) -> None: ...
def decode_type_of_type(reader: BinaryReader) -> TypeOfType: ...
def to_json_type_of_type(value: TypeOfType) -> Json: ...
def from_json_type_of_type(value: Json) -> TypeOfType: ...

@dataclass(frozen=True, slots=True)
class UnaryType:
    """A unary type operator."""

    # the target type
    target: GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> UnaryType: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> UnaryType: ...

def encode_unary_type(writer: BinaryWriter, value: UnaryType) -> None: ...
def decode_unary_type(reader: BinaryReader) -> UnaryType: ...
def to_json_unary_type(value: UnaryType) -> Json: ...
def from_json_unary_type(value: Json) -> UnaryType: ...

@dataclass(frozen=True, slots=True)
class StaticBinaryType:
    """One static binary operation over singleton operands."""

    # the applied operator
    operator: StaticBinaryOperator
    # the left operand
    left: GlobalTypeId
    # the right operand
    right: GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> StaticBinaryType: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> StaticBinaryType: ...

def encode_static_binary_type(
    writer: BinaryWriter, value: StaticBinaryType
) -> None: ...
def decode_static_binary_type(reader: BinaryReader) -> StaticBinaryType: ...
def to_json_static_binary_type(value: StaticBinaryType) -> Json: ...
def from_json_static_binary_type(value: Json) -> StaticBinaryType: ...

"""One static binary operator."""
StaticBinaryOperator: typing.TypeAlias = (
    typing.Literal["add"]
    | typing.Literal["subtract"]
    | typing.Literal["multiply"]
    | typing.Literal["divide"]
    | typing.Literal["remainder"]
    | typing.Literal["exponent"]
    | typing.Literal["shiftLeft"]
    | typing.Literal["shiftRight"]
    | typing.Literal["unsignedShiftRight"]
    | typing.Literal["bitwiseAnd"]
    | typing.Literal["bitwiseXor"]
    | typing.Literal["bitwiseOr"]
    | typing.Literal["equal"]
    | typing.Literal["equalStrict"]
    | typing.Literal["notEqual"]
    | typing.Literal["notEqualStrict"]
    | typing.Literal["lessThan"]
    | typing.Literal["lessThanOrEqual"]
    | typing.Literal["greaterThan"]
    | typing.Literal["greaterThanOrEqual"]
    | typing.Literal["and"]
    | typing.Literal["or"]
)

def encode_static_binary_operator(
    writer: BinaryWriter, value: StaticBinaryOperator
) -> None: ...
def decode_static_binary_operator(reader: BinaryReader) -> StaticBinaryOperator: ...
def to_json_static_binary_operator(value: StaticBinaryOperator) -> Json: ...
def from_json_static_binary_operator(value: Json) -> StaticBinaryOperator: ...

@dataclass(frozen=True, slots=True)
class StaticUnaryType:
    """One static unary operation over one singleton operand."""

    # the applied operator
    operator: StaticUnaryOperator
    # the operand
    target: GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> StaticUnaryType: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> StaticUnaryType: ...

def encode_static_unary_type(writer: BinaryWriter, value: StaticUnaryType) -> None: ...
def decode_static_unary_type(reader: BinaryReader) -> StaticUnaryType: ...
def to_json_static_unary_type(value: StaticUnaryType) -> Json: ...
def from_json_static_unary_type(value: Json) -> StaticUnaryType: ...

"""One static unary operator."""
StaticUnaryOperator: typing.TypeAlias = (
    typing.Literal["not"] | typing.Literal["negate"] | typing.Literal["bitwiseNot"]
)

def encode_static_unary_operator(
    writer: BinaryWriter, value: StaticUnaryOperator
) -> None: ...
def decode_static_unary_operator(reader: BinaryReader) -> StaticUnaryOperator: ...
def to_json_static_unary_operator(value: StaticUnaryOperator) -> Json: ...
def from_json_static_unary_operator(value: Json) -> StaticUnaryOperator: ...

@dataclass(frozen=True, slots=True)
class ArrayType:
    """Homogeneous array type."""

    # the element type
    element: GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ArrayType: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ArrayType: ...

def encode_array_type(writer: BinaryWriter, value: ArrayType) -> None: ...
def decode_array_type(reader: BinaryReader) -> ArrayType: ...
def to_json_array_type(value: ArrayType) -> Json: ...
def from_json_array_type(value: Json) -> ArrayType: ...

@dataclass(frozen=True, slots=True)
class FixedArrayType:
    """A fixed-length array type."""

    # the element type
    element: GlobalTypeId
    # the static array length singleton
    count: GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FixedArrayType: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FixedArrayType: ...

def encode_fixed_array_type(writer: BinaryWriter, value: FixedArrayType) -> None: ...
def decode_fixed_array_type(reader: BinaryReader) -> FixedArrayType: ...
def to_json_fixed_array_type(value: FixedArrayType) -> Json: ...
def from_json_fixed_array_type(value: Json) -> FixedArrayType: ...

@dataclass(frozen=True, slots=True)
class RangeType:
    """Compact discrete scalar interval type."""

    # the inclusive lower bound
    start: destack._generated.dir.tree.literal.ScalarLiteral | None
    # the upper bound
    end: destack._generated.dir.tree.literal.ScalarLiteral | None
    # whether the upper bound is included
    is_inclusive: bool

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> RangeType: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> RangeType: ...

def encode_range_type(writer: BinaryWriter, value: RangeType) -> None: ...
def decode_range_type(reader: BinaryReader) -> RangeType: ...
def to_json_range_type(value: RangeType) -> Json: ...
def from_json_range_type(value: Json) -> RangeType: ...

@dataclass(frozen=True, slots=True)
class SliceType:
    """Runtime-length homogeneous view type."""

    # the element type
    element: GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SliceType: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SliceType: ...

def encode_slice_type(writer: BinaryWriter, value: SliceType) -> None: ...
def decode_slice_type(reader: BinaryReader) -> SliceType: ...
def to_json_slice_type(value: SliceType) -> Json: ...
def from_json_slice_type(value: Json) -> SliceType: ...

@dataclass(frozen=True, slots=True)
class TupleType:
    """A tuple type."""

    # the tuple source form
    form: TupleForm
    # the tuple element list
    elements: TypeListId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TupleType: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TupleType: ...

def encode_tuple_type(writer: BinaryWriter, value: TupleType) -> None: ...
def decode_tuple_type(reader: BinaryReader) -> TupleType: ...
def to_json_tuple_type(value: TupleType) -> Json: ...
def from_json_tuple_type(value: Json) -> TupleType: ...

"""The source form of a tuple type."""
TupleForm: typing.TypeAlias = typing.Literal["tuple"] | typing.Literal["array"]

def encode_tuple_form(writer: BinaryWriter, value: TupleForm) -> None: ...
def decode_tuple_form(reader: BinaryReader) -> TupleForm: ...
def to_json_tuple_form(value: TupleForm) -> Json: ...
def from_json_tuple_form(value: Json) -> TupleForm: ...

@dataclass(frozen=True, slots=True)
class ShapeType:
    """A structural object shape type."""

    # the shape field list
    fields: TypeListId
    # the call signature list
    call_signatures: TypeListId
    # the construct signature list
    construct_signatures: TypeListId
    # the index signature list
    index_signatures: TypeListId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ShapeType: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ShapeType: ...

def encode_shape_type(writer: BinaryWriter, value: ShapeType) -> None: ...
def decode_shape_type(reader: BinaryReader) -> ShapeType: ...
def to_json_shape_type(value: ShapeType) -> Json: ...
def from_json_shape_type(value: Json) -> ShapeType: ...

@dataclass(frozen=True, slots=True)
class FunctionSignatureType:
    """A function signature type."""

    # the function asynchrony
    asynchrony: destack._generated.dir.tree.node.Asynchrony
    # the template that owns this signature's generic parameters
    template: destack._generated.dir.type.generic.GlobalGenericTemplateId | None
    # the optional `this` parameter type
    this_parameter: GlobalTypeId | None
    # the runtime parameter list
    parameters: TypeListId
    # the optional return type
    return_type: GlobalTypeId | None
    # whether this is a generator function
    is_generator: bool

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionSignatureType: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FunctionSignatureType: ...

def encode_function_signature_type(
    writer: BinaryWriter, value: FunctionSignatureType
) -> None: ...
def decode_function_signature_type(reader: BinaryReader) -> FunctionSignatureType: ...
def to_json_function_signature_type(value: FunctionSignatureType) -> Json: ...
def from_json_function_signature_type(value: Json) -> FunctionSignatureType: ...

@dataclass(frozen=True, slots=True)
class FunctionType:
    """A fat callable value with a function signature and captured environment."""

    # the function signature
    signature: GlobalTypeId
    # the captured environment type
    environment: GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionType: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FunctionType: ...

def encode_function_type(writer: BinaryWriter, value: FunctionType) -> None: ...
def decode_function_type(reader: BinaryReader) -> FunctionType: ...
def to_json_function_type(value: FunctionType) -> Json: ...
def from_json_function_type(value: Json) -> FunctionType: ...

@dataclass(frozen=True, slots=True)
class FunctionPointerType:
    """A thin callable value with no captured environment."""

    # the function signature
    signature: GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionPointerType: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FunctionPointerType: ...

def encode_function_pointer_type(
    writer: BinaryWriter, value: FunctionPointerType
) -> None: ...
def decode_function_pointer_type(reader: BinaryReader) -> FunctionPointerType: ...
def to_json_function_pointer_type(value: FunctionPointerType) -> Json: ...
def from_json_function_pointer_type(value: Json) -> FunctionPointerType: ...

@dataclass(frozen=True, slots=True)
class UnionType:
    """A union type."""

    # the union element list
    elements: TypeListId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> UnionType: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> UnionType: ...

def encode_union_type(writer: BinaryWriter, value: UnionType) -> None: ...
def decode_union_type(reader: BinaryReader) -> UnionType: ...
def to_json_union_type(value: UnionType) -> Json: ...
def from_json_union_type(value: Json) -> UnionType: ...

@dataclass(frozen=True, slots=True)
class IntersectionType:
    """An intersection type."""

    # the intersection element list
    elements: TypeListId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> IntersectionType: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> IntersectionType: ...

def encode_intersection_type(writer: BinaryWriter, value: IntersectionType) -> None: ...
def decode_intersection_type(reader: BinaryReader) -> IntersectionType: ...
def to_json_intersection_type(value: IntersectionType) -> Json: ...
def from_json_intersection_type(value: Json) -> IntersectionType: ...

@dataclass(frozen=True, slots=True)
class TypeVariable:
    """One open inference variable."""

    variable: TypeVariableId
    kind: typing.Literal["variable"] = "variable"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeError:
    """Error type that could not be resolved."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeNever:
    """Never type `never`."""

    kind: typing.Literal["never"] = "never"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeAny:
    """TypeScript `any` compatibility marker."""

    kind: typing.Literal["any"] = "any"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeUnknown:
    """Unknown type."""

    kind: typing.Literal["unknown"] = "unknown"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeVoid:
    """Void type."""

    kind: typing.Literal["void"] = "void"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeNull:
    """Null type and value."""

    kind: typing.Literal["null"] = "null"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeUndefined:
    """Undefined type and value."""

    kind: typing.Literal["undefined"] = "undefined"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeObject:
    """TypeScript `object` constraint."""

    kind: typing.Literal["object"] = "object"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypePrimitive:
    """Primitive type, like `string` or `int32`."""

    primitive: destack._generated.dir.type.primitive.PrimitiveType
    kind: typing.Literal["primitive"] = "primitive"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeLiteral:
    """Scalar literal type, like `"id"` or `42`."""

    literal: destack._generated.dir.tree.literal.ScalarLiteral
    kind: typing.Literal["literal"] = "literal"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeKey:
    """Singleton property key type, like `Symbol.for("id")`."""

    key: destack._generated.dir.symbol.key.StaticKey
    kind: typing.Literal["key"] = "key"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeMemory:
    """Singleton type of one normalized memory value."""

    memory: MemoryLiteral
    kind: typing.Literal["memory"] = "memory"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeStatic:
    """Singleton type of one committed static value."""

    static: destack._generated.dir.tree.static.GlobalStaticId
    kind: typing.Literal["static"] = "static"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeIntrinsic:
    """Compiler intrinsic type body."""

    kind: typing.Literal["intrinsic"] = "intrinsic"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeErased:
    """Erased generic argument captured by a runtime head test."""

    erased: destack._generated.dir.type.generic.GlobalGenericParameterId
    kind: typing.Literal["erased"] = "erased"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeParameter:
    """Generic parameter, like the `T` in `class Box<T>`."""

    parameter: destack._generated.dir.type.generic.GlobalGenericParameterId
    kind: typing.Literal["parameter"] = "parameter"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeReferenceVariant:
    """Type declaration reference before application, like `Box` in `Box.empty`."""

    reference: TypeReference
    kind: typing.Literal["reference"] = "reference"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeInstance:
    """Applied type declaration, like `User` or `Map<string, User>`."""

    instance: GenericInstance
    kind: typing.Literal["instance"] = "instance"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeThis:
    """This type in a method signature, like `this` in `clone(): this`."""

    kind: typing.Literal["this"] = "this"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeMember:
    """Member type selected from an owner type, like `T.Output`."""

    member: MemberType
    kind: typing.Literal["member"] = "member"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeEnumMember:
    """Singleton enum member type, like `Mode.Read`."""

    enum_member: EnumMemberType
    kind: typing.Literal["enumMember"] = "enumMember"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeForm:
    """Canonical memory or access form, like `^User` or `&exclusive User`."""

    form: FormType
    kind: typing.Literal["form"] = "form"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeDynamic:
    """Explicit runtime `Dynamic<T>` representation, like `Dynamic<Printable>`."""

    dynamic: DynamicType
    kind: typing.Literal["dynamic"] = "dynamic"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeOperationVariant:
    """Type-level operation preserved by check."""

    operation: TypeOperation
    kind: typing.Literal["operation"] = "operation"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeArray:
    """Homogeneous array type, like `int32[]`."""

    array: ArrayType
    kind: typing.Literal["array"] = "array"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeFixedArray:
    """Fixed-length array type, like `[uint8; 4]`."""

    fixed_array: FixedArrayType
    kind: typing.Literal["fixedArray"] = "fixedArray"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeRange:
    """Compact scalar interval type, like `0..10`."""

    range: RangeType
    kind: typing.Literal["range"] = "range"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeSlice:
    """Runtime-length homogeneous view type, like `[uint8]`."""

    slice: SliceType
    kind: typing.Literal["slice"] = "slice"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeTuple:
    """Tuple type, like `(string, int32)`."""

    tuple: TupleType
    kind: typing.Literal["tuple"] = "tuple"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeShape:
    """Structural object shape type, like `{ name: string }`."""

    shape: ShapeType
    kind: typing.Literal["shape"] = "shape"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeFunctionSignature:
    """Function signature type, like `(value: int32) => string`."""

    function_signature: FunctionSignatureType
    kind: typing.Literal["functionSignature"] = "functionSignature"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeFunction:
    """Fat callable value with an explicit captured environment."""

    function: FunctionType
    kind: typing.Literal["function"] = "function"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeFunctionPointer:
    """Thin callable value with no captured environment."""

    function_pointer: FunctionPointerType
    kind: typing.Literal["functionPointer"] = "functionPointer"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeUnion:
    """Union type `A | B | C`."""

    union: UnionType
    kind: typing.Literal["union"] = "union"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeIntersection:
    """Intersection type `A & B & C`."""

    intersection: IntersectionType
    kind: typing.Literal["intersection"] = "intersection"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""A canonical solved type."""
Type: typing.TypeAlias = (
    TypeVariable
    | TypeError
    | TypeNever
    | TypeAny
    | TypeUnknown
    | TypeVoid
    | TypeNull
    | TypeUndefined
    | TypeObject
    | TypePrimitive
    | TypeLiteral
    | TypeKey
    | TypeMemory
    | TypeStatic
    | TypeIntrinsic
    | TypeErased
    | TypeParameter
    | TypeReferenceVariant
    | TypeInstance
    | TypeThis
    | TypeMember
    | TypeEnumMember
    | TypeForm
    | TypeDynamic
    | TypeOperationVariant
    | TypeArray
    | TypeFixedArray
    | TypeRange
    | TypeSlice
    | TypeTuple
    | TypeShape
    | TypeFunctionSignature
    | TypeFunction
    | TypeFunctionPointer
    | TypeUnion
    | TypeIntersection
)

def encode_type(writer: BinaryWriter, value: Type) -> None: ...
def decode_type(reader: BinaryReader) -> Type: ...
def to_json_type(value: Type) -> Json: ...
def from_json_type(value: Json) -> Type: ...

"""The symbolic leaf kinds contained in one type graph, computed once at intern time."""
TypeFlags: typing.TypeAlias = int

def encode_type_flags(writer: BinaryWriter, value: TypeFlags) -> None: ...
def decode_type_flags(reader: BinaryReader) -> TypeFlags: ...
def to_json_type_flags(value: TypeFlags) -> Json: ...
def from_json_type_flags(value: Json) -> TypeFlags: ...

__all__ = [
    "TypeVariableId",
    "encode_type_variable_id",
    "decode_type_variable_id",
    "to_json_type_variable_id",
    "from_json_type_variable_id",
    "MemoryLiteral",
    "encode_memory_literal",
    "decode_memory_literal",
    "to_json_memory_literal",
    "from_json_memory_literal",
    "MemoryLiteralAccess",
    "MemoryLiteralSpace",
    "MemoryLiteralPlace",
    "MemoryLiteralLifetime",
    "Access",
    "encode_access",
    "decode_access",
    "to_json_access",
    "from_json_access",
    "Space",
    "encode_space",
    "decode_space",
    "to_json_space",
    "from_json_space",
    "Place",
    "encode_place",
    "decode_place",
    "to_json_place",
    "from_json_place",
    "PlaceAmbient",
    "PlaceSpace",
    "Lifetime",
    "encode_lifetime",
    "decode_lifetime",
    "to_json_lifetime",
    "from_json_lifetime",
    "LifetimeStatic",
    "LifetimeFrame",
    "LifetimeSymbol",
    "TypeReference",
    "encode_type_reference",
    "decode_type_reference",
    "to_json_type_reference",
    "from_json_type_reference",
    "GenericInstance",
    "encode_generic_instance",
    "decode_generic_instance",
    "to_json_generic_instance",
    "from_json_generic_instance",
    "TypeListId",
    "encode_type_list_id",
    "decode_type_list_id",
    "to_json_type_list_id",
    "from_json_type_list_id",
    "MemberType",
    "encode_member_type",
    "decode_member_type",
    "to_json_member_type",
    "from_json_member_type",
    "GlobalTypeId",
    "encode_global_type_id",
    "decode_global_type_id",
    "to_json_global_type_id",
    "from_json_global_type_id",
    "LocalTypeId",
    "encode_local_type_id",
    "decode_local_type_id",
    "to_json_local_type_id",
    "from_json_local_type_id",
    "EnumMemberType",
    "encode_enum_member_type",
    "decode_enum_member_type",
    "to_json_enum_member_type",
    "from_json_enum_member_type",
    "FormType",
    "encode_form_type",
    "decode_form_type",
    "to_json_form_type",
    "from_json_form_type",
    "Form",
    "encode_form",
    "decode_form",
    "to_json_form",
    "from_json_form",
    "FormManaged",
    "FormOwned",
    "FormBorrowed",
    "FormRaw",
    "FormPlaced",
    "FormReadonly",
    "DynamicType",
    "encode_dynamic_type",
    "decode_dynamic_type",
    "to_json_dynamic_type",
    "from_json_dynamic_type",
    "TypeOperation",
    "encode_type_operation",
    "decode_type_operation",
    "to_json_type_operation",
    "from_json_type_operation",
    "TypeOperationStringMapping",
    "TypeOperationConditional",
    "TypeOperationNarrow",
    "TypeOperationMapped",
    "TypeOperationIndex",
    "TypeOperationTemplateLiteral",
    "TypeOperationInfer",
    "TypeOperationTypeOf",
    "TypeOperationKeyOf",
    "TypeOperationNoInfer",
    "TypeOperationAwaited",
    "TypeOperationTryOutput",
    "TypeOperationTryResidual",
    "TypeOperationStaticBinary",
    "TypeOperationStaticUnary",
    "StringMapping",
    "encode_string_mapping",
    "decode_string_mapping",
    "to_json_string_mapping",
    "from_json_string_mapping",
    "ConditionalType",
    "encode_conditional_type",
    "decode_conditional_type",
    "to_json_conditional_type",
    "from_json_conditional_type",
    "NarrowType",
    "encode_narrow_type",
    "decode_narrow_type",
    "to_json_narrow_type",
    "from_json_narrow_type",
    "MappedType",
    "encode_mapped_type",
    "decode_mapped_type",
    "to_json_mapped_type",
    "from_json_mapped_type",
    "MappedTypeParameter",
    "encode_mapped_type_parameter",
    "decode_mapped_type_parameter",
    "to_json_mapped_type_parameter",
    "from_json_mapped_type_parameter",
    "MappedTypeModifiers",
    "encode_mapped_type_modifiers",
    "decode_mapped_type_modifiers",
    "to_json_mapped_type_modifiers",
    "from_json_mapped_type_modifiers",
    "IndexType",
    "encode_index_type",
    "decode_index_type",
    "to_json_index_type",
    "from_json_index_type",
    "TemplateLiteralType",
    "encode_template_literal_type",
    "decode_template_literal_type",
    "to_json_template_literal_type",
    "from_json_template_literal_type",
    "InferType",
    "encode_infer_type",
    "decode_infer_type",
    "to_json_infer_type",
    "from_json_infer_type",
    "TypeOfType",
    "encode_type_of_type",
    "decode_type_of_type",
    "to_json_type_of_type",
    "from_json_type_of_type",
    "UnaryType",
    "encode_unary_type",
    "decode_unary_type",
    "to_json_unary_type",
    "from_json_unary_type",
    "StaticBinaryType",
    "encode_static_binary_type",
    "decode_static_binary_type",
    "to_json_static_binary_type",
    "from_json_static_binary_type",
    "StaticBinaryOperator",
    "encode_static_binary_operator",
    "decode_static_binary_operator",
    "to_json_static_binary_operator",
    "from_json_static_binary_operator",
    "StaticUnaryType",
    "encode_static_unary_type",
    "decode_static_unary_type",
    "to_json_static_unary_type",
    "from_json_static_unary_type",
    "StaticUnaryOperator",
    "encode_static_unary_operator",
    "decode_static_unary_operator",
    "to_json_static_unary_operator",
    "from_json_static_unary_operator",
    "ArrayType",
    "encode_array_type",
    "decode_array_type",
    "to_json_array_type",
    "from_json_array_type",
    "FixedArrayType",
    "encode_fixed_array_type",
    "decode_fixed_array_type",
    "to_json_fixed_array_type",
    "from_json_fixed_array_type",
    "RangeType",
    "encode_range_type",
    "decode_range_type",
    "to_json_range_type",
    "from_json_range_type",
    "SliceType",
    "encode_slice_type",
    "decode_slice_type",
    "to_json_slice_type",
    "from_json_slice_type",
    "TupleType",
    "encode_tuple_type",
    "decode_tuple_type",
    "to_json_tuple_type",
    "from_json_tuple_type",
    "TupleForm",
    "encode_tuple_form",
    "decode_tuple_form",
    "to_json_tuple_form",
    "from_json_tuple_form",
    "ShapeType",
    "encode_shape_type",
    "decode_shape_type",
    "to_json_shape_type",
    "from_json_shape_type",
    "FunctionSignatureType",
    "encode_function_signature_type",
    "decode_function_signature_type",
    "to_json_function_signature_type",
    "from_json_function_signature_type",
    "FunctionType",
    "encode_function_type",
    "decode_function_type",
    "to_json_function_type",
    "from_json_function_type",
    "FunctionPointerType",
    "encode_function_pointer_type",
    "decode_function_pointer_type",
    "to_json_function_pointer_type",
    "from_json_function_pointer_type",
    "UnionType",
    "encode_union_type",
    "decode_union_type",
    "to_json_union_type",
    "from_json_union_type",
    "IntersectionType",
    "encode_intersection_type",
    "decode_intersection_type",
    "to_json_intersection_type",
    "from_json_intersection_type",
    "Type",
    "encode_type",
    "decode_type",
    "to_json_type",
    "from_json_type",
    "TypeVariable",
    "TypeError",
    "TypeNever",
    "TypeAny",
    "TypeUnknown",
    "TypeVoid",
    "TypeNull",
    "TypeUndefined",
    "TypeObject",
    "TypePrimitive",
    "TypeLiteral",
    "TypeKey",
    "TypeMemory",
    "TypeStatic",
    "TypeIntrinsic",
    "TypeErased",
    "TypeParameter",
    "TypeReferenceVariant",
    "TypeInstance",
    "TypeThis",
    "TypeMember",
    "TypeEnumMember",
    "TypeForm",
    "TypeDynamic",
    "TypeOperationVariant",
    "TypeArray",
    "TypeFixedArray",
    "TypeRange",
    "TypeSlice",
    "TypeTuple",
    "TypeShape",
    "TypeFunctionSignature",
    "TypeFunction",
    "TypeFunctionPointer",
    "TypeUnion",
    "TypeIntersection",
    "TypeFlags",
    "encode_type_flags",
    "decode_type_flags",
    "to_json_type_flags",
    "from_json_type_flags",
]
