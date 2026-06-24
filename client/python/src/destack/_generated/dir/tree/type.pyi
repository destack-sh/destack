# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.string
import destack._generated.dir.tree.expression
import destack._generated.dir.tree.function
import destack._generated.dir.tree.key
import destack._generated.dir.tree.literal
import destack._generated.dir.tree.node
import destack._generated.dir.tree.operator
import destack._generated.dir.tree.path

@dataclass(frozen=True, slots=True)
class TypeExpressionParenthesized:
    """Parenthesized type expression."""

    expression: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["parenthesized"] = "parenthesized"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionScalarLiteral:
    """Scalar literal type."""

    value: destack._generated.dir.tree.literal.ScalarLiteral
    kind: typing.Literal["scalarLiteral"] = "scalarLiteral"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionLiteral:
    """Literal type."""

    value: destack._generated.dir.tree.literal.TypeLiteral
    kind: typing.Literal["literal"] = "literal"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionIntrinsic:
    """Bare `intrinsic` marker in type space."""

    kind: typing.Literal["intrinsic"] = "intrinsic"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionTuple:
    """Parenthesized tuple type."""

    elements: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["tuple"] = "tuple"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionArrayTuple:
    """Bracket tuple type."""

    elements: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["arrayTuple"] = "arrayTuple"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionArray:
    """Homogeneous array type."""

    element: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["array"] = "array"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionSlice:
    """Runtime-length homogeneous view type."""

    # the element type
    element: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["slice"] = "slice"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionFixedArray:
    """Fixed-length array type."""

    # the element type
    element: destack._generated.dir.tree.node.LocalNodeId
    # the length expression
    length: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["fixedArray"] = "fixedArray"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionObject:
    """Object type."""

    members: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["object"] = "object"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionFunction:
    """Function type."""

    function: FunctionTypeExpression
    kind: typing.Literal["function"] = "function"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionConstructor:
    """Constructor type."""

    constructor: ConstructorType
    kind: typing.Literal["constructor"] = "constructor"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionReference:
    """Qualified type reference with optional generic arguments."""

    path: destack._generated.dir.tree.path.Path
    generic_arguments: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["reference"] = "reference"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionMember:
    """Type member projection with optional generic arguments."""

    left: destack._generated.dir.tree.node.LocalNodeId
    name: destack._generated.core.string.StringId
    generic_arguments: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["member"] = "member"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionRange:
    """Range type expression."""

    start: destack._generated.dir.tree.node.LocalNodeId | None
    end: destack._generated.dir.tree.node.LocalNodeId | None
    end_kind: destack._generated.dir.tree.operator.RangeEnd
    kind: typing.Literal["range"] = "range"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionConst:
    """`const` in type space."""

    kind: typing.Literal["const"] = "const"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionThis:
    """`this` in type space."""

    kind: typing.Literal["this"] = "this"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionReadonly:
    """`readonly T`."""

    target_type: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["readonly"] = "readonly"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionLocal:
    """`local T`."""

    target_type: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["local"] = "local"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionShared:
    """`shared T`."""

    target_type: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["shared"] = "shared"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionKeyOf:
    """`keyof T`."""

    target_type: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["keyOf"] = "keyOf"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionTypeOf:
    """`typeof value`."""

    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["typeOf"] = "typeOf"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionMust:
    """`T!`."""

    target_type: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["must"] = "must"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionNot:
    """`!T`."""

    target_type: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["not"] = "not"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionOwnedOf:
    """`^T`."""

    mutability: destack._generated.dir.tree.node.Mutability | None
    variance: destack._generated.dir.tree.expression.VarianceBound | None
    target_type: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["ownedOf"] = "ownedOf"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionBorrowedOf:
    """`&T`."""

    mutability: destack._generated.dir.tree.node.Mutability | None
    variance: destack._generated.dir.tree.expression.VarianceBound | None
    target_type: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["borrowedOf"] = "borrowedOf"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionPointerOf:
    """`*T`."""

    mutability: destack._generated.dir.tree.node.Mutability | None
    target_type: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["pointerOf"] = "pointerOf"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionUnion:
    """Union type."""

    elements: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["union"] = "union"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionIntersection:
    """Intersection type."""

    elements: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["intersection"] = "intersection"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionConditional:
    """Conditional type."""

    left: destack._generated.dir.tree.node.LocalNodeId
    extends_type: destack._generated.dir.tree.node.LocalNodeId
    then_type: destack._generated.dir.tree.node.LocalNodeId
    else_type: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["conditional"] = "conditional"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionExtends:
    """Assignability relation."""

    left: destack._generated.dir.tree.node.LocalNodeId
    right: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["extends"] = "extends"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionImplements:
    """Explicit conformance relation."""

    left: destack._generated.dir.tree.node.LocalNodeId
    right: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["implements"] = "implements"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionMapped:
    """Mapped type."""

    parameter: destack._generated.dir.tree.node.LocalNodeId
    readonly: MappedTypeModifier
    optional: MappedTypeModifier
    value: destack._generated.dir.tree.node.LocalNodeId | None
    kind: typing.Literal["mapped"] = "mapped"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionIndex:
    """Indexed access type."""

    left: destack._generated.dir.tree.node.LocalNodeId
    index: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["index"] = "index"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionTemplateLiteral:
    """Template literal type."""

    strings: Sequence[destack._generated.core.string.StringId]
    spans: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["templateLiteral"] = "templateLiteral"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionInfer:
    """An Infer expression is a named infer binding or anonymous inference hole."""

    form: InferForm
    name: destack._generated.core.string.StringId | None
    constraint: destack._generated.dir.tree.node.LocalNodeId | None
    kind: typing.Literal["infer"] = "infer"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionMissing:
    """Missing type child."""

    kind: typing.Literal["missing"] = "missing"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeExpressionError:
    """Error placeholder."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""A type-space expression."""
