# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_array,
    json_bool,
    json_field,
    json_object,
    json_optional,
    json_string,
)

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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionScalarLiteral:
    """Scalar literal type."""

    value: destack._generated.dir.tree.literal.ScalarLiteral
    kind: typing.Literal["scalarLiteral"] = "scalarLiteral"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionLiteral:
    """Literal type."""

    value: destack._generated.dir.tree.literal.TypeLiteral
    kind: typing.Literal["literal"] = "literal"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionIntrinsic:
    """Bare `intrinsic` marker in type space."""

    kind: typing.Literal["intrinsic"] = "intrinsic"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionTuple:
    """Parenthesized tuple type."""

    elements: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["tuple"] = "tuple"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionArrayTuple:
    """Bracket tuple type."""

    elements: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["arrayTuple"] = "arrayTuple"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionArray:
    """Homogeneous array type."""

    element: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["array"] = "array"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionSlice:
    """Runtime-length homogeneous view type."""

    # the element type
    element: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["slice"] = "slice"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionFixedArray:
    """Fixed-length array type."""

    # the element type
    element: destack._generated.dir.tree.node.LocalNodeId
    # the length expression
    length: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["fixedArray"] = "fixedArray"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionObject:
    """Object type."""

    members: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["object"] = "object"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionFunction:
    """Function type."""

    function: FunctionTypeExpression
    kind: typing.Literal["function"] = "function"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionConstructor:
    """Constructor type."""

    constructor: ConstructorType
    kind: typing.Literal["constructor"] = "constructor"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionReference:
    """Qualified type reference with optional generic arguments."""

    path: destack._generated.dir.tree.path.Path
    generic_arguments: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["reference"] = "reference"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionMember:
    """Type member projection with optional generic arguments."""

    left: destack._generated.dir.tree.node.LocalNodeId
    name: destack._generated.core.string.StringId
    generic_arguments: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["member"] = "member"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionRange:
    """Range type expression."""

    start: destack._generated.dir.tree.node.LocalNodeId | None
    end: destack._generated.dir.tree.node.LocalNodeId | None
    end_kind: destack._generated.dir.tree.operator.RangeEnd
    kind: typing.Literal["range"] = "range"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionConst:
    """`const` in type space."""

    kind: typing.Literal["const"] = "const"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionThis:
    """`this` in type space."""

    kind: typing.Literal["this"] = "this"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionReadonly:
    """`readonly T`."""

    target_type: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["readonly"] = "readonly"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionLocal:
    """`local T`."""

    target_type: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["local"] = "local"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionShared:
    """`shared T`."""

    target_type: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["shared"] = "shared"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionKeyOf:
    """`keyof T`."""

    target_type: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["keyOf"] = "keyOf"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionTypeOf:
    """`typeof value`."""

    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["typeOf"] = "typeOf"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionMust:
    """`T!`."""

    target_type: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["must"] = "must"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionNot:
    """`!T`."""

    target_type: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["not"] = "not"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionOwnedOf:
    """`^T`."""

    mutability: destack._generated.dir.tree.node.Mutability | None
    variance: destack._generated.dir.tree.expression.VarianceBound | None
    target_type: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["ownedOf"] = "ownedOf"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionBorrowedOf:
    """`&T`."""

    mutability: destack._generated.dir.tree.node.Mutability | None
    variance: destack._generated.dir.tree.expression.VarianceBound | None
    target_type: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["borrowedOf"] = "borrowedOf"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionPointerOf:
    """`*T`."""

    mutability: destack._generated.dir.tree.node.Mutability | None
    target_type: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["pointerOf"] = "pointerOf"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionUnion:
    """Union type."""

    elements: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["union"] = "union"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionIntersection:
    """Intersection type."""

    elements: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["intersection"] = "intersection"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionConditional:
    """Conditional type."""

    left: destack._generated.dir.tree.node.LocalNodeId
    extends_type: destack._generated.dir.tree.node.LocalNodeId
    then_type: destack._generated.dir.tree.node.LocalNodeId
    else_type: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["conditional"] = "conditional"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionExtends:
    """Assignability relation."""

    left: destack._generated.dir.tree.node.LocalNodeId
    right: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["extends"] = "extends"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionImplements:
    """Explicit conformance relation."""

    left: destack._generated.dir.tree.node.LocalNodeId
    right: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["implements"] = "implements"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionMapped:
    """Mapped type."""

    parameter: destack._generated.dir.tree.node.LocalNodeId
    readonly: MappedTypeModifier
    optional: MappedTypeModifier
    value: destack._generated.dir.tree.node.LocalNodeId | None
    kind: typing.Literal["mapped"] = "mapped"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionIndex:
    """Indexed access type."""

    left: destack._generated.dir.tree.node.LocalNodeId
    index: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["index"] = "index"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionTemplateLiteral:
    """Template literal type."""

    strings: Sequence[destack._generated.core.string.StringId]
    spans: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    kind: typing.Literal["templateLiteral"] = "templateLiteral"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionInfer:
    """An Infer expression is a named infer binding or anonymous inference hole."""

    form: InferForm
    name: destack._generated.core.string.StringId | None
    constraint: destack._generated.dir.tree.node.LocalNodeId | None
    kind: typing.Literal["infer"] = "infer"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionMissing:
    """Missing type child."""

    kind: typing.Literal["missing"] = "missing"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionError:
    """Error placeholder."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


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


def encode_type_expression(writer: BinaryWriter, value: TypeExpression) -> None:
    """Encode one TypeExpression."""
    if value.kind == "parenthesized":
        writer.write_unsigned(0)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.expression)
    elif value.kind == "scalarLiteral":
        writer.write_unsigned(1)
        destack._generated.dir.tree.literal.encode_scalar_literal(writer, value.value)
    elif value.kind == "literal":
        writer.write_unsigned(2)
        destack._generated.dir.tree.literal.encode_type_literal(writer, value.value)
    elif value.kind == "intrinsic":
        writer.write_unsigned(3)
    elif value.kind == "tuple":
        writer.write_unsigned(4)
        writer.write_unsigned(len(value.elements))
        for item_value_elements_0 in value.elements:
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, item_value_elements_0
            )
    elif value.kind == "arrayTuple":
        writer.write_unsigned(5)
        writer.write_unsigned(len(value.elements))
        for item_value_elements_0 in value.elements:
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, item_value_elements_0
            )
    elif value.kind == "array":
        writer.write_unsigned(6)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.element)
    elif value.kind == "slice":
        writer.write_unsigned(7)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.element)
    elif value.kind == "fixedArray":
        writer.write_unsigned(8)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.element)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.length)
    elif value.kind == "object":
        writer.write_unsigned(9)
        writer.write_unsigned(len(value.members))
        for item_value_members_0 in value.members:
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, item_value_members_0
            )
    elif value.kind == "function":
        writer.write_unsigned(10)
        encode_function_type_expression(writer, value.function)
    elif value.kind == "constructor":
        writer.write_unsigned(11)
        encode_constructor_type(writer, value.constructor)
    elif value.kind == "reference":
        writer.write_unsigned(12)
        destack._generated.dir.tree.path.encode_path(writer, value.path)
        writer.write_unsigned(len(value.generic_arguments))
        for item_value_generic_arguments_0 in value.generic_arguments:
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, item_value_generic_arguments_0
            )
    elif value.kind == "member":
        writer.write_unsigned(13)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.left)
        destack._generated.core.string.encode_string_id(writer, value.name)
        writer.write_unsigned(len(value.generic_arguments))
        for item_value_generic_arguments_0 in value.generic_arguments:
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, item_value_generic_arguments_0
            )
    elif value.kind == "range":
        writer.write_unsigned(14)
        if value.start is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(writer, value.start)
        if value.end is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(writer, value.end)
        destack._generated.dir.tree.operator.encode_range_end(writer, value.end_kind)
    elif value.kind == "const":
        writer.write_unsigned(15)
    elif value.kind == "this":
        writer.write_unsigned(16)
    elif value.kind == "readonly":
        writer.write_unsigned(17)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.target_type)
    elif value.kind == "local":
        writer.write_unsigned(18)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.target_type)
    elif value.kind == "shared":
        writer.write_unsigned(19)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.target_type)
    elif value.kind == "keyOf":
        writer.write_unsigned(20)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.target_type)
    elif value.kind == "typeOf":
        writer.write_unsigned(21)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "must":
        writer.write_unsigned(22)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.target_type)
    elif value.kind == "not":
        writer.write_unsigned(23)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.target_type)
    elif value.kind == "ownedOf":
        writer.write_unsigned(24)
        if value.mutability is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_mutability(writer, value.mutability)
        if value.variance is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.expression.encode_variance_bound(
                writer, value.variance
            )
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.target_type)
    elif value.kind == "borrowedOf":
        writer.write_unsigned(25)
        if value.mutability is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_mutability(writer, value.mutability)
        if value.variance is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.expression.encode_variance_bound(
                writer, value.variance
            )
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.target_type)
    elif value.kind == "pointerOf":
        writer.write_unsigned(26)
        if value.mutability is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_mutability(writer, value.mutability)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.target_type)
    elif value.kind == "union":
        writer.write_unsigned(27)
        writer.write_unsigned(len(value.elements))
        for item_value_elements_0 in value.elements:
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, item_value_elements_0
            )
    elif value.kind == "intersection":
        writer.write_unsigned(28)
        writer.write_unsigned(len(value.elements))
        for item_value_elements_0 in value.elements:
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, item_value_elements_0
            )
    elif value.kind == "conditional":
        writer.write_unsigned(29)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.left)
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, value.extends_type
        )
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.then_type)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.else_type)
    elif value.kind == "extends":
        writer.write_unsigned(30)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.left)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.right)
    elif value.kind == "implements":
        writer.write_unsigned(31)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.left)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.right)
    elif value.kind == "mapped":
        writer.write_unsigned(32)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.parameter)
        encode_mapped_type_modifier(writer, value.readonly)
        encode_mapped_type_modifier(writer, value.optional)
        if value.value is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "index":
        writer.write_unsigned(33)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.left)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.index)
    elif value.kind == "templateLiteral":
        writer.write_unsigned(34)
        writer.write_unsigned(len(value.strings))
        for item_value_strings_0 in value.strings:
            destack._generated.core.string.encode_string_id(
                writer, item_value_strings_0
            )
        writer.write_unsigned(len(value.spans))
        for item_value_spans_0 in value.spans:
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, item_value_spans_0
            )
    elif value.kind == "infer":
        writer.write_unsigned(35)
        encode_infer_form(writer, value.form)
        if value.name is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.core.string.encode_string_id(writer, value.name)
        if value.constraint is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, value.constraint
            )
    elif value.kind == "missing":
        writer.write_unsigned(36)
    elif value.kind == "error":
        writer.write_unsigned(37)
    else:
        raise SerdeError("unknown enum variant")


def decode_type_expression(reader: BinaryReader) -> TypeExpression:
    """Decode one TypeExpression."""
    variant = reader.read_number()

    if variant == 0:
        expression = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return TypeExpressionParenthesized(
            expression=expression,
        )
    elif variant == 1:
        value_ = destack._generated.dir.tree.literal.decode_scalar_literal(reader)

        return TypeExpressionScalarLiteral(
            value=value_,
        )
    elif variant == 2:
        value_ = destack._generated.dir.tree.literal.decode_type_literal(reader)

        return TypeExpressionLiteral(
            value=value_,
        )
    elif variant == 3:
        return TypeExpressionIntrinsic()
    elif variant == 4:
        elements = [
            destack._generated.dir.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return TypeExpressionTuple(
            elements=elements,
        )
    elif variant == 5:
        elements = [
            destack._generated.dir.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return TypeExpressionArrayTuple(
            elements=elements,
        )
    elif variant == 6:
        element = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return TypeExpressionArray(
            element=element,
        )
    elif variant == 7:
        element = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return TypeExpressionSlice(
            element=element,
        )
    elif variant == 8:
        element = destack._generated.dir.tree.node.decode_local_node_id(reader)
        length = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return TypeExpressionFixedArray(
            element=element,
            length=length,
        )
    elif variant == 9:
        members = [
            destack._generated.dir.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return TypeExpressionObject(
            members=members,
        )
    elif variant == 10:
        function = decode_function_type_expression(reader)

        return TypeExpressionFunction(function=function)
    elif variant == 11:
        constructor = decode_constructor_type(reader)

        return TypeExpressionConstructor(constructor=constructor)
    elif variant == 12:
        path = destack._generated.dir.tree.path.decode_path(reader)
        generic_arguments = [
            destack._generated.dir.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return TypeExpressionReference(
            path=path,
            generic_arguments=generic_arguments,
        )
    elif variant == 13:
        left = destack._generated.dir.tree.node.decode_local_node_id(reader)
        name = destack._generated.core.string.decode_string_id(reader)
        generic_arguments = [
            destack._generated.dir.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return TypeExpressionMember(
            left=left,
            name=name,
            generic_arguments=generic_arguments,
        )
    elif variant == 14:
        start = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )
        end = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )
        end_kind = destack._generated.dir.tree.operator.decode_range_end(reader)

        return TypeExpressionRange(
            start=start,
            end=end,
            end_kind=end_kind,
        )
    elif variant == 15:
        return TypeExpressionConst()
    elif variant == 16:
        return TypeExpressionThis()
    elif variant == 17:
        target_type = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return TypeExpressionReadonly(
            target_type=target_type,
        )
    elif variant == 18:
        target_type = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return TypeExpressionLocal(
            target_type=target_type,
        )
    elif variant == 19:
        target_type = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return TypeExpressionShared(
            target_type=target_type,
        )
    elif variant == 20:
        target_type = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return TypeExpressionKeyOf(
            target_type=target_type,
        )
    elif variant == 21:
        value_ = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return TypeExpressionTypeOf(
            value=value_,
        )
    elif variant == 22:
        target_type = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return TypeExpressionMust(
            target_type=target_type,
        )
    elif variant == 23:
        target_type = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return TypeExpressionNot(
            target_type=target_type,
        )
    elif variant == 24:
        mutability = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_mutability(reader)
        )
        variance = reader.read_option(
            lambda: destack._generated.dir.tree.expression.decode_variance_bound(reader)
        )
        target_type = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return TypeExpressionOwnedOf(
            mutability=mutability,
            variance=variance,
            target_type=target_type,
        )
    elif variant == 25:
        mutability = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_mutability(reader)
        )
        variance = reader.read_option(
            lambda: destack._generated.dir.tree.expression.decode_variance_bound(reader)
        )
        target_type = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return TypeExpressionBorrowedOf(
            mutability=mutability,
            variance=variance,
            target_type=target_type,
        )
    elif variant == 26:
        mutability = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_mutability(reader)
        )
        target_type = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return TypeExpressionPointerOf(
            mutability=mutability,
            target_type=target_type,
        )
    elif variant == 27:
        elements = [
            destack._generated.dir.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return TypeExpressionUnion(
            elements=elements,
        )
    elif variant == 28:
        elements = [
            destack._generated.dir.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return TypeExpressionIntersection(
            elements=elements,
        )
    elif variant == 29:
        left = destack._generated.dir.tree.node.decode_local_node_id(reader)
        extends_type = destack._generated.dir.tree.node.decode_local_node_id(reader)
        then_type = destack._generated.dir.tree.node.decode_local_node_id(reader)
        else_type = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return TypeExpressionConditional(
            left=left,
            extends_type=extends_type,
            then_type=then_type,
            else_type=else_type,
        )
    elif variant == 30:
        left = destack._generated.dir.tree.node.decode_local_node_id(reader)
        right = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return TypeExpressionExtends(
            left=left,
            right=right,
        )
    elif variant == 31:
        left = destack._generated.dir.tree.node.decode_local_node_id(reader)
        right = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return TypeExpressionImplements(
            left=left,
            right=right,
        )
    elif variant == 32:
        parameter = destack._generated.dir.tree.node.decode_local_node_id(reader)
        readonly = decode_mapped_type_modifier(reader)
        optional = decode_mapped_type_modifier(reader)
        value_ = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )

        return TypeExpressionMapped(
            parameter=parameter,
            readonly=readonly,
            optional=optional,
            value=value_,
        )
    elif variant == 33:
        left = destack._generated.dir.tree.node.decode_local_node_id(reader)
        index = destack._generated.dir.tree.node.decode_local_node_id(reader)

        return TypeExpressionIndex(
            left=left,
            index=index,
        )
    elif variant == 34:
        strings = [
            destack._generated.core.string.decode_string_id(reader)
            for _ in range(reader.read_number())
        ]
        spans = [
            destack._generated.dir.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return TypeExpressionTemplateLiteral(
            strings=strings,
            spans=spans,
        )
    elif variant == 35:
        form = decode_infer_form(reader)
        name = reader.read_option(
            lambda: destack._generated.core.string.decode_string_id(reader)
        )
        constraint = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )

        return TypeExpressionInfer(
            form=form,
            name=name,
            constraint=constraint,
        )
    elif variant == 36:
        return TypeExpressionMissing()
    elif variant == 37:
        return TypeExpressionError()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_type_expression(value: TypeExpression) -> Json:
    """Return one JSON value for one TypeExpression."""
    if value.kind == "parenthesized":
        return {
            "kind": "parenthesized",
            "expression": destack._generated.dir.tree.node.to_json_local_node_id(
                value.expression
            ),
        }
    elif value.kind == "scalarLiteral":
        return {
            "kind": "scalarLiteral",
            "value": destack._generated.dir.tree.literal.to_json_scalar_literal(
                value.value
            ),
        }
    elif value.kind == "literal":
        return {
            "kind": "literal",
            "value": destack._generated.dir.tree.literal.to_json_type_literal(
                value.value
            ),
        }
    elif value.kind == "intrinsic":
        return {
            "kind": "intrinsic",
        }
    elif value.kind == "tuple":
        return {
            "kind": "tuple",
            "elements": [
                destack._generated.dir.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.elements
            ],
        }
    elif value.kind == "arrayTuple":
        return {
            "kind": "arrayTuple",
            "elements": [
                destack._generated.dir.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.elements
            ],
        }
    elif value.kind == "array":
        return {
            "kind": "array",
            "element": destack._generated.dir.tree.node.to_json_local_node_id(
                value.element
            ),
        }
    elif value.kind == "slice":
        return {
            "kind": "slice",
            "element": destack._generated.dir.tree.node.to_json_local_node_id(
                value.element
            ),
        }
    elif value.kind == "fixedArray":
        return {
            "kind": "fixedArray",
            "element": destack._generated.dir.tree.node.to_json_local_node_id(
                value.element
            ),
            "length": destack._generated.dir.tree.node.to_json_local_node_id(
                value.length
            ),
        }
    elif value.kind == "object":
        return {
            "kind": "object",
            "members": [
                destack._generated.dir.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.members
            ],
        }
    elif value.kind == "function":
        return {
            "kind": "function",
            "function": to_json_function_type_expression(value.function),
        }
    elif value.kind == "constructor":
        return {
            "kind": "constructor",
            "constructor": to_json_constructor_type(value.constructor),
        }
    elif value.kind == "reference":
        return {
            "kind": "reference",
            "path": destack._generated.dir.tree.path.to_json_path(value.path),
            "genericArguments": [
                destack._generated.dir.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.generic_arguments
            ],
        }
    elif value.kind == "member":
        return {
            "kind": "member",
            "left": destack._generated.dir.tree.node.to_json_local_node_id(value.left),
            "name": destack._generated.core.string.to_json_string_id(value.name),
            "genericArguments": [
                destack._generated.dir.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.generic_arguments
            ],
        }
    elif value.kind == "range":
        return {
            "kind": "range",
            **(
                {}
                if value.start is None
                else {
                    "start": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.start
                    )
                }
            ),
            **(
                {}
                if value.end is None
                else {
                    "end": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.end
                    )
                }
            ),
            "endKind": destack._generated.dir.tree.operator.to_json_range_end(
                value.end_kind
            ),
        }
    elif value.kind == "const":
        return {
            "kind": "const",
        }
    elif value.kind == "this":
        return {
            "kind": "this",
        }
    elif value.kind == "readonly":
        return {
            "kind": "readonly",
            "targetType": destack._generated.dir.tree.node.to_json_local_node_id(
                value.target_type
            ),
        }
    elif value.kind == "local":
        return {
            "kind": "local",
            "targetType": destack._generated.dir.tree.node.to_json_local_node_id(
                value.target_type
            ),
        }
    elif value.kind == "shared":
        return {
            "kind": "shared",
            "targetType": destack._generated.dir.tree.node.to_json_local_node_id(
                value.target_type
            ),
        }
    elif value.kind == "keyOf":
        return {
            "kind": "keyOf",
            "targetType": destack._generated.dir.tree.node.to_json_local_node_id(
                value.target_type
            ),
        }
    elif value.kind == "typeOf":
        return {
            "kind": "typeOf",
            "value": destack._generated.dir.tree.node.to_json_local_node_id(
                value.value
            ),
        }
    elif value.kind == "must":
        return {
            "kind": "must",
            "targetType": destack._generated.dir.tree.node.to_json_local_node_id(
                value.target_type
            ),
        }
    elif value.kind == "not":
        return {
            "kind": "not",
            "targetType": destack._generated.dir.tree.node.to_json_local_node_id(
                value.target_type
            ),
        }
    elif value.kind == "ownedOf":
        return {
            "kind": "ownedOf",
            **(
                {}
                if value.mutability is None
                else {
                    "mutability": destack._generated.dir.tree.node.to_json_mutability(
                        value.mutability
                    )
                }
            ),
            **(
                {}
                if value.variance is None
                else {
                    "variance": destack._generated.dir.tree.expression.to_json_variance_bound(
                        value.variance
                    )
                }
            ),
            "targetType": destack._generated.dir.tree.node.to_json_local_node_id(
                value.target_type
            ),
        }
    elif value.kind == "borrowedOf":
        return {
            "kind": "borrowedOf",
            **(
                {}
                if value.mutability is None
                else {
                    "mutability": destack._generated.dir.tree.node.to_json_mutability(
                        value.mutability
                    )
                }
            ),
            **(
                {}
                if value.variance is None
                else {
                    "variance": destack._generated.dir.tree.expression.to_json_variance_bound(
                        value.variance
                    )
                }
            ),
            "targetType": destack._generated.dir.tree.node.to_json_local_node_id(
                value.target_type
            ),
        }
    elif value.kind == "pointerOf":
        return {
            "kind": "pointerOf",
            **(
                {}
                if value.mutability is None
                else {
                    "mutability": destack._generated.dir.tree.node.to_json_mutability(
                        value.mutability
                    )
                }
            ),
            "targetType": destack._generated.dir.tree.node.to_json_local_node_id(
                value.target_type
            ),
        }
    elif value.kind == "union":
        return {
            "kind": "union",
            "elements": [
                destack._generated.dir.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.elements
            ],
        }
    elif value.kind == "intersection":
        return {
            "kind": "intersection",
            "elements": [
                destack._generated.dir.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.elements
            ],
        }
    elif value.kind == "conditional":
        return {
            "kind": "conditional",
            "left": destack._generated.dir.tree.node.to_json_local_node_id(value.left),
            "extendsType": destack._generated.dir.tree.node.to_json_local_node_id(
                value.extends_type
            ),
            "thenType": destack._generated.dir.tree.node.to_json_local_node_id(
                value.then_type
            ),
            "elseType": destack._generated.dir.tree.node.to_json_local_node_id(
                value.else_type
            ),
        }
    elif value.kind == "extends":
        return {
            "kind": "extends",
            "left": destack._generated.dir.tree.node.to_json_local_node_id(value.left),
            "right": destack._generated.dir.tree.node.to_json_local_node_id(
                value.right
            ),
        }
    elif value.kind == "implements":
        return {
            "kind": "implements",
            "left": destack._generated.dir.tree.node.to_json_local_node_id(value.left),
            "right": destack._generated.dir.tree.node.to_json_local_node_id(
                value.right
            ),
        }
    elif value.kind == "mapped":
        return {
            "kind": "mapped",
            "parameter": destack._generated.dir.tree.node.to_json_local_node_id(
                value.parameter
            ),
            "readonly": to_json_mapped_type_modifier(value.readonly),
            "optional": to_json_mapped_type_modifier(value.optional),
            **(
                {}
                if value.value is None
                else {
                    "value": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.value
                    )
                }
            ),
        }
    elif value.kind == "index":
        return {
            "kind": "index",
            "left": destack._generated.dir.tree.node.to_json_local_node_id(value.left),
            "index": destack._generated.dir.tree.node.to_json_local_node_id(
                value.index
            ),
        }
    elif value.kind == "templateLiteral":
        return {
            "kind": "templateLiteral",
            "strings": [
                destack._generated.core.string.to_json_string_id(item_0)
                for item_0 in value.strings
            ],
            "spans": [
                destack._generated.dir.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.spans
            ],
        }
    elif value.kind == "infer":
        return {
            "kind": "infer",
            "form": to_json_infer_form(value.form),
            **(
                {}
                if value.name is None
                else {
                    "name": destack._generated.core.string.to_json_string_id(value.name)
                }
            ),
            **(
                {}
                if value.constraint is None
                else {
                    "constraint": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.constraint
                    )
                }
            ),
        }
    elif value.kind == "missing":
        return {
            "kind": "missing",
        }
    elif value.kind == "error":
        return {
            "kind": "error",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_type_expression(value: Json) -> TypeExpression:
    """Return one TypeExpression from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "parenthesized":
        return TypeExpressionParenthesized(
            expression=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "expression")
            ),
        )
    elif kind == "scalarLiteral":
        return TypeExpressionScalarLiteral(
            value=destack._generated.dir.tree.literal.from_json_scalar_literal(
                json_field(object_, "value")
            ),
        )
    elif kind == "literal":
        return TypeExpressionLiteral(
            value=destack._generated.dir.tree.literal.from_json_type_literal(
                json_field(object_, "value")
            ),
        )
    elif kind == "intrinsic":
        return TypeExpressionIntrinsic()
    elif kind == "tuple":
        return TypeExpressionTuple(
            elements=[
                destack._generated.dir.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "elements"))
            ],
        )
    elif kind == "arrayTuple":
        return TypeExpressionArrayTuple(
            elements=[
                destack._generated.dir.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "elements"))
            ],
        )
    elif kind == "array":
        return TypeExpressionArray(
            element=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "element")
            ),
        )
    elif kind == "slice":
        return TypeExpressionSlice(
            element=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "element")
            ),
        )
    elif kind == "fixedArray":
        return TypeExpressionFixedArray(
            element=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "element")
            ),
            length=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "length")
            ),
        )
    elif kind == "object":
        return TypeExpressionObject(
            members=[
                destack._generated.dir.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "members"))
            ],
        )
    elif kind == "function":
        return TypeExpressionFunction(
            function=from_json_function_type_expression(json_field(object_, "function"))
        )
    elif kind == "constructor":
        return TypeExpressionConstructor(
            constructor=from_json_constructor_type(json_field(object_, "constructor"))
        )
    elif kind == "reference":
        return TypeExpressionReference(
            path=destack._generated.dir.tree.path.from_json_path(
                json_field(object_, "path")
            ),
            generic_arguments=[
                destack._generated.dir.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "genericArguments"))
            ],
        )
    elif kind == "member":
        return TypeExpressionMember(
            left=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "left")
            ),
            name=destack._generated.core.string.from_json_string_id(
                json_field(object_, "name")
            ),
            generic_arguments=[
                destack._generated.dir.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "genericArguments"))
            ],
        )
    elif kind == "range":
        return TypeExpressionRange(
            start=json_optional(
                object_,
                "start",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                    value
                ),
            ),
            end=json_optional(
                object_,
                "end",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                    value
                ),
            ),
            end_kind=destack._generated.dir.tree.operator.from_json_range_end(
                json_field(object_, "endKind")
            ),
        )
    elif kind == "const":
        return TypeExpressionConst()
    elif kind == "this":
        return TypeExpressionThis()
    elif kind == "readonly":
        return TypeExpressionReadonly(
            target_type=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "targetType")
            ),
        )
    elif kind == "local":
        return TypeExpressionLocal(
            target_type=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "targetType")
            ),
        )
    elif kind == "shared":
        return TypeExpressionShared(
            target_type=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "targetType")
            ),
        )
    elif kind == "keyOf":
        return TypeExpressionKeyOf(
            target_type=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "targetType")
            ),
        )
    elif kind == "typeOf":
        return TypeExpressionTypeOf(
            value=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
        )
    elif kind == "must":
        return TypeExpressionMust(
            target_type=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "targetType")
            ),
        )
    elif kind == "not":
        return TypeExpressionNot(
            target_type=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "targetType")
            ),
        )
    elif kind == "ownedOf":
        return TypeExpressionOwnedOf(
            mutability=json_optional(
                object_,
                "mutability",
                lambda value: destack._generated.dir.tree.node.from_json_mutability(
                    value
                ),
            ),
            variance=json_optional(
                object_,
                "variance",
                lambda value: (
                    destack._generated.dir.tree.expression.from_json_variance_bound(
                        value
                    )
                ),
            ),
            target_type=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "targetType")
            ),
        )
    elif kind == "borrowedOf":
        return TypeExpressionBorrowedOf(
            mutability=json_optional(
                object_,
                "mutability",
                lambda value: destack._generated.dir.tree.node.from_json_mutability(
                    value
                ),
            ),
            variance=json_optional(
                object_,
                "variance",
                lambda value: (
                    destack._generated.dir.tree.expression.from_json_variance_bound(
                        value
                    )
                ),
            ),
            target_type=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "targetType")
            ),
        )
    elif kind == "pointerOf":
        return TypeExpressionPointerOf(
            mutability=json_optional(
                object_,
                "mutability",
                lambda value: destack._generated.dir.tree.node.from_json_mutability(
                    value
                ),
            ),
            target_type=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "targetType")
            ),
        )
    elif kind == "union":
        return TypeExpressionUnion(
            elements=[
                destack._generated.dir.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "elements"))
            ],
        )
    elif kind == "intersection":
        return TypeExpressionIntersection(
            elements=[
                destack._generated.dir.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "elements"))
            ],
        )
    elif kind == "conditional":
        return TypeExpressionConditional(
            left=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "left")
            ),
            extends_type=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "extendsType")
            ),
            then_type=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "thenType")
            ),
            else_type=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "elseType")
            ),
        )
    elif kind == "extends":
        return TypeExpressionExtends(
            left=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "left")
            ),
            right=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "right")
            ),
        )
    elif kind == "implements":
        return TypeExpressionImplements(
            left=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "left")
            ),
            right=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "right")
            ),
        )
    elif kind == "mapped":
        return TypeExpressionMapped(
            parameter=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "parameter")
            ),
            readonly=from_json_mapped_type_modifier(json_field(object_, "readonly")),
            optional=from_json_mapped_type_modifier(json_field(object_, "optional")),
            value=json_optional(
                object_,
                "value",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                    value
                ),
            ),
        )
    elif kind == "index":
        return TypeExpressionIndex(
            left=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "left")
            ),
            index=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "index")
            ),
        )
    elif kind == "templateLiteral":
        return TypeExpressionTemplateLiteral(
            strings=[
                destack._generated.core.string.from_json_string_id(item_0)
                for item_0 in json_array(json_field(object_, "strings"))
            ],
            spans=[
                destack._generated.dir.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "spans"))
            ],
        )
    elif kind == "infer":
        return TypeExpressionInfer(
            form=from_json_infer_form(json_field(object_, "form")),
            name=json_optional(
                object_,
                "name",
                lambda value: destack._generated.core.string.from_json_string_id(value),
            ),
            constraint=json_optional(
                object_,
                "constraint",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                    value
                ),
            ),
        )
    elif kind == "missing":
        return TypeExpressionMissing()
    elif kind == "error":
        return TypeExpressionError()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_function_type_expression(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionTypeExpression:
        """Decode one FunctionTypeExpression."""
        return decode_function_type_expression(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_function_type_expression(self)

    @classmethod
    def from_json(cls, value: Json) -> FunctionTypeExpression:
        """Return one FunctionTypeExpression from one JSON value."""
        return from_json_function_type_expression(value)


def encode_function_type_expression(
    writer: BinaryWriter, value: FunctionTypeExpression
) -> None:
    """Encode one FunctionTypeExpression."""
    writer.write_unsigned(len(value.generic_parameters))
    for item_value_generic_parameters_0 in value.generic_parameters:
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, item_value_generic_parameters_0
        )
    writer.write_unsigned(len(value.where_clauses))
    for item_value_where_clauses_0 in value.where_clauses:
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, item_value_where_clauses_0
        )
    if value.this_form is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.function.encode_this_form(writer, value.this_form)
    if value.this_parameter is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, value.this_parameter
        )
    writer.write_unsigned(len(value.parameters))
    for item_value_parameters_0 in value.parameters:
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, item_value_parameters_0
        )
    if value.return_type is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.return_type)


