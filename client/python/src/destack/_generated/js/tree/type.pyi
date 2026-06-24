# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.string
import destack._generated.js.tree.argument
import destack._generated.js.tree.function
import destack._generated.js.tree.key
import destack._generated.js.tree.literal
import destack._generated.js.tree.node
import destack._generated.js.tree.path

@dataclass(frozen=True, slots=True)
class TypeExpressionScalar:
    """Scalar type literal."""

    scalar: TypeLiteral
    kind: typing.Literal["scalar"] = "scalar"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionThis:
    """This type."""

    kind: typing.Literal["this"] = "this"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionPath:
    """Path to something."""

    path: destack._generated.js.tree.path.Path
    generic_arguments: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["path"] = "path"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionReadonly:
    """`readonly T`."""

    target_type: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["readonly"] = "readonly"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionKeyOf:
    """`keyof T`."""

    target_type: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["keyOf"] = "keyOf"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionMust:
    """`T!`."""

    target_type: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["must"] = "must"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionNot:
    """`!T`."""

    target_type: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["not"] = "not"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionExtends:
    """`T extends U`."""

    left: destack._generated.js.tree.node.LocalNodeId
    right: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["extends"] = "extends"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionImplements:
    """`T implements U`."""

    left: destack._generated.js.tree.node.LocalNodeId
    right: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["implements"] = "implements"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionConditional:
    """Conditional type."""

    left: destack._generated.js.tree.node.LocalNodeId
    right: destack._generated.js.tree.node.LocalNodeId
    then_type: destack._generated.js.tree.node.LocalNodeId
    else_type: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["conditional"] = "conditional"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionMapped:
    """Mapped type."""

    parameter: TypeMappedParameter
    modifiers: TypeMappedModifiers
    value: destack._generated.js.tree.node.LocalNodeId | None
    kind: typing.Literal["mapped"] = "mapped"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionIndex:
    """Index access type."""

    left: destack._generated.js.tree.node.LocalNodeId
    index: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["index"] = "index"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionTemplateLiteral:
    """Template literal type."""

    template_literal: TypeTemplateLiteral
    kind: typing.Literal["templateLiteral"] = "templateLiteral"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionImport:
    """Import type."""

    target: destack._generated.core.string.StringId
    qualifier: destack._generated.js.tree.path.Path | None
    generic_arguments: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["import"] = "import"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionInfer:
    """Infer type binding."""

    name: destack._generated.core.string.StringId
    constraint: destack._generated.js.tree.node.LocalNodeId | None
    kind: typing.Literal["infer"] = "infer"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionArray:
    """Array type `T[]`."""

    element: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["array"] = "array"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionTuple:
    """Tuple type `[T1, T2, ...]`."""

    elements: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["tuple"] = "tuple"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionObject:
    """Object type `{ a: T1, b: T2, ... }`."""

    members: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["object"] = "object"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionUnion:
    """Union type `A | B | C`."""

    elements: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["union"] = "union"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionIntersection:
    """Intersection type `A & B & C`."""

    elements: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["intersection"] = "intersection"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionFunctionTypeDeclaration:
    """Function type declaration."""

    function_type_declaration: FunctionTypeDeclaration
    kind: typing.Literal["functionTypeDeclaration"] = "functionTypeDeclaration"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionConstructorTypeDeclaration:
    """Constructor type declaration."""

    constructor_type_declaration: ConstructorTypeDeclaration
    kind: typing.Literal["constructorTypeDeclaration"] = "constructorTypeDeclaration"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionError:
    """Error type that could not be resolved."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""A TypeExpression is a TypeScript type expression."""
TypeExpression: typing.TypeAlias = (
    TypeExpressionScalar
    | TypeExpressionThis
    | TypeExpressionPath
    | TypeExpressionReadonly
    | TypeExpressionKeyOf
    | TypeExpressionMust
    | TypeExpressionNot
    | TypeExpressionExtends
    | TypeExpressionImplements
    | TypeExpressionConditional
    | TypeExpressionMapped
    | TypeExpressionIndex
    | TypeExpressionTemplateLiteral
    | TypeExpressionImport
    | TypeExpressionInfer
    | TypeExpressionArray
    | TypeExpressionTuple
    | TypeExpressionObject
    | TypeExpressionUnion
    | TypeExpressionIntersection
    | TypeExpressionFunctionTypeDeclaration
    | TypeExpressionConstructorTypeDeclaration
    | TypeExpressionError
)