TypeExpression: typing.TypeAlias = (
    TypeExpressionParenthesized
    | TypeExpressionScalarLiteral
    | TypeExpressionLiteral
    | TypeExpressionIntrinsic
    | TypeExpressionTuple
    | TypeExpressionArrayTuple
    | TypeExpressionArray
    | TypeExpressionSlice
    | TypeExpressionFixedArray
    | TypeExpressionObject
    | TypeExpressionFunction
    | TypeExpressionConstructor
    | TypeExpressionReference
    | TypeExpressionMember
    | TypeExpressionRange
    | TypeExpressionConst
    | TypeExpressionThis
    | TypeExpressionReadonly
    | TypeExpressionLocal
    | TypeExpressionShared
    | TypeExpressionKeyOf
    | TypeExpressionTypeOf
    | TypeExpressionMust
    | TypeExpressionNot
    | TypeExpressionOwnedOf
    | TypeExpressionBorrowedOf
    | TypeExpressionPointerOf
    | TypeExpressionUnion
    | TypeExpressionIntersection
    | TypeExpressionConditional
    | TypeExpressionExtends
    | TypeExpressionImplements
    | TypeExpressionMapped
    | TypeExpressionIndex
    | TypeExpressionTemplateLiteral
    | TypeExpressionInfer
    | TypeExpressionMissing
    | TypeExpressionError
)

def encode_type_expression(writer: BinaryWriter, value: TypeExpression) -> None: ...
def decode_type_expression(reader: BinaryReader) -> TypeExpression: ...
def to_json_type_expression(value: TypeExpression) -> Json: ...
def from_json_type_expression(value: Json) -> TypeExpression: ...