def decode_function_type_expression(reader: BinaryReader) -> FunctionTypeExpression:
    """Decode one FunctionTypeExpression."""
    generic_parameters = [
        destack._generated.dir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    where_clauses = [
        destack._generated.dir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    this_form = reader.read_option(
        lambda: destack._generated.dir.tree.function.decode_this_form(reader)
    )
    this_parameter = reader.read_option(
        lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
    )
    parameters = [
        destack._generated.dir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    return_type = reader.read_option(
        lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
    )

    return FunctionTypeExpression(
        generic_parameters=generic_parameters,
        where_clauses=where_clauses,
        this_form=this_form,
        this_parameter=this_parameter,
        parameters=parameters,
        return_type=return_type,
    )


def to_json_function_type_expression(value: FunctionTypeExpression) -> Json:
    """Return one JSON value for one FunctionTypeExpression."""
    return {
        "genericParameters": [
            destack._generated.dir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.generic_parameters
        ],
        "whereClauses": [
            destack._generated.dir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.where_clauses
        ],
        **(
            {}
            if value.this_form is None
            else {
                "thisForm": destack._generated.dir.tree.function.to_json_this_form(
                    value.this_form
                )
            }
        ),
        **(
            {}
            if value.this_parameter is None
            else {
                "thisParameter": destack._generated.dir.tree.node.to_json_local_node_id(
                    value.this_parameter
                )
            }
        ),
        "parameters": [
            destack._generated.dir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.parameters
        ],
        **(
            {}
            if value.return_type is None
            else {
                "returnType": destack._generated.dir.tree.node.to_json_local_node_id(
                    value.return_type
                )
            }
        ),
    }


def from_json_function_type_expression(value: Json) -> FunctionTypeExpression:
    """Return one FunctionTypeExpression from one JSON value."""
    object_ = json_object(value)

    return FunctionTypeExpression(
        generic_parameters=[
            destack._generated.dir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "genericParameters"))
        ],
        where_clauses=[
            destack._generated.dir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "whereClauses"))
        ],
        this_form=json_optional(
            object_,
            "thisForm",
            lambda value: destack._generated.dir.tree.function.from_json_this_form(
                value
            ),
        ),
        this_parameter=json_optional(
            object_,
            "thisParameter",
            lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                value
            ),
        ),
        parameters=[
            destack._generated.dir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "parameters"))
        ],
        return_type=json_optional(
            object_,
            "returnType",
            lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                value
            ),
        ),
    )


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_constructor_type(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ConstructorType:
        """Decode one ConstructorType."""
        return decode_constructor_type(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_constructor_type(self)

    @classmethod
    def from_json(cls, value: Json) -> ConstructorType:
        """Return one ConstructorType from one JSON value."""
        return from_json_constructor_type(value)


def encode_constructor_type(writer: BinaryWriter, value: ConstructorType) -> None:
    """Encode one ConstructorType."""
    writer.write_unsigned(len(value.generic_parameters))
    for item_value_generic_parameters_0 in value.generic_parameters:
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, item_value_generic_parameters_0
        )
    writer.write_unsigned(len(value.where_clauses))
    for item_value_where_clauses_0 in value.where_clauses:
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, item_value_where_clauses_0
        )
    writer.write_unsigned(len(value.parameters))
    for item_value_parameters_0 in value.parameters:
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, item_value_parameters_0
        )
    if value.return_type is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.return_type)
    writer.write_bool(value.is_abstract)