def encode_type_expression(writer: BinaryWriter, value: TypeExpression) -> None: ...
def decode_type_expression(reader: BinaryReader) -> TypeExpression: ...
def to_json_type_expression(value: TypeExpression) -> Json: ...
def from_json_type_expression(value: Json) -> TypeExpression: ...

@dataclass(frozen=True, slots=True)
class TypeLiteralNever:
    """Never type `never`."""

    kind: typing.Literal["never"] = "never"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeLiteralAny:
    """Any type `any`."""

    kind: typing.Literal["any"] = "any"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeLiteralUndefined:
    """Undefined type and value."""

    kind: typing.Literal["undefined"] = "undefined"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeLiteralUnknown:
    """Unknown type."""

    kind: typing.Literal["unknown"] = "unknown"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeLiteralObject:
    """Object type (any non-primitive)."""

    kind: typing.Literal["object"] = "object"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeLiteralVoid:
    """Void type."""

    kind: typing.Literal["void"] = "void"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeLiteralNull:
    """Null type and value."""

    kind: typing.Literal["null"] = "null"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeLiteralPrimitive:
    """Primitive type."""

    primitive: PrimitiveType
    kind: typing.Literal["primitive"] = "primitive"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeLiteralScalarLiteral:
    """Scalar literal."""

    scalar_literal: destack._generated.js.tree.literal.ScalarLiteral
    kind: typing.Literal["scalarLiteral"] = "scalarLiteral"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""A TypeLiteral is a scalar type."""
TypeLiteral: typing.TypeAlias = (
    TypeLiteralNever
    | TypeLiteralAny
    | TypeLiteralUndefined
    | TypeLiteralUnknown
    | TypeLiteralObject
    | TypeLiteralVoid
    | TypeLiteralNull
    | TypeLiteralPrimitive
    | TypeLiteralScalarLiteral
)

def encode_type_literal(writer: BinaryWriter, value: TypeLiteral) -> None: ...
def decode_type_literal(reader: BinaryReader) -> TypeLiteral: ...
def to_json_type_literal(value: TypeLiteral) -> Json: ...
def from_json_type_literal(value: Json) -> TypeLiteral: ...

"""A PrimitiveType is a primitive type node."""
PrimitiveType: typing.TypeAlias = (
    typing.Literal["boolean"]
    | typing.Literal["string"]
    | typing.Literal["bigint"]
    | typing.Literal["number"]
    | typing.Literal["symbol"]
    | typing.Literal["uniqueSymbol"]
)

def encode_primitive_type(writer: BinaryWriter, value: PrimitiveType) -> None: ...
def decode_primitive_type(reader: BinaryReader) -> PrimitiveType: ...
def to_json_primitive_type(value: PrimitiveType) -> Json: ...
def from_json_primitive_type(value: Json) -> PrimitiveType: ...

@dataclass(frozen=True, slots=True)
class TypeMappedParameter:
    """One mapped type parameter."""

    # the parameter name
    name: destack._generated.core.string.StringId
    # the source type iterated by `in`
    source_type: destack._generated.js.tree.node.LocalNodeId
    # the optional key remap
    key_remap: destack._generated.js.tree.node.LocalNodeId | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeMappedParameter: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TypeMappedParameter: ...

def encode_type_mapped_parameter(
    writer: BinaryWriter, value: TypeMappedParameter
) -> None: ...
def decode_type_mapped_parameter(reader: BinaryReader) -> TypeMappedParameter: ...
def to_json_type_mapped_parameter(value: TypeMappedParameter) -> Json: ...
def from_json_type_mapped_parameter(value: Json) -> TypeMappedParameter: ...

@dataclass(frozen=True, slots=True)
class TypeMappedModifiers:
    """One mapped type modifier set."""

    # the readonly modifier
    readonly: MappedTypeModifier
    # the optional modifier
    optional: MappedTypeModifier

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeMappedModifiers: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TypeMappedModifiers: ...

def encode_type_mapped_modifiers(
    writer: BinaryWriter, value: TypeMappedModifiers
) -> None: ...
def decode_type_mapped_modifiers(reader: BinaryReader) -> TypeMappedModifiers: ...
def to_json_type_mapped_modifiers(value: TypeMappedModifiers) -> Json: ...
def from_json_type_mapped_modifiers(value: Json) -> TypeMappedModifiers: ...

"""One mapped type modifier."""
MappedTypeModifier: typing.TypeAlias = (
    typing.Literal["present"]
    | typing.Literal["add"]
    | typing.Literal["remove"]
    | typing.Literal["none"]
)

def encode_mapped_type_modifier(
    writer: BinaryWriter, value: MappedTypeModifier
) -> None: ...
def decode_mapped_type_modifier(reader: BinaryReader) -> MappedTypeModifier: ...
def to_json_mapped_type_modifier(value: MappedTypeModifier) -> Json: ...
def from_json_mapped_type_modifier(value: Json) -> MappedTypeModifier: ...

@dataclass(frozen=True, slots=True)
class TypeTemplateLiteral:
    """One type template literal."""

    # the raw template strings
    strings: Sequence[destack._generated.core.string.StringId]
    # the interpolated type spans
    spans: Sequence[destack._generated.js.tree.node.LocalNodeId]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeTemplateLiteral: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TypeTemplateLiteral: ...

def encode_type_template_literal(
    writer: BinaryWriter, value: TypeTemplateLiteral
) -> None: ...
def decode_type_template_literal(reader: BinaryReader) -> TypeTemplateLiteral: ...
def to_json_type_template_literal(value: TypeTemplateLiteral) -> Json: ...
def from_json_type_template_literal(value: Json) -> TypeTemplateLiteral: ...

@dataclass(frozen=True, slots=True)
class FunctionTypeDeclaration:
    """One function type declaration in type space."""

    # the generic parameters of the function type
    generic_parameters: Sequence[destack._generated.js.tree.node.LocalNodeId]
    # the optional `this` parameter
    this_parameter: destack._generated.js.tree.node.LocalNodeId | None
    # the parameters of the function type
    parameters: Sequence[destack._generated.js.tree.node.LocalNodeId]
    # the return type of the function type
    return_type: destack._generated.js.tree.node.LocalNodeId | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionTypeDeclaration: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FunctionTypeDeclaration: ...

def encode_function_type_declaration(
    writer: BinaryWriter, value: FunctionTypeDeclaration
) -> None: ...
def decode_function_type_declaration(
    reader: BinaryReader,
) -> FunctionTypeDeclaration: ...
def to_json_function_type_declaration(value: FunctionTypeDeclaration) -> Json: ...
def from_json_function_type_declaration(value: Json) -> FunctionTypeDeclaration: ...

@dataclass(frozen=True, slots=True)
class ConstructorTypeDeclaration:
    """One constructor type declaration in type space."""

    # whether the constructor type is abstract
    is_abstract: bool
    # the generic parameters of the constructor type
    generic_parameters: Sequence[destack._generated.js.tree.node.LocalNodeId]
    # the parameters of the constructor type
    parameters: Sequence[destack._generated.js.tree.node.LocalNodeId]
    # the return type of the constructor type
    return_type: destack._generated.js.tree.node.LocalNodeId | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ConstructorTypeDeclaration: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ConstructorTypeDeclaration: ...

def encode_constructor_type_declaration(
    writer: BinaryWriter, value: ConstructorTypeDeclaration
) -> None: ...
def decode_constructor_type_declaration(
    reader: BinaryReader,
) -> ConstructorTypeDeclaration: ...
def to_json_constructor_type_declaration(value: ConstructorTypeDeclaration) -> Json: ...
def from_json_constructor_type_declaration(
    value: Json,
) -> ConstructorTypeDeclaration: ...

@dataclass(frozen=True, slots=True)
class TupleElement:
    """One tuple type element."""

    # the optional element label
    label: destack._generated.core.string.StringId | None
    # the element type
    ty: destack._generated.js.tree.node.LocalNodeId
    # whether the element is optional
    is_optional: bool
    # whether the element is readonly
    is_readonly: bool
    # whether the element is a rest element
    is_rest: bool

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TupleElement: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TupleElement: ...

def encode_tuple_element(writer: BinaryWriter, value: TupleElement) -> None: ...
def decode_tuple_element(reader: BinaryReader) -> TupleElement: ...
def to_json_tuple_element(value: TupleElement) -> Json: ...
def from_json_tuple_element(value: Json) -> TupleElement: ...

@dataclass(frozen=True, slots=True)
class TypeMemberField:
    """Named field (like `a: T`)."""

    modifiers: destack._generated.js.tree.argument.BindingModifier | None
    key: destack._generated.js.tree.key.Key
    ty: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["field"] = "field"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeMemberMethod:
    """Named method (like `foo(): T`)."""

    modifiers: destack._generated.js.tree.argument.BindingModifier | None
    key: destack._generated.js.tree.key.Key
    signature: destack._generated.js.tree.function.FunctionSignature
    kind: typing.Literal["method"] = "method"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeMemberCallSignature:
    """Call signature (like `<T>(value: T): U`)."""

    modifiers: destack._generated.js.tree.argument.BindingModifier | None
    signature: FunctionTypeDeclaration
    kind: typing.Literal["callSignature"] = "callSignature"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeMemberConstructSignature:
    """Construct signature (like `new <T>(value: T): U`)."""

    modifiers: destack._generated.js.tree.argument.BindingModifier | None
    signature: ConstructorTypeDeclaration
    kind: typing.Literal["constructSignature"] = "constructSignature"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeMemberIndexSignature:
    """Index signature (like `[key: string]: T`)."""

    modifiers: destack._generated.js.tree.argument.BindingModifier | None
    name: destack._generated.core.string.StringId
    key_type: destack._generated.js.tree.node.LocalNodeId
    value_type: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["indexSignature"] = "indexSignature"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""The type of an attribute (like a property or field)."""