@dataclass(frozen=True, slots=True)
class FunctionTypeExpression:
    """One function type expression in type space."""

    # the generic parameters of the function type
    generic_parameters: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # the where clauses of the function type
    where_clauses: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # the source form used for the receiver
    this_form: destack._generated.dir.tree.function.ThisForm | None
    # the optional `this` parameter
    this_parameter: destack._generated.dir.tree.node.LocalNodeId | None
    # the parameters of the function type
    parameters: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # the return type of the function type
    return_type: destack._generated.dir.tree.node.LocalNodeId | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionTypeExpression: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FunctionTypeExpression: ...

def encode_function_type_expression(
    writer: BinaryWriter, value: FunctionTypeExpression
) -> None: ...
def decode_function_type_expression(reader: BinaryReader) -> FunctionTypeExpression: ...
def to_json_function_type_expression(value: FunctionTypeExpression) -> Json: ...
def from_json_function_type_expression(value: Json) -> FunctionTypeExpression: ...

@dataclass(frozen=True, slots=True)
class ConstructorType:
    """One constructor type in type space."""

    # the generic parameters of the constructor type
    generic_parameters: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # the where clauses of the constructor type
    where_clauses: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # the parameters of the constructor type
    parameters: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # the return type of the constructor type
    return_type: destack._generated.dir.tree.node.LocalNodeId | None
    # whether the constructor type is abstract
    is_abstract: bool

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ConstructorType: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ConstructorType: ...

def encode_constructor_type(writer: BinaryWriter, value: ConstructorType) -> None: ...
def decode_constructor_type(reader: BinaryReader) -> ConstructorType: ...
def to_json_constructor_type(value: ConstructorType) -> Json: ...
def from_json_constructor_type(value: Json) -> ConstructorType: ...

"""A mapped-type modifier sign."""
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

"""The parsed form of an infer type expression."""
InferForm: typing.TypeAlias = typing.Literal["hole"] | typing.Literal["infer"]

def encode_infer_form(writer: BinaryWriter, value: InferForm) -> None: ...
def decode_infer_form(reader: BinaryReader) -> InferForm: ...
def to_json_infer_form(value: InferForm) -> Json: ...
def from_json_infer_form(value: Json) -> InferForm: ...

@dataclass(frozen=True, slots=True)
class TypeMemberField:
    """Named field."""

    key: destack._generated.dir.tree.key.Key
    declared_type: destack._generated.dir.tree.node.LocalNodeId | None
    is_static: bool
    is_optional: bool
    is_readonly: bool
    kind: typing.Literal["field"] = "field"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeMemberMethod:
    """Named method."""

    key: destack._generated.dir.tree.key.Key
    signature: destack._generated.dir.tree.function.FunctionSignature
    body: destack._generated.dir.tree.node.LocalNodeId | None
    is_static: bool
    is_optional: bool
    kind: typing.Literal["method"] = "method"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeMemberCallSignature:
    """Call signature declaration."""

    # the call signature function type expression
    signature: FunctionTypeExpression
    kind: typing.Literal["callSignature"] = "callSignature"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeMemberConstructSignature:
    """Construct signature declaration."""

    signature: ConstructorType
    kind: typing.Literal["constructSignature"] = "constructSignature"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeMemberIndexSignature:
    """Index signature."""

    name: destack._generated.core.string.StringId
    key_type: destack._generated.dir.tree.node.LocalNodeId
    value_type: destack._generated.dir.tree.node.LocalNodeId
    is_optional: bool
    is_readonly: bool
    kind: typing.Literal["indexSignature"] = "indexSignature"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeMemberAssociatedType:
    """Associated type requirement or definition."""

    name: destack._generated.core.string.StringId
    generic_parameters: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    where_clauses: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    constraint: destack._generated.dir.tree.node.LocalNodeId | None
    value: destack._generated.dir.tree.node.LocalNodeId | None
    is_abstract: bool
    is_override: bool
    kind: typing.Literal["associatedType"] = "associatedType"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeMemberAssociatedConst:
    """Associated compile-time constant requirement or definition."""

    name: destack._generated.core.string.StringId
    declared_type: destack._generated.dir.tree.node.LocalNodeId | None
    value: destack._generated.dir.tree.node.LocalNodeId | None
    is_abstract: bool
    is_override: bool
    kind: typing.Literal["associatedConst"] = "associatedConst"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TypeMemberError:
    """Malformed type member slot."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""One type-surface member."""