def decode_constructor_type(reader: BinaryReader) -> ConstructorType:
    """Decode one ConstructorType."""
    generic_parameters = [
        destack._generated.dir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    where_clauses = [
        destack._generated.dir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    parameters = [
        destack._generated.dir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    return_type = reader.read_option(
        lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
    )
    is_abstract = reader.read_bool()

    return ConstructorType(
        generic_parameters=generic_parameters,
        where_clauses=where_clauses,
        parameters=parameters,
        return_type=return_type,
        is_abstract=is_abstract,
    )


def to_json_constructor_type(value: ConstructorType) -> Json:
    """Return one JSON value for one ConstructorType."""
    return {
        "genericParameters": [
            destack._generated.dir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.generic_parameters
        ],
        "whereClauses": [
            destack._generated.dir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.where_clauses
        ],
        "parameters": [
            destack._generated.dir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.parameters
        ],
        **(
            {}
            if value.return_type is None
            else {
                "returnType": destack._generated.dir.tree.node.to_json_local_node_id(
                    value.return_type
                )
            }
        ),
        "isAbstract": value.is_abstract,
    }


def from_json_constructor_type(value: Json) -> ConstructorType:
    """Return one ConstructorType from one JSON value."""
    object_ = json_object(value)

    return ConstructorType(
        generic_parameters=[
            destack._generated.dir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "genericParameters"))
        ],
        where_clauses=[
            destack._generated.dir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "whereClauses"))
        ],
        parameters=[
            destack._generated.dir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "parameters"))
        ],
        return_type=json_optional(
            object_,
            "returnType",
            lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                value
            ),
        ),
        is_abstract=json_bool(json_field(object_, "isAbstract")),
    )