TypeMember: typing.TypeAlias = (
    TypeMemberField
    | TypeMemberMethod
    | TypeMemberCallSignature
    | TypeMemberConstructSignature
    | TypeMemberIndexSignature
)

def encode_type_member(writer: BinaryWriter, value: TypeMember) -> None: ...
def decode_type_member(reader: BinaryReader) -> TypeMember: ...
def to_json_type_member(value: TypeMember) -> Json: ...
def from_json_type_member(value: Json) -> TypeMember: ...

__all__ = [
    "TypeExpression",
    "encode_type_expression",
    "decode_type_expression",
    "to_json_type_expression",
    "from_json_type_expression",
    "TypeExpressionScalar",
    "TypeExpressionThis",
    "TypeExpressionPath",
    "TypeExpressionReadonly",
    "TypeExpressionKeyOf",
    "TypeExpressionMust",
    "TypeExpressionNot",
    "TypeExpressionExtends",
    "TypeExpressionImplements",
    "TypeExpressionConditional",
    "TypeExpressionMapped",
    "TypeExpressionIndex",
    "TypeExpressionTemplateLiteral",
    "TypeExpressionImport",
    "TypeExpressionInfer",
    "TypeExpressionArray",
    "TypeExpressionTuple",
    "TypeExpressionObject",
    "TypeExpressionUnion",
    "TypeExpressionIntersection",
    "TypeExpressionFunctionTypeDeclaration",
    "TypeExpressionConstructorTypeDeclaration",
    "TypeExpressionError",
    "TypeLiteral",
    "encode_type_literal",
    "decode_type_literal",
    "to_json_type_literal",
    "from_json_type_literal",
    "TypeLiteralNever",
    "TypeLiteralAny",
    "TypeLiteralUndefined",
    "TypeLiteralUnknown",
    "TypeLiteralObject",
    "TypeLiteralVoid",
    "TypeLiteralNull",
    "TypeLiteralPrimitive",
    "TypeLiteralScalarLiteral",
    "PrimitiveType",
    "encode_primitive_type",
    "decode_primitive_type",
    "to_json_primitive_type",
    "from_json_primitive_type",
    "TypeMappedParameter",
    "encode_type_mapped_parameter",
    "decode_type_mapped_parameter",
    "to_json_type_mapped_parameter",
    "from_json_type_mapped_parameter",
    "TypeMappedModifiers",
    "encode_type_mapped_modifiers",
    "decode_type_mapped_modifiers",
    "to_json_type_mapped_modifiers",
    "from_json_type_mapped_modifiers",
    "MappedTypeModifier",
    "encode_mapped_type_modifier",
    "decode_mapped_type_modifier",
    "to_json_mapped_type_modifier",
    "from_json_mapped_type_modifier",
    "TypeTemplateLiteral",
    "encode_type_template_literal",
    "decode_type_template_literal",
    "to_json_type_template_literal",
    "from_json_type_template_literal",
    "FunctionTypeDeclaration",
    "encode_function_type_declaration",
    "decode_function_type_declaration",
    "to_json_function_type_declaration",
    "from_json_function_type_declaration",
    "ConstructorTypeDeclaration",
    "encode_constructor_type_declaration",
    "decode_constructor_type_declaration",
    "to_json_constructor_type_declaration",
    "from_json_constructor_type_declaration",
    "TupleElement",
    "encode_tuple_element",
    "decode_tuple_element",
    "to_json_tuple_element",
    "from_json_tuple_element",
    "TypeMember",
    "encode_type_member",
    "decode_type_member",
    "to_json_type_member",
    "from_json_type_member",
    "TypeMemberField",
    "TypeMemberMethod",
    "TypeMemberCallSignature",
    "TypeMemberConstructSignature",
    "TypeMemberIndexSignature",
]