TypeMember: typing.TypeAlias = (
    TypeMemberField
    | TypeMemberMethod
    | TypeMemberCallSignature
    | TypeMemberConstructSignature
    | TypeMemberIndexSignature
    | TypeMemberAssociatedType
    | TypeMemberAssociatedConst
    | TypeMemberError
)

def encode_type_member(writer: BinaryWriter, value: TypeMember) -> None: ...
def decode_type_member(reader: BinaryReader) -> TypeMember: ...
def to_json_type_member(value: TypeMember) -> Json: ...
def from_json_type_member(value: Json) -> TypeMember: ...

@dataclass(frozen=True, slots=True)
class TypeMappedParameter:
    """A mapped type parameter."""

    # the parameter name
    name: destack._generated.core.string.StringId
    # the source type iterated by `in`
    source_type: destack._generated.dir.tree.node.LocalNodeId
    # the optional key remap
    key_remap: destack._generated.dir.tree.node.LocalNodeId | None

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

__all__ = [
    "TypeExpression",
    "encode_type_expression",
    "decode_type_expression",
    "to_json_type_expression",
    "from_json_type_expression",
    "TypeExpressionParenthesized",
    "TypeExpressionScalarLiteral",
    "TypeExpressionLiteral",
    "TypeExpressionIntrinsic",
    "TypeExpressionTuple",
    "TypeExpressionArrayTuple",
    "TypeExpressionArray",
    "TypeExpressionSlice",
    "TypeExpressionFixedArray",
    "TypeExpressionObject",
    "TypeExpressionFunction",
    "TypeExpressionConstructor",
    "TypeExpressionReference",
    "TypeExpressionMember",
    "TypeExpressionRange",
    "TypeExpressionConst",
    "TypeExpressionThis",
    "TypeExpressionReadonly",
    "TypeExpressionLocal",
    "TypeExpressionShared",
    "TypeExpressionKeyOf",
    "TypeExpressionTypeOf",
    "TypeExpressionMust",
    "TypeExpressionNot",
    "TypeExpressionOwnedOf",
    "TypeExpressionBorrowedOf",
    "TypeExpressionPointerOf",
    "TypeExpressionUnion",
    "TypeExpressionIntersection",
    "TypeExpressionConditional",
    "TypeExpressionExtends",
    "TypeExpressionImplements",
    "TypeExpressionMapped",
    "TypeExpressionIndex",
    "TypeExpressionTemplateLiteral",
    "TypeExpressionInfer",
    "TypeExpressionMissing",
    "TypeExpressionError",
    "FunctionTypeExpression",
    "encode_function_type_expression",
    "decode_function_type_expression",
    "to_json_function_type_expression",
    "from_json_function_type_expression",
    "ConstructorType",
    "encode_constructor_type",
    "decode_constructor_type",
    "to_json_constructor_type",
    "from_json_constructor_type",
    "MappedTypeModifier",
    "encode_mapped_type_modifier",
    "decode_mapped_type_modifier",
    "to_json_mapped_type_modifier",
    "from_json_mapped_type_modifier",
    "InferForm",
    "encode_infer_form",
    "decode_infer_form",
    "to_json_infer_form",
    "from_json_infer_form",
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
    "TypeMemberAssociatedType",
    "TypeMemberAssociatedConst",
    "TypeMemberError",
    "TypeMappedParameter",
    "encode_type_mapped_parameter",
    "decode_type_mapped_parameter",
    "to_json_type_mapped_parameter",
    "from_json_type_mapped_parameter",
]