"""A mapped-type modifier sign."""
MappedTypeModifier: typing.TypeAlias = (
    typing.Literal["present"]
    | typing.Literal["add"]
    | typing.Literal["remove"]
    | typing.Literal["none"]
)


def encode_mapped_type_modifier(
    writer: BinaryWriter, value: MappedTypeModifier
) -> None:
    """Encode one MappedTypeModifier."""
    if value == "present":
        writer.write_unsigned(0)
    elif value == "add":
        writer.write_unsigned(1)
    elif value == "remove":
        writer.write_unsigned(2)
    elif value == "none":
        writer.write_unsigned(3)
    else:
        raise SerdeError("unknown enum variant")


def decode_mapped_type_modifier(reader: BinaryReader) -> MappedTypeModifier:
    """Decode one MappedTypeModifier."""
    variant = reader.read_number()

    if variant == 0:
        return "present"
    elif variant == 1:
        return "add"
    elif variant == 2:
        return "remove"
    elif variant == 3:
        return "none"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_mapped_type_modifier(value: MappedTypeModifier) -> Json:
    """Return one JSON value for one MappedTypeModifier."""
    return value


def from_json_mapped_type_modifier(value: Json) -> MappedTypeModifier:
    """Return one MappedTypeModifier from one JSON value."""
    variant = json_string(value)

    if variant == "present":
        return "present"
    elif variant == "add":
        return "add"
    elif variant == "remove":
        return "remove"
    elif variant == "none":
        return "none"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""The parsed form of an infer type expression."""
InferForm: typing.TypeAlias = typing.Literal["hole"] | typing.Literal["infer"]


def encode_infer_form(writer: BinaryWriter, value: InferForm) -> None:
    """Encode one InferForm."""
    if value == "hole":
        writer.write_unsigned(0)
    elif value == "infer":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_infer_form(reader: BinaryReader) -> InferForm:
    """Decode one InferForm."""
    variant = reader.read_number()

    if variant == 0:
        return "hole"
    elif variant == 1:
        return "infer"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_infer_form(value: InferForm) -> Json:
    """Return one JSON value for one InferForm."""
    return value


def from_json_infer_form(value: Json) -> InferForm:
    """Return one InferForm from one JSON value."""
    variant = json_string(value)

    if variant == "hole":
        return "hole"
    elif variant == "infer":
        return "infer"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class TypeMemberField:
    """Named field."""

    key: destack._generated.dir.tree.key.Key
    declared_type: destack._generated.dir.tree.node.LocalNodeId | None
    is_static: bool
    is_optional: bool
    is_readonly: bool
    kind: typing.Literal["field"] = "field"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_member(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_member(self)


@dataclass(frozen=True, slots=True)
class TypeMemberMethod:
    """Named method."""

    key: destack._generated.dir.tree.key.Key
    signature: destack._generated.dir.tree.function.FunctionSignature
    body: destack._generated.dir.tree.node.LocalNodeId | None
    is_static: bool
    is_optional: bool
    kind: typing.Literal["method"] = "method"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_member(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_member(self)


@dataclass(frozen=True, slots=True)
class TypeMemberCallSignature:
    """Call signature declaration."""

    # the call signature function type expression
    signature: FunctionTypeExpression
    kind: typing.Literal["callSignature"] = "callSignature"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_member(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_member(self)


@dataclass(frozen=True, slots=True)
class TypeMemberConstructSignature:
    """Construct signature declaration."""

    signature: ConstructorType
    kind: typing.Literal["constructSignature"] = "constructSignature"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_member(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_member(self)


@dataclass(frozen=True, slots=True)
class TypeMemberIndexSignature:
    """Index signature."""

    name: destack._generated.core.string.StringId
    key_type: destack._generated.dir.tree.node.LocalNodeId
    value_type: destack._generated.dir.tree.node.LocalNodeId
    is_optional: bool
    is_readonly: bool
    kind: typing.Literal["indexSignature"] = "indexSignature"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_member(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_member(self)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_member(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_member(self)


@dataclass(frozen=True, slots=True)
class TypeMemberAssociatedConst:
    """Associated compile-time constant requirement or definition."""

    name: destack._generated.core.string.StringId
    declared_type: destack._generated.dir.tree.node.LocalNodeId | None
    value: destack._generated.dir.tree.node.LocalNodeId | None
    is_abstract: bool
    is_override: bool
    kind: typing.Literal["associatedConst"] = "associatedConst"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_member(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_member(self)


@dataclass(frozen=True, slots=True)
class TypeMemberError:
    """Malformed type member slot."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_member(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_member(self)


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


def encode_type_member(writer: BinaryWriter, value: TypeMember) -> None:
    """Encode one TypeMember."""
    if value.kind == "field":
        writer.write_unsigned(0)
        destack._generated.dir.tree.key.encode_key(writer, value.key)
        if value.declared_type is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, value.declared_type
            )
        writer.write_bool(value.is_static)
        writer.write_bool(value.is_optional)
        writer.write_bool(value.is_readonly)
    elif value.kind == "method":
        writer.write_unsigned(1)
        destack._generated.dir.tree.key.encode_key(writer, value.key)
        destack._generated.dir.tree.function.encode_function_signature(
            writer, value.signature
        )
        if value.body is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(writer, value.body)
        writer.write_bool(value.is_static)
        writer.write_bool(value.is_optional)
    elif value.kind == "callSignature":
        writer.write_unsigned(2)
        encode_function_type_expression(writer, value.signature)
    elif value.kind == "constructSignature":
        writer.write_unsigned(3)
        encode_constructor_type(writer, value.signature)
    elif value.kind == "indexSignature":
        writer.write_unsigned(4)
        destack._generated.core.string.encode_string_id(writer, value.name)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.key_type)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.value_type)
        writer.write_bool(value.is_optional)
        writer.write_bool(value.is_readonly)
    elif value.kind == "associatedType":
        writer.write_unsigned(5)
        destack._generated.core.string.encode_string_id(writer, value.name)
        writer.write_unsigned(len(value.generic_parameters))
        for item_value_generic_parameters_0 in value.generic_parameters:
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, item_value_generic_parameters_0
            )
        writer.write_unsigned(len(value.where_clauses))
        for item_value_where_clauses_0 in value.where_clauses:
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, item_value_where_clauses_0
            )
        if value.constraint is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, value.constraint
            )
        if value.value is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(writer, value.value)
        writer.write_bool(value.is_abstract)
        writer.write_bool(value.is_override)
    elif value.kind == "associatedConst":
        writer.write_unsigned(6)
        destack._generated.core.string.encode_string_id(writer, value.name)
        if value.declared_type is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, value.declared_type
            )
        if value.value is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(writer, value.value)
        writer.write_bool(value.is_abstract)
        writer.write_bool(value.is_override)
    elif value.kind == "error":
        writer.write_unsigned(7)
    else:
        raise SerdeError("unknown enum variant")


def decode_type_member(reader: BinaryReader) -> TypeMember:
    """Decode one TypeMember."""
    variant = reader.read_number()

    if variant == 0:
        key = destack._generated.dir.tree.key.decode_key(reader)
        declared_type = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )
        is_static = reader.read_bool()
        is_optional = reader.read_bool()
        is_readonly = reader.read_bool()

        return TypeMemberField(
            key=key,
            declared_type=declared_type,
            is_static=is_static,
            is_optional=is_optional,
            is_readonly=is_readonly,
        )
    elif variant == 1:
        key = destack._generated.dir.tree.key.decode_key(reader)
        signature = destack._generated.dir.tree.function.decode_function_signature(
            reader
        )
        body = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )
        is_static = reader.read_bool()
        is_optional = reader.read_bool()

        return TypeMemberMethod(
            key=key,
            signature=signature,
            body=body,
            is_static=is_static,
            is_optional=is_optional,
        )
    elif variant == 2:
        signature = decode_function_type_expression(reader)

        return TypeMemberCallSignature(
            signature=signature,
        )
    elif variant == 3:
        signature = decode_constructor_type(reader)

        return TypeMemberConstructSignature(
            signature=signature,
        )
    elif variant == 4:
        name = destack._generated.core.string.decode_string_id(reader)
        key_type = destack._generated.dir.tree.node.decode_local_node_id(reader)
        value_type = destack._generated.dir.tree.node.decode_local_node_id(reader)
        is_optional = reader.read_bool()
        is_readonly = reader.read_bool()

        return TypeMemberIndexSignature(
            name=name,
            key_type=key_type,
            value_type=value_type,
            is_optional=is_optional,
            is_readonly=is_readonly,
        )
    elif variant == 5:
        name = destack._generated.core.string.decode_string_id(reader)
        generic_parameters = [
            destack._generated.dir.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]
        where_clauses = [
            destack._generated.dir.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]
        constraint = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )
        value_ = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )
        is_abstract = reader.read_bool()
        is_override = reader.read_bool()

        return TypeMemberAssociatedType(
            name=name,
            generic_parameters=generic_parameters,
            where_clauses=where_clauses,
            constraint=constraint,
            value=value_,
            is_abstract=is_abstract,
            is_override=is_override,
        )
    elif variant == 6:
        name = destack._generated.core.string.decode_string_id(reader)
        declared_type = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )
        value_ = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )
        is_abstract = reader.read_bool()
        is_override = reader.read_bool()

        return TypeMemberAssociatedConst(
            name=name,
            declared_type=declared_type,
            value=value_,
            is_abstract=is_abstract,
            is_override=is_override,
        )
    elif variant == 7:
        return TypeMemberError()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_type_member(value: TypeMember) -> Json:
    """Return one JSON value for one TypeMember."""
    if value.kind == "field":
        return {
            "kind": "field",
            "key": destack._generated.dir.tree.key.to_json_key(value.key),
            **(
                {}
                if value.declared_type is None
                else {
                    "declaredType": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.declared_type
                    )
                }
            ),
            "isStatic": value.is_static,
            "isOptional": value.is_optional,
            "isReadonly": value.is_readonly,
        }
    elif value.kind == "method":
        return {
            "kind": "method",
            "key": destack._generated.dir.tree.key.to_json_key(value.key),
            "signature": destack._generated.dir.tree.function.to_json_function_signature(
                value.signature
            ),
            **(
                {}
                if value.body is None
                else {
                    "body": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.body
                    )
                }
            ),
            "isStatic": value.is_static,
            "isOptional": value.is_optional,
        }
    elif value.kind == "callSignature":
        return {
            "kind": "callSignature",
            "signature": to_json_function_type_expression(value.signature),
        }
    elif value.kind == "constructSignature":
        return {
            "kind": "constructSignature",
            "signature": to_json_constructor_type(value.signature),
        }
    elif value.kind == "indexSignature":
        return {
            "kind": "indexSignature",
            "name": destack._generated.core.string.to_json_string_id(value.name),
            "keyType": destack._generated.dir.tree.node.to_json_local_node_id(
                value.key_type
            ),
            "valueType": destack._generated.dir.tree.node.to_json_local_node_id(
                value.value_type
            ),
            "isOptional": value.is_optional,
            "isReadonly": value.is_readonly,
        }
    elif value.kind == "associatedType":
        return {
            "kind": "associatedType",
            "name": destack._generated.core.string.to_json_string_id(value.name),
            "genericParameters": [
                destack._generated.dir.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.generic_parameters
            ],
            "whereClauses": [
                destack._generated.dir.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.where_clauses
            ],
            **(
                {}
                if value.constraint is None
                else {
                    "constraint": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.constraint
                    )
                }
            ),
            **(
                {}
                if value.value is None
                else {
                    "value": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.value
                    )
                }
            ),
            "isAbstract": value.is_abstract,
            "isOverride": value.is_override,
        }
    elif value.kind == "associatedConst":
        return {
            "kind": "associatedConst",
            "name": destack._generated.core.string.to_json_string_id(value.name),
            **(
                {}
                if value.declared_type is None
                else {
                    "declaredType": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.declared_type
                    )
                }
            ),
            **(
                {}
                if value.value is None
                else {
                    "value": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.value
                    )
                }
            ),
            "isAbstract": value.is_abstract,
            "isOverride": value.is_override,
        }
    elif value.kind == "error":
        return {
            "kind": "error",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_type_member(value: Json) -> TypeMember:
    """Return one TypeMember from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "field":
        return TypeMemberField(
            key=destack._generated.dir.tree.key.from_json_key(
                json_field(object_, "key")
            ),
            declared_type=json_optional(
                object_,
                "declaredType",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                    value
                ),
            ),
            is_static=json_bool(json_field(object_, "isStatic")),
            is_optional=json_bool(json_field(object_, "isOptional")),
            is_readonly=json_bool(json_field(object_, "isReadonly")),
        )
    elif kind == "method":
        return TypeMemberMethod(
            key=destack._generated.dir.tree.key.from_json_key(
                json_field(object_, "key")
            ),
            signature=destack._generated.dir.tree.function.from_json_function_signature(
                json_field(object_, "signature")
            ),
            body=json_optional(
                object_,
                "body",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                    value
                ),
            ),
            is_static=json_bool(json_field(object_, "isStatic")),
            is_optional=json_bool(json_field(object_, "isOptional")),
        )
    elif kind == "callSignature":
        return TypeMemberCallSignature(
            signature=from_json_function_type_expression(
                json_field(object_, "signature")
            ),
        )
    elif kind == "constructSignature":
        return TypeMemberConstructSignature(
            signature=from_json_constructor_type(json_field(object_, "signature")),
        )
    elif kind == "indexSignature":
        return TypeMemberIndexSignature(
            name=destack._generated.core.string.from_json_string_id(
                json_field(object_, "name")
            ),
            key_type=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "keyType")
            ),
            value_type=destack._generated.dir.tree.node.from_json_local_node_id(
                json_field(object_, "valueType")
            ),
            is_optional=json_bool(json_field(object_, "isOptional")),
            is_readonly=json_bool(json_field(object_, "isReadonly")),
        )
    elif kind == "associatedType":
        return TypeMemberAssociatedType(
            name=destack._generated.core.string.from_json_string_id(
                json_field(object_, "name")
            ),
            generic_parameters=[
                destack._generated.dir.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "genericParameters"))
            ],
            where_clauses=[
                destack._generated.dir.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "whereClauses"))
            ],
            constraint=json_optional(
                object_,
                "constraint",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                    value
                ),
            ),
            value=json_optional(
                object_,
                "value",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                    value
                ),
            ),
            is_abstract=json_bool(json_field(object_, "isAbstract")),
            is_override=json_bool(json_field(object_, "isOverride")),
        )
    elif kind == "associatedConst":
        return TypeMemberAssociatedConst(
            name=destack._generated.core.string.from_json_string_id(
                json_field(object_, "name")
            ),
            declared_type=json_optional(
                object_,
                "declaredType",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                    value
                ),
            ),
            value=json_optional(
                object_,
                "value",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                    value
                ),
            ),
            is_abstract=json_bool(json_field(object_, "isAbstract")),
            is_override=json_bool(json_field(object_, "isOverride")),
        )
    elif kind == "error":
        return TypeMemberError()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class TypeMappedParameter:
    """A mapped type parameter."""

    # the parameter name
    name: destack._generated.core.string.StringId
    # the source type iterated by `in`
    source_type: destack._generated.dir.tree.node.LocalNodeId
    # the optional key remap
    key_remap: destack._generated.dir.tree.node.LocalNodeId | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_mapped_parameter(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeMappedParameter:
        """Decode one TypeMappedParameter."""
        return decode_type_mapped_parameter(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_mapped_parameter(self)

    @classmethod
    def from_json(cls, value: Json) -> TypeMappedParameter:
        """Return one TypeMappedParameter from one JSON value."""
        return from_json_type_mapped_parameter(value)


def encode_type_mapped_parameter(
    writer: BinaryWriter, value: TypeMappedParameter
) -> None:
    """Encode one TypeMappedParameter."""
    destack._generated.core.string.encode_string_id(writer, value.name)
    destack._generated.dir.tree.node.encode_local_node_id(writer, value.source_type)
    if value.key_remap is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.key_remap)


def decode_type_mapped_parameter(reader: BinaryReader) -> TypeMappedParameter:
    """Decode one TypeMappedParameter."""
    name = destack._generated.core.string.decode_string_id(reader)
    source_type = destack._generated.dir.tree.node.decode_local_node_id(reader)
    key_remap = reader.read_option(
        lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
    )

    return TypeMappedParameter(
        name=name,
        source_type=source_type,
        key_remap=key_remap,
    )


def to_json_type_mapped_parameter(value: TypeMappedParameter) -> Json:
    """Return one JSON value for one TypeMappedParameter."""
    return {
        "name": destack._generated.core.string.to_json_string_id(value.name),
        "sourceType": destack._generated.dir.tree.node.to_json_local_node_id(
            value.source_type
        ),
        **(
            {}
            if value.key_remap is None
            else {
                "keyRemap": destack._generated.dir.tree.node.to_json_local_node_id(
                    value.key_remap
                )
            }
        ),
    }


def from_json_type_mapped_parameter(value: Json) -> TypeMappedParameter:
    """Return one TypeMappedParameter from one JSON value."""
    object_ = json_object(value)

    return TypeMappedParameter(
        name=destack._generated.core.string.from_json_string_id(
            json_field(object_, "name")
        ),
        source_type=destack._generated.dir.tree.node.from_json_local_node_id(
            json_field(object_, "sourceType")
        ),
        key_remap=json_optional(
            object_,
            "keyRemap",
            lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                value
            ),
        ),
    )


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
