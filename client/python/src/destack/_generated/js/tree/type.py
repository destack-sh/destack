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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionThis:
    """This type."""

    kind: typing.Literal["this"] = "this"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionPath:
    """Path to something."""

    path: destack._generated.js.tree.path.Path
    generic_arguments: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["path"] = "path"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionReadonly:
    """`readonly T`."""

    target_type: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["readonly"] = "readonly"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionKeyOf:
    """`keyof T`."""

    target_type: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["keyOf"] = "keyOf"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionMust:
    """`T!`."""

    target_type: destack._generated.js.tree.node.LocalNodeId
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

    target_type: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["not"] = "not"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionExtends:
    """`T extends U`."""

    left: destack._generated.js.tree.node.LocalNodeId
    right: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["extends"] = "extends"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionImplements:
    """`T implements U`."""

    left: destack._generated.js.tree.node.LocalNodeId
    right: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["implements"] = "implements"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionConditional:
    """Conditional type."""

    left: destack._generated.js.tree.node.LocalNodeId
    right: destack._generated.js.tree.node.LocalNodeId
    then_type: destack._generated.js.tree.node.LocalNodeId
    else_type: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["conditional"] = "conditional"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionMapped:
    """Mapped type."""

    parameter: TypeMappedParameter
    modifiers: TypeMappedModifiers
    value: destack._generated.js.tree.node.LocalNodeId | None
    kind: typing.Literal["mapped"] = "mapped"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionIndex:
    """Index access type."""

    left: destack._generated.js.tree.node.LocalNodeId
    index: destack._generated.js.tree.node.LocalNodeId
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

    template_literal: TypeTemplateLiteral
    kind: typing.Literal["templateLiteral"] = "templateLiteral"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionImport:
    """Import type."""

    target: destack._generated.core.string.StringId
    qualifier: destack._generated.js.tree.path.Path | None
    generic_arguments: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["import"] = "import"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionInfer:
    """Infer type binding."""

    name: destack._generated.core.string.StringId
    constraint: destack._generated.js.tree.node.LocalNodeId | None
    kind: typing.Literal["infer"] = "infer"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionArray:
    """Array type `T[]`."""

    element: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["array"] = "array"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionTuple:
    """Tuple type `[T1, T2, ...]`."""

    elements: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["tuple"] = "tuple"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionObject:
    """Object type `{ a: T1, b: T2, ... }`."""

    members: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["object"] = "object"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionUnion:
    """Union type `A | B | C`."""

    elements: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["union"] = "union"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionIntersection:
    """Intersection type `A & B & C`."""

    elements: Sequence[destack._generated.js.tree.node.LocalNodeId]
    kind: typing.Literal["intersection"] = "intersection"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionFunctionTypeDeclaration:
    """Function type declaration."""

    function_type_declaration: FunctionTypeDeclaration
    kind: typing.Literal["functionTypeDeclaration"] = "functionTypeDeclaration"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionConstructorTypeDeclaration:
    """Constructor type declaration."""

    constructor_type_declaration: ConstructorTypeDeclaration
    kind: typing.Literal["constructorTypeDeclaration"] = "constructorTypeDeclaration"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


@dataclass(frozen=True, slots=True)
class TypeExpressionError:
    """Error type that could not be resolved."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_expression(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_expression(self)


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


def encode_type_expression(writer: BinaryWriter, value: TypeExpression) -> None:
    """Encode one TypeExpression."""
    if value.kind == "scalar":
        writer.write_unsigned(0)
        encode_type_literal(writer, value.scalar)
    elif value.kind == "this":
        writer.write_unsigned(1)
    elif value.kind == "path":
        writer.write_unsigned(2)
        destack._generated.js.tree.path.encode_path(writer, value.path)
        writer.write_unsigned(len(value.generic_arguments))
        for item_value_generic_arguments_0 in value.generic_arguments:
            destack._generated.js.tree.node.encode_local_node_id(
                writer, item_value_generic_arguments_0
            )
    elif value.kind == "readonly":
        writer.write_unsigned(3)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.target_type)
    elif value.kind == "keyOf":
        writer.write_unsigned(4)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.target_type)
    elif value.kind == "must":
        writer.write_unsigned(5)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.target_type)
    elif value.kind == "not":
        writer.write_unsigned(6)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.target_type)
    elif value.kind == "extends":
        writer.write_unsigned(7)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.left)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.right)
    elif value.kind == "implements":
        writer.write_unsigned(8)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.left)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.right)
    elif value.kind == "conditional":
        writer.write_unsigned(9)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.left)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.right)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.then_type)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.else_type)
    elif value.kind == "mapped":
        writer.write_unsigned(10)
        encode_type_mapped_parameter(writer, value.parameter)
        encode_type_mapped_modifiers(writer, value.modifiers)
        if value.value is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "index":
        writer.write_unsigned(11)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.left)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.index)
    elif value.kind == "templateLiteral":
        writer.write_unsigned(12)
        encode_type_template_literal(writer, value.template_literal)
    elif value.kind == "import":
        writer.write_unsigned(13)
        destack._generated.core.string.encode_string_id(writer, value.target)
        if value.qualifier is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.path.encode_path(writer, value.qualifier)
        writer.write_unsigned(len(value.generic_arguments))
        for item_value_generic_arguments_0 in value.generic_arguments:
            destack._generated.js.tree.node.encode_local_node_id(
                writer, item_value_generic_arguments_0
            )
    elif value.kind == "infer":
        writer.write_unsigned(14)
        destack._generated.core.string.encode_string_id(writer, value.name)
        if value.constraint is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.node.encode_local_node_id(
                writer, value.constraint
            )
    elif value.kind == "array":
        writer.write_unsigned(15)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.element)
    elif value.kind == "tuple":
        writer.write_unsigned(16)
        writer.write_unsigned(len(value.elements))
        for item_value_elements_0 in value.elements:
            destack._generated.js.tree.node.encode_local_node_id(
                writer, item_value_elements_0
            )
    elif value.kind == "object":
        writer.write_unsigned(17)
        writer.write_unsigned(len(value.members))
        for item_value_members_0 in value.members:
            destack._generated.js.tree.node.encode_local_node_id(
                writer, item_value_members_0
            )
    elif value.kind == "union":
        writer.write_unsigned(18)
        writer.write_unsigned(len(value.elements))
        for item_value_elements_0 in value.elements:
            destack._generated.js.tree.node.encode_local_node_id(
                writer, item_value_elements_0
            )
    elif value.kind == "intersection":
        writer.write_unsigned(19)
        writer.write_unsigned(len(value.elements))
        for item_value_elements_0 in value.elements:
            destack._generated.js.tree.node.encode_local_node_id(
                writer, item_value_elements_0
            )
    elif value.kind == "functionTypeDeclaration":
        writer.write_unsigned(20)
        encode_function_type_declaration(writer, value.function_type_declaration)
    elif value.kind == "constructorTypeDeclaration":
        writer.write_unsigned(21)
        encode_constructor_type_declaration(writer, value.constructor_type_declaration)
    elif value.kind == "error":
        writer.write_unsigned(22)
    else:
        raise SerdeError("unknown enum variant")


def decode_type_expression(reader: BinaryReader) -> TypeExpression:
    """Decode one TypeExpression."""
    variant = reader.read_number()

    if variant == 0:
        scalar = decode_type_literal(reader)

        return TypeExpressionScalar(scalar=scalar)
    elif variant == 1:
        return TypeExpressionThis()
    elif variant == 2:
        path = destack._generated.js.tree.path.decode_path(reader)
        generic_arguments = [
            destack._generated.js.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return TypeExpressionPath(
            path=path,
            generic_arguments=generic_arguments,
        )
    elif variant == 3:
        target_type = destack._generated.js.tree.node.decode_local_node_id(reader)

        return TypeExpressionReadonly(
            target_type=target_type,
        )
    elif variant == 4:
        target_type = destack._generated.js.tree.node.decode_local_node_id(reader)

        return TypeExpressionKeyOf(
            target_type=target_type,
        )
    elif variant == 5:
        target_type = destack._generated.js.tree.node.decode_local_node_id(reader)

        return TypeExpressionMust(
            target_type=target_type,
        )
    elif variant == 6:
        target_type = destack._generated.js.tree.node.decode_local_node_id(reader)

        return TypeExpressionNot(
            target_type=target_type,
        )
    elif variant == 7:
        left = destack._generated.js.tree.node.decode_local_node_id(reader)
        right = destack._generated.js.tree.node.decode_local_node_id(reader)

        return TypeExpressionExtends(
            left=left,
            right=right,
        )
    elif variant == 8:
        left = destack._generated.js.tree.node.decode_local_node_id(reader)
        right = destack._generated.js.tree.node.decode_local_node_id(reader)

        return TypeExpressionImplements(
            left=left,
            right=right,
        )
    elif variant == 9:
        left = destack._generated.js.tree.node.decode_local_node_id(reader)
        right = destack._generated.js.tree.node.decode_local_node_id(reader)
        then_type = destack._generated.js.tree.node.decode_local_node_id(reader)
        else_type = destack._generated.js.tree.node.decode_local_node_id(reader)

        return TypeExpressionConditional(
            left=left,
            right=right,
            then_type=then_type,
            else_type=else_type,
        )
    elif variant == 10:
        parameter = decode_type_mapped_parameter(reader)
        modifiers = decode_type_mapped_modifiers(reader)
        value_ = reader.read_option(
            lambda: destack._generated.js.tree.node.decode_local_node_id(reader)
        )

        return TypeExpressionMapped(
            parameter=parameter,
            modifiers=modifiers,
            value=value_,
        )
    elif variant == 11:
        left = destack._generated.js.tree.node.decode_local_node_id(reader)
        index = destack._generated.js.tree.node.decode_local_node_id(reader)

        return TypeExpressionIndex(
            left=left,
            index=index,
        )
    elif variant == 12:
        template_literal = decode_type_template_literal(reader)

        return TypeExpressionTemplateLiteral(template_literal=template_literal)
    elif variant == 13:
        target = destack._generated.core.string.decode_string_id(reader)
        qualifier = reader.read_option(
            lambda: destack._generated.js.tree.path.decode_path(reader)
        )
        generic_arguments = [
            destack._generated.js.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return TypeExpressionImport(
            target=target,
            qualifier=qualifier,
            generic_arguments=generic_arguments,
        )
    elif variant == 14:
        name = destack._generated.core.string.decode_string_id(reader)
        constraint = reader.read_option(
            lambda: destack._generated.js.tree.node.decode_local_node_id(reader)
        )

        return TypeExpressionInfer(
            name=name,
            constraint=constraint,
        )
    elif variant == 15:
        element = destack._generated.js.tree.node.decode_local_node_id(reader)

        return TypeExpressionArray(
            element=element,
        )
    elif variant == 16:
        elements = [
            destack._generated.js.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return TypeExpressionTuple(
            elements=elements,
        )
    elif variant == 17:
        members = [
            destack._generated.js.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return TypeExpressionObject(
            members=members,
        )
    elif variant == 18:
        elements = [
            destack._generated.js.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return TypeExpressionUnion(
            elements=elements,
        )
    elif variant == 19:
        elements = [
            destack._generated.js.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]

        return TypeExpressionIntersection(
            elements=elements,
        )
    elif variant == 20:
        function_type_declaration = decode_function_type_declaration(reader)

        return TypeExpressionFunctionTypeDeclaration(
            function_type_declaration=function_type_declaration
        )
    elif variant == 21:
        constructor_type_declaration = decode_constructor_type_declaration(reader)

        return TypeExpressionConstructorTypeDeclaration(
            constructor_type_declaration=constructor_type_declaration
        )
    elif variant == 22:
        return TypeExpressionError()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_type_expression(value: TypeExpression) -> Json:
    """Return one JSON value for one TypeExpression."""
    if value.kind == "scalar":
        return {
            "kind": "scalar",
            "scalar": to_json_type_literal(value.scalar),
        }
    elif value.kind == "this":
        return {
            "kind": "this",
        }
    elif value.kind == "path":
        return {
            "kind": "path",
            "path": destack._generated.js.tree.path.to_json_path(value.path),
            "genericArguments": [
                destack._generated.js.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.generic_arguments
            ],
        }
    elif value.kind == "readonly":
        return {
            "kind": "readonly",
            "targetType": destack._generated.js.tree.node.to_json_local_node_id(
                value.target_type
            ),
        }
    elif value.kind == "keyOf":
        return {
            "kind": "keyOf",
            "targetType": destack._generated.js.tree.node.to_json_local_node_id(
                value.target_type
            ),
        }
    elif value.kind == "must":
        return {
            "kind": "must",
            "targetType": destack._generated.js.tree.node.to_json_local_node_id(
                value.target_type
            ),
        }
    elif value.kind == "not":
        return {
            "kind": "not",
            "targetType": destack._generated.js.tree.node.to_json_local_node_id(
                value.target_type
            ),
        }
    elif value.kind == "extends":
        return {
            "kind": "extends",
            "left": destack._generated.js.tree.node.to_json_local_node_id(value.left),
            "right": destack._generated.js.tree.node.to_json_local_node_id(value.right),
        }
    elif value.kind == "implements":
        return {
            "kind": "implements",
            "left": destack._generated.js.tree.node.to_json_local_node_id(value.left),
            "right": destack._generated.js.tree.node.to_json_local_node_id(value.right),
        }
    elif value.kind == "conditional":
        return {
            "kind": "conditional",
            "left": destack._generated.js.tree.node.to_json_local_node_id(value.left),
            "right": destack._generated.js.tree.node.to_json_local_node_id(value.right),
            "thenType": destack._generated.js.tree.node.to_json_local_node_id(
                value.then_type
            ),
            "elseType": destack._generated.js.tree.node.to_json_local_node_id(
                value.else_type
            ),
        }
    elif value.kind == "mapped":
        return {
            "kind": "mapped",
            "parameter": to_json_type_mapped_parameter(value.parameter),
            "modifiers": to_json_type_mapped_modifiers(value.modifiers),
            **(
                {}
                if value.value is None
                else {
                    "value": destack._generated.js.tree.node.to_json_local_node_id(
                        value.value
                    )
                }
            ),
        }
    elif value.kind == "index":
        return {
            "kind": "index",
            "left": destack._generated.js.tree.node.to_json_local_node_id(value.left),
            "index": destack._generated.js.tree.node.to_json_local_node_id(value.index),
        }
    elif value.kind == "templateLiteral":
        return {
            "kind": "templateLiteral",
            "template_literal": to_json_type_template_literal(value.template_literal),
        }
    elif value.kind == "import":
        return {
            "kind": "import",
            "target": destack._generated.core.string.to_json_string_id(value.target),
            **(
                {}
                if value.qualifier is None
                else {
                    "qualifier": destack._generated.js.tree.path.to_json_path(
                        value.qualifier
                    )
                }
            ),
            "genericArguments": [
                destack._generated.js.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.generic_arguments
            ],
        }
    elif value.kind == "infer":
        return {
            "kind": "infer",
            "name": destack._generated.core.string.to_json_string_id(value.name),
            **(
                {}
                if value.constraint is None
                else {
                    "constraint": destack._generated.js.tree.node.to_json_local_node_id(
                        value.constraint
                    )
                }
            ),
        }
    elif value.kind == "array":
        return {
            "kind": "array",
            "element": destack._generated.js.tree.node.to_json_local_node_id(
                value.element
            ),
        }
    elif value.kind == "tuple":
        return {
            "kind": "tuple",
            "elements": [
                destack._generated.js.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.elements
            ],
        }
    elif value.kind == "object":
        return {
            "kind": "object",
            "members": [
                destack._generated.js.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.members
            ],
        }
    elif value.kind == "union":
        return {
            "kind": "union",
            "elements": [
                destack._generated.js.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.elements
            ],
        }
    elif value.kind == "intersection":
        return {
            "kind": "intersection",
            "elements": [
                destack._generated.js.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.elements
            ],
        }
    elif value.kind == "functionTypeDeclaration":
        return {
            "kind": "functionTypeDeclaration",
            "function_type_declaration": to_json_function_type_declaration(
                value.function_type_declaration
            ),
        }
    elif value.kind == "constructorTypeDeclaration":
        return {
            "kind": "constructorTypeDeclaration",
            "constructor_type_declaration": to_json_constructor_type_declaration(
                value.constructor_type_declaration
            ),
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

    if kind == "scalar":
        return TypeExpressionScalar(
            scalar=from_json_type_literal(json_field(object_, "scalar"))
        )
    elif kind == "this":
        return TypeExpressionThis()
    elif kind == "path":
        return TypeExpressionPath(
            path=destack._generated.js.tree.path.from_json_path(
                json_field(object_, "path")
            ),
            generic_arguments=[
                destack._generated.js.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "genericArguments"))
            ],
        )
    elif kind == "readonly":
        return TypeExpressionReadonly(
            target_type=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "targetType")
            ),
        )
    elif kind == "keyOf":
        return TypeExpressionKeyOf(
            target_type=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "targetType")
            ),
        )
    elif kind == "must":
        return TypeExpressionMust(
            target_type=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "targetType")
            ),
        )
    elif kind == "not":
        return TypeExpressionNot(
            target_type=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "targetType")
            ),
        )
    elif kind == "extends":
        return TypeExpressionExtends(
            left=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "left")
            ),
            right=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "right")
            ),
        )
    elif kind == "implements":
        return TypeExpressionImplements(
            left=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "left")
            ),
            right=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "right")
            ),
        )
    elif kind == "conditional":
        return TypeExpressionConditional(
            left=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "left")
            ),
            right=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "right")
            ),
            then_type=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "thenType")
            ),
            else_type=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "elseType")
            ),
        )
    elif kind == "mapped":
        return TypeExpressionMapped(
            parameter=from_json_type_mapped_parameter(json_field(object_, "parameter")),
            modifiers=from_json_type_mapped_modifiers(json_field(object_, "modifiers")),
            value=json_optional(
                object_,
                "value",
                lambda value: destack._generated.js.tree.node.from_json_local_node_id(
                    value
                ),
            ),
        )
    elif kind == "index":
        return TypeExpressionIndex(
            left=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "left")
            ),
            index=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "index")
            ),
        )
    elif kind == "templateLiteral":
        return TypeExpressionTemplateLiteral(
            template_literal=from_json_type_template_literal(
                json_field(object_, "template_literal")
            )
        )
    elif kind == "import":
        return TypeExpressionImport(
            target=destack._generated.core.string.from_json_string_id(
                json_field(object_, "target")
            ),
            qualifier=json_optional(
                object_,
                "qualifier",
                lambda value: destack._generated.js.tree.path.from_json_path(value),
            ),
            generic_arguments=[
                destack._generated.js.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "genericArguments"))
            ],
        )
    elif kind == "infer":
        return TypeExpressionInfer(
            name=destack._generated.core.string.from_json_string_id(
                json_field(object_, "name")
            ),
            constraint=json_optional(
                object_,
                "constraint",
                lambda value: destack._generated.js.tree.node.from_json_local_node_id(
                    value
                ),
            ),
        )
    elif kind == "array":
        return TypeExpressionArray(
            element=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "element")
            ),
        )
    elif kind == "tuple":
        return TypeExpressionTuple(
            elements=[
                destack._generated.js.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "elements"))
            ],
        )
    elif kind == "object":
        return TypeExpressionObject(
            members=[
                destack._generated.js.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "members"))
            ],
        )
    elif kind == "union":
        return TypeExpressionUnion(
            elements=[
                destack._generated.js.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "elements"))
            ],
        )
    elif kind == "intersection":
        return TypeExpressionIntersection(
            elements=[
                destack._generated.js.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "elements"))
            ],
        )
    elif kind == "functionTypeDeclaration":
        return TypeExpressionFunctionTypeDeclaration(
            function_type_declaration=from_json_function_type_declaration(
                json_field(object_, "function_type_declaration")
            )
        )
    elif kind == "constructorTypeDeclaration":
        return TypeExpressionConstructorTypeDeclaration(
            constructor_type_declaration=from_json_constructor_type_declaration(
                json_field(object_, "constructor_type_declaration")
            )
        )
    elif kind == "error":
        return TypeExpressionError()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class TypeLiteralNever:
    """Never type `never`."""

    kind: typing.Literal["never"] = "never"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_literal(self)


@dataclass(frozen=True, slots=True)
class TypeLiteralAny:
    """Any type `any`."""

    kind: typing.Literal["any"] = "any"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_literal(self)


@dataclass(frozen=True, slots=True)
class TypeLiteralUndefined:
    """Undefined type and value."""

    kind: typing.Literal["undefined"] = "undefined"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_literal(self)


@dataclass(frozen=True, slots=True)
class TypeLiteralUnknown:
    """Unknown type."""

    kind: typing.Literal["unknown"] = "unknown"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_literal(self)


@dataclass(frozen=True, slots=True)
class TypeLiteralObject:
    """Object type (any non-primitive)."""

    kind: typing.Literal["object"] = "object"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_literal(self)


@dataclass(frozen=True, slots=True)
class TypeLiteralVoid:
    """Void type."""

    kind: typing.Literal["void"] = "void"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_literal(self)


@dataclass(frozen=True, slots=True)
class TypeLiteralNull:
    """Null type and value."""

    kind: typing.Literal["null"] = "null"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_literal(self)


@dataclass(frozen=True, slots=True)
class TypeLiteralPrimitive:
    """Primitive type."""

    primitive: PrimitiveType
    kind: typing.Literal["primitive"] = "primitive"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_literal(self)


@dataclass(frozen=True, slots=True)
class TypeLiteralScalarLiteral:
    """Scalar literal."""

    scalar_literal: destack._generated.js.tree.literal.ScalarLiteral
    kind: typing.Literal["scalarLiteral"] = "scalarLiteral"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_literal(self)


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


def encode_type_literal(writer: BinaryWriter, value: TypeLiteral) -> None:
    """Encode one TypeLiteral."""
    if value.kind == "never":
        writer.write_unsigned(0)
    elif value.kind == "any":
        writer.write_unsigned(1)
    elif value.kind == "undefined":
        writer.write_unsigned(2)
    elif value.kind == "unknown":
        writer.write_unsigned(3)
    elif value.kind == "object":
        writer.write_unsigned(4)
    elif value.kind == "void":
        writer.write_unsigned(5)
    elif value.kind == "null":
        writer.write_unsigned(6)
    elif value.kind == "primitive":
        writer.write_unsigned(7)
        encode_primitive_type(writer, value.primitive)
    elif value.kind == "scalarLiteral":
        writer.write_unsigned(8)
        destack._generated.js.tree.literal.encode_scalar_literal(
            writer, value.scalar_literal
        )
    else:
        raise SerdeError("unknown enum variant")


def decode_type_literal(reader: BinaryReader) -> TypeLiteral:
    """Decode one TypeLiteral."""
    variant = reader.read_number()

    if variant == 0:
        return TypeLiteralNever()
    elif variant == 1:
        return TypeLiteralAny()
    elif variant == 2:
        return TypeLiteralUndefined()
    elif variant == 3:
        return TypeLiteralUnknown()
    elif variant == 4:
        return TypeLiteralObject()
    elif variant == 5:
        return TypeLiteralVoid()
    elif variant == 6:
        return TypeLiteralNull()
    elif variant == 7:
        primitive = decode_primitive_type(reader)

        return TypeLiteralPrimitive(primitive=primitive)
    elif variant == 8:
        scalar_literal = destack._generated.js.tree.literal.decode_scalar_literal(
            reader
        )

        return TypeLiteralScalarLiteral(scalar_literal=scalar_literal)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_type_literal(value: TypeLiteral) -> Json:
    """Return one JSON value for one TypeLiteral."""
    if value.kind == "never":
        return {
            "kind": "never",
        }
    elif value.kind == "any":
        return {
            "kind": "any",
        }
    elif value.kind == "undefined":
        return {
            "kind": "undefined",
        }
    elif value.kind == "unknown":
        return {
            "kind": "unknown",
        }
    elif value.kind == "object":
        return {
            "kind": "object",
        }
    elif value.kind == "void":
        return {
            "kind": "void",
        }
    elif value.kind == "null":
        return {
            "kind": "null",
        }
    elif value.kind == "primitive":
        return {
            "kind": "primitive",
            "primitive": to_json_primitive_type(value.primitive),
        }
    elif value.kind == "scalarLiteral":
        return {
            "kind": "scalarLiteral",
            "scalar_literal": destack._generated.js.tree.literal.to_json_scalar_literal(
                value.scalar_literal
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_type_literal(value: Json) -> TypeLiteral:
    """Return one TypeLiteral from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "never":
        return TypeLiteralNever()
    elif kind == "any":
        return TypeLiteralAny()
    elif kind == "undefined":
        return TypeLiteralUndefined()
    elif kind == "unknown":
        return TypeLiteralUnknown()
    elif kind == "object":
        return TypeLiteralObject()
    elif kind == "void":
        return TypeLiteralVoid()
    elif kind == "null":
        return TypeLiteralNull()
    elif kind == "primitive":
        return TypeLiteralPrimitive(
            primitive=from_json_primitive_type(json_field(object_, "primitive"))
        )
    elif kind == "scalarLiteral":
        return TypeLiteralScalarLiteral(
            scalar_literal=destack._generated.js.tree.literal.from_json_scalar_literal(
                json_field(object_, "scalar_literal")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


"""A PrimitiveType is a primitive type node."""
PrimitiveType: typing.TypeAlias = (
    typing.Literal["boolean"]
    | typing.Literal["string"]
    | typing.Literal["bigint"]
    | typing.Literal["number"]
    | typing.Literal["symbol"]
    | typing.Literal["uniqueSymbol"]
)


def encode_primitive_type(writer: BinaryWriter, value: PrimitiveType) -> None:
    """Encode one PrimitiveType."""
    if value == "boolean":
        writer.write_unsigned(0)
    elif value == "string":
        writer.write_unsigned(1)
    elif value == "bigint":
        writer.write_unsigned(2)
    elif value == "number":
        writer.write_unsigned(3)
    elif value == "symbol":
        writer.write_unsigned(4)
    elif value == "uniqueSymbol":
        writer.write_unsigned(5)
    else:
        raise SerdeError("unknown enum variant")


def decode_primitive_type(reader: BinaryReader) -> PrimitiveType:
    """Decode one PrimitiveType."""
    variant = reader.read_number()

    if variant == 0:
        return "boolean"
    elif variant == 1:
        return "string"
    elif variant == 2:
        return "bigint"
    elif variant == 3:
        return "number"
    elif variant == 4:
        return "symbol"
    elif variant == 5:
        return "uniqueSymbol"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_primitive_type(value: PrimitiveType) -> Json:
    """Return one JSON value for one PrimitiveType."""
    return value


def from_json_primitive_type(value: Json) -> PrimitiveType:
    """Return one PrimitiveType from one JSON value."""
    variant = json_string(value)

    if variant == "boolean":
        return "boolean"
    elif variant == "string":
        return "string"
    elif variant == "bigint":
        return "bigint"
    elif variant == "number":
        return "number"
    elif variant == "symbol":
        return "symbol"
    elif variant == "uniqueSymbol":
        return "uniqueSymbol"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class TypeMappedParameter:
    """One mapped type parameter."""

    # the parameter name
    name: destack._generated.core.string.StringId
    # the source type iterated by `in`
    source_type: destack._generated.js.tree.node.LocalNodeId
    # the optional key remap
    key_remap: destack._generated.js.tree.node.LocalNodeId | None

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
    destack._generated.js.tree.node.encode_local_node_id(writer, value.source_type)
    if value.key_remap is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.key_remap)


def decode_type_mapped_parameter(reader: BinaryReader) -> TypeMappedParameter:
    """Decode one TypeMappedParameter."""
    name = destack._generated.core.string.decode_string_id(reader)
    source_type = destack._generated.js.tree.node.decode_local_node_id(reader)
    key_remap = reader.read_option(
        lambda: destack._generated.js.tree.node.decode_local_node_id(reader)
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
        "sourceType": destack._generated.js.tree.node.to_json_local_node_id(
            value.source_type
        ),
        **(
            {}
            if value.key_remap is None
            else {
                "keyRemap": destack._generated.js.tree.node.to_json_local_node_id(
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
        source_type=destack._generated.js.tree.node.from_json_local_node_id(
            json_field(object_, "sourceType")
        ),
        key_remap=json_optional(
            object_,
            "keyRemap",
            lambda value: destack._generated.js.tree.node.from_json_local_node_id(
                value
            ),
        ),
    )


@dataclass(frozen=True, slots=True)
class TypeMappedModifiers:
    """One mapped type modifier set."""

    # the readonly modifier
    readonly: MappedTypeModifier
    # the optional modifier
    optional: MappedTypeModifier

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_mapped_modifiers(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeMappedModifiers:
        """Decode one TypeMappedModifiers."""
        return decode_type_mapped_modifiers(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_mapped_modifiers(self)

    @classmethod
    def from_json(cls, value: Json) -> TypeMappedModifiers:
        """Return one TypeMappedModifiers from one JSON value."""
        return from_json_type_mapped_modifiers(value)


def encode_type_mapped_modifiers(
    writer: BinaryWriter, value: TypeMappedModifiers
) -> None:
    """Encode one TypeMappedModifiers."""
    encode_mapped_type_modifier(writer, value.readonly)
    encode_mapped_type_modifier(writer, value.optional)


def decode_type_mapped_modifiers(reader: BinaryReader) -> TypeMappedModifiers:
    """Decode one TypeMappedModifiers."""
    readonly = decode_mapped_type_modifier(reader)
    optional = decode_mapped_type_modifier(reader)

    return TypeMappedModifiers(
        readonly=readonly,
        optional=optional,
    )


def to_json_type_mapped_modifiers(value: TypeMappedModifiers) -> Json:
    """Return one JSON value for one TypeMappedModifiers."""
    return {
        "readonly": to_json_mapped_type_modifier(value.readonly),
        "optional": to_json_mapped_type_modifier(value.optional),
    }


def from_json_type_mapped_modifiers(value: Json) -> TypeMappedModifiers:
    """Return one TypeMappedModifiers from one JSON value."""
    object_ = json_object(value)

    return TypeMappedModifiers(
        readonly=from_json_mapped_type_modifier(json_field(object_, "readonly")),
        optional=from_json_mapped_type_modifier(json_field(object_, "optional")),
    )


"""One mapped type modifier."""
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


@dataclass(frozen=True, slots=True)
class TypeTemplateLiteral:
    """One type template literal."""

    # the raw template strings
    strings: Sequence[destack._generated.core.string.StringId]
    # the interpolated type spans
    spans: Sequence[destack._generated.js.tree.node.LocalNodeId]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_template_literal(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeTemplateLiteral:
        """Decode one TypeTemplateLiteral."""
        return decode_type_template_literal(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_template_literal(self)

    @classmethod
    def from_json(cls, value: Json) -> TypeTemplateLiteral:
        """Return one TypeTemplateLiteral from one JSON value."""
        return from_json_type_template_literal(value)


def encode_type_template_literal(
    writer: BinaryWriter, value: TypeTemplateLiteral
) -> None:
    """Encode one TypeTemplateLiteral."""
    writer.write_unsigned(len(value.strings))
    for item_value_strings_0 in value.strings:
        destack._generated.core.string.encode_string_id(writer, item_value_strings_0)
    writer.write_unsigned(len(value.spans))
    for item_value_spans_0 in value.spans:
        destack._generated.js.tree.node.encode_local_node_id(writer, item_value_spans_0)


def decode_type_template_literal(reader: BinaryReader) -> TypeTemplateLiteral:
    """Decode one TypeTemplateLiteral."""
    strings = [
        destack._generated.core.string.decode_string_id(reader)
        for _ in range(reader.read_number())
    ]
    spans = [
        destack._generated.js.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]

    return TypeTemplateLiteral(
        strings=strings,
        spans=spans,
    )


def to_json_type_template_literal(value: TypeTemplateLiteral) -> Json:
    """Return one JSON value for one TypeTemplateLiteral."""
    return {
        "strings": [
            destack._generated.core.string.to_json_string_id(item_0)
            for item_0 in value.strings
        ],
        "spans": [
            destack._generated.js.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.spans
        ],
    }


def from_json_type_template_literal(value: Json) -> TypeTemplateLiteral:
    """Return one TypeTemplateLiteral from one JSON value."""
    object_ = json_object(value)

    return TypeTemplateLiteral(
        strings=[
            destack._generated.core.string.from_json_string_id(item_0)
            for item_0 in json_array(json_field(object_, "strings"))
        ],
        spans=[
            destack._generated.js.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "spans"))
        ],
    )


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_function_type_declaration(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionTypeDeclaration:
        """Decode one FunctionTypeDeclaration."""
        return decode_function_type_declaration(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_function_type_declaration(self)

    @classmethod
    def from_json(cls, value: Json) -> FunctionTypeDeclaration:
        """Return one FunctionTypeDeclaration from one JSON value."""
        return from_json_function_type_declaration(value)


def encode_function_type_declaration(
    writer: BinaryWriter, value: FunctionTypeDeclaration
) -> None:
    """Encode one FunctionTypeDeclaration."""
    writer.write_unsigned(len(value.generic_parameters))
    for item_value_generic_parameters_0 in value.generic_parameters:
        destack._generated.js.tree.node.encode_local_node_id(
            writer, item_value_generic_parameters_0
        )
    if value.this_parameter is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.js.tree.node.encode_local_node_id(
            writer, value.this_parameter
        )
    writer.write_unsigned(len(value.parameters))
    for item_value_parameters_0 in value.parameters:
        destack._generated.js.tree.node.encode_local_node_id(
            writer, item_value_parameters_0
        )
    if value.return_type is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.return_type)


def decode_function_type_declaration(reader: BinaryReader) -> FunctionTypeDeclaration:
    """Decode one FunctionTypeDeclaration."""
    generic_parameters = [
        destack._generated.js.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    this_parameter = reader.read_option(
        lambda: destack._generated.js.tree.node.decode_local_node_id(reader)
    )
    parameters = [
        destack._generated.js.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    return_type = reader.read_option(
        lambda: destack._generated.js.tree.node.decode_local_node_id(reader)
    )

    return FunctionTypeDeclaration(
        generic_parameters=generic_parameters,
        this_parameter=this_parameter,
        parameters=parameters,
        return_type=return_type,
    )


def to_json_function_type_declaration(value: FunctionTypeDeclaration) -> Json:
    """Return one JSON value for one FunctionTypeDeclaration."""
    return {
        "genericParameters": [
            destack._generated.js.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.generic_parameters
        ],
        **(
            {}
            if value.this_parameter is None
            else {
                "thisParameter": destack._generated.js.tree.node.to_json_local_node_id(
                    value.this_parameter
                )
            }
        ),
        "parameters": [
            destack._generated.js.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.parameters
        ],
        **(
            {}
            if value.return_type is None
            else {
                "returnType": destack._generated.js.tree.node.to_json_local_node_id(
                    value.return_type
                )
            }
        ),
    }


def from_json_function_type_declaration(value: Json) -> FunctionTypeDeclaration:
    """Return one FunctionTypeDeclaration from one JSON value."""
    object_ = json_object(value)

    return FunctionTypeDeclaration(
        generic_parameters=[
            destack._generated.js.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "genericParameters"))
        ],
        this_parameter=json_optional(
            object_,
            "thisParameter",
            lambda value: destack._generated.js.tree.node.from_json_local_node_id(
                value
            ),
        ),
        parameters=[
            destack._generated.js.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "parameters"))
        ],
        return_type=json_optional(
            object_,
            "returnType",
            lambda value: destack._generated.js.tree.node.from_json_local_node_id(
                value
            ),
        ),
    )


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_constructor_type_declaration(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ConstructorTypeDeclaration:
        """Decode one ConstructorTypeDeclaration."""
        return decode_constructor_type_declaration(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_constructor_type_declaration(self)

    @classmethod
    def from_json(cls, value: Json) -> ConstructorTypeDeclaration:
        """Return one ConstructorTypeDeclaration from one JSON value."""
        return from_json_constructor_type_declaration(value)


def encode_constructor_type_declaration(
    writer: BinaryWriter, value: ConstructorTypeDeclaration
) -> None:
    """Encode one ConstructorTypeDeclaration."""
    writer.write_bool(value.is_abstract)
    writer.write_unsigned(len(value.generic_parameters))
    for item_value_generic_parameters_0 in value.generic_parameters:
        destack._generated.js.tree.node.encode_local_node_id(
            writer, item_value_generic_parameters_0
        )
    writer.write_unsigned(len(value.parameters))
    for item_value_parameters_0 in value.parameters:
        destack._generated.js.tree.node.encode_local_node_id(
            writer, item_value_parameters_0
        )
    if value.return_type is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.return_type)


def decode_constructor_type_declaration(
    reader: BinaryReader,
) -> ConstructorTypeDeclaration:
    """Decode one ConstructorTypeDeclaration."""
    is_abstract = reader.read_bool()
    generic_parameters = [
        destack._generated.js.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    parameters = [
        destack._generated.js.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    return_type = reader.read_option(
        lambda: destack._generated.js.tree.node.decode_local_node_id(reader)
    )

    return ConstructorTypeDeclaration(
        is_abstract=is_abstract,
        generic_parameters=generic_parameters,
        parameters=parameters,
        return_type=return_type,
    )


def to_json_constructor_type_declaration(value: ConstructorTypeDeclaration) -> Json:
    """Return one JSON value for one ConstructorTypeDeclaration."""
    return {
        "isAbstract": value.is_abstract,
        "genericParameters": [
            destack._generated.js.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.generic_parameters
        ],
        "parameters": [
            destack._generated.js.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.parameters
        ],
        **(
            {}
            if value.return_type is None
            else {
                "returnType": destack._generated.js.tree.node.to_json_local_node_id(
                    value.return_type
                )
            }
        ),
    }


def from_json_constructor_type_declaration(value: Json) -> ConstructorTypeDeclaration:
    """Return one ConstructorTypeDeclaration from one JSON value."""
    object_ = json_object(value)

    return ConstructorTypeDeclaration(
        is_abstract=json_bool(json_field(object_, "isAbstract")),
        generic_parameters=[
            destack._generated.js.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "genericParameters"))
        ],
        parameters=[
            destack._generated.js.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "parameters"))
        ],
        return_type=json_optional(
            object_,
            "returnType",
            lambda value: destack._generated.js.tree.node.from_json_local_node_id(
                value
            ),
        ),
    )


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tuple_element(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TupleElement:
        """Decode one TupleElement."""
        return decode_tuple_element(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tuple_element(self)

    @classmethod
    def from_json(cls, value: Json) -> TupleElement:
        """Return one TupleElement from one JSON value."""
        return from_json_tuple_element(value)


def encode_tuple_element(writer: BinaryWriter, value: TupleElement) -> None:
    """Encode one TupleElement."""
    if value.label is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.core.string.encode_string_id(writer, value.label)
    destack._generated.js.tree.node.encode_local_node_id(writer, value.ty)
    writer.write_bool(value.is_optional)
    writer.write_bool(value.is_readonly)
    writer.write_bool(value.is_rest)


def decode_tuple_element(reader: BinaryReader) -> TupleElement:
    """Decode one TupleElement."""
    label = reader.read_option(
        lambda: destack._generated.core.string.decode_string_id(reader)
    )
    ty = destack._generated.js.tree.node.decode_local_node_id(reader)
    is_optional = reader.read_bool()
    is_readonly = reader.read_bool()
    is_rest = reader.read_bool()

    return TupleElement(
        label=label,
        ty=ty,
        is_optional=is_optional,
        is_readonly=is_readonly,
        is_rest=is_rest,
    )


def to_json_tuple_element(value: TupleElement) -> Json:
    """Return one JSON value for one TupleElement."""
    return {
        **(
            {}
            if value.label is None
            else {
                "label": destack._generated.core.string.to_json_string_id(value.label)
            }
        ),
        "ty": destack._generated.js.tree.node.to_json_local_node_id(value.ty),
        "isOptional": value.is_optional,
        "isReadonly": value.is_readonly,
        "isRest": value.is_rest,
    }


def from_json_tuple_element(value: Json) -> TupleElement:
    """Return one TupleElement from one JSON value."""
    object_ = json_object(value)

    return TupleElement(
        label=json_optional(
            object_,
            "label",
            lambda value: destack._generated.core.string.from_json_string_id(value),
        ),
        ty=destack._generated.js.tree.node.from_json_local_node_id(
            json_field(object_, "ty")
        ),
        is_optional=json_bool(json_field(object_, "isOptional")),
        is_readonly=json_bool(json_field(object_, "isReadonly")),
        is_rest=json_bool(json_field(object_, "isRest")),
    )


@dataclass(frozen=True, slots=True)
class TypeMemberField:
    """Named field (like `a: T`)."""

    modifiers: destack._generated.js.tree.argument.BindingModifier | None
    key: destack._generated.js.tree.key.Key
    ty: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["field"] = "field"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_member(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_member(self)


@dataclass(frozen=True, slots=True)
class TypeMemberMethod:
    """Named method (like `foo(): T`)."""

    modifiers: destack._generated.js.tree.argument.BindingModifier | None
    key: destack._generated.js.tree.key.Key
    signature: destack._generated.js.tree.function.FunctionSignature
    kind: typing.Literal["method"] = "method"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_member(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_member(self)


@dataclass(frozen=True, slots=True)
class TypeMemberCallSignature:
    """Call signature (like `<T>(value: T): U`)."""

    modifiers: destack._generated.js.tree.argument.BindingModifier | None
    signature: FunctionTypeDeclaration
    kind: typing.Literal["callSignature"] = "callSignature"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_member(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_member(self)


@dataclass(frozen=True, slots=True)
class TypeMemberConstructSignature:
    """Construct signature (like `new <T>(value: T): U`)."""

    modifiers: destack._generated.js.tree.argument.BindingModifier | None
    signature: ConstructorTypeDeclaration
    kind: typing.Literal["constructSignature"] = "constructSignature"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_member(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_member(self)


@dataclass(frozen=True, slots=True)
class TypeMemberIndexSignature:
    """Index signature (like `[key: string]: T`)."""

    modifiers: destack._generated.js.tree.argument.BindingModifier | None
    name: destack._generated.core.string.StringId
    key_type: destack._generated.js.tree.node.LocalNodeId
    value_type: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["indexSignature"] = "indexSignature"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_member(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_member(self)


"""The type of an attribute (like a property or field)."""
TypeMember: typing.TypeAlias = (
    TypeMemberField
    | TypeMemberMethod
    | TypeMemberCallSignature
    | TypeMemberConstructSignature
    | TypeMemberIndexSignature
)


def encode_type_member(writer: BinaryWriter, value: TypeMember) -> None:
    """Encode one TypeMember."""
    if value.kind == "field":
        writer.write_unsigned(0)
        if value.modifiers is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.argument.encode_binding_modifier(
                writer, value.modifiers
            )
        destack._generated.js.tree.key.encode_key(writer, value.key)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.ty)
    elif value.kind == "method":
        writer.write_unsigned(1)
        if value.modifiers is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.argument.encode_binding_modifier(
                writer, value.modifiers
            )
        destack._generated.js.tree.key.encode_key(writer, value.key)
        destack._generated.js.tree.function.encode_function_signature(
            writer, value.signature
        )
    elif value.kind == "callSignature":
        writer.write_unsigned(2)
        if value.modifiers is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.argument.encode_binding_modifier(
                writer, value.modifiers
            )
        encode_function_type_declaration(writer, value.signature)
    elif value.kind == "constructSignature":
        writer.write_unsigned(3)
        if value.modifiers is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.argument.encode_binding_modifier(
                writer, value.modifiers
            )
        encode_constructor_type_declaration(writer, value.signature)
    elif value.kind == "indexSignature":
        writer.write_unsigned(4)
        if value.modifiers is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.js.tree.argument.encode_binding_modifier(
                writer, value.modifiers
            )
        destack._generated.core.string.encode_string_id(writer, value.name)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.key_type)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.value_type)
    else:
        raise SerdeError("unknown enum variant")


def decode_type_member(reader: BinaryReader) -> TypeMember:
    """Decode one TypeMember."""
    variant = reader.read_number()

    if variant == 0:
        modifiers = reader.read_option(
            lambda: destack._generated.js.tree.argument.decode_binding_modifier(reader)
        )
        key = destack._generated.js.tree.key.decode_key(reader)
        ty = destack._generated.js.tree.node.decode_local_node_id(reader)

        return TypeMemberField(
            modifiers=modifiers,
            key=key,
            ty=ty,
        )
    elif variant == 1:
        modifiers = reader.read_option(
            lambda: destack._generated.js.tree.argument.decode_binding_modifier(reader)
        )
        key = destack._generated.js.tree.key.decode_key(reader)
        signature = destack._generated.js.tree.function.decode_function_signature(
            reader
        )

        return TypeMemberMethod(
            modifiers=modifiers,
            key=key,
            signature=signature,
        )
    elif variant == 2:
        modifiers = reader.read_option(
            lambda: destack._generated.js.tree.argument.decode_binding_modifier(reader)
        )
        signature = decode_function_type_declaration(reader)

        return TypeMemberCallSignature(
            modifiers=modifiers,
            signature=signature,
        )
    elif variant == 3:
        modifiers = reader.read_option(
            lambda: destack._generated.js.tree.argument.decode_binding_modifier(reader)
        )
        signature = decode_constructor_type_declaration(reader)

        return TypeMemberConstructSignature(
            modifiers=modifiers,
            signature=signature,
        )
    elif variant == 4:
        modifiers = reader.read_option(
            lambda: destack._generated.js.tree.argument.decode_binding_modifier(reader)
        )
        name = destack._generated.core.string.decode_string_id(reader)
        key_type = destack._generated.js.tree.node.decode_local_node_id(reader)
        value_type = destack._generated.js.tree.node.decode_local_node_id(reader)

        return TypeMemberIndexSignature(
            modifiers=modifiers,
            name=name,
            key_type=key_type,
            value_type=value_type,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_type_member(value: TypeMember) -> Json:
    """Return one JSON value for one TypeMember."""
    if value.kind == "field":
        return {
            "kind": "field",
            **(
                {}
                if value.modifiers is None
                else {
                    "modifiers": destack._generated.js.tree.argument.to_json_binding_modifier(
                        value.modifiers
                    )
                }
            ),
            "key": destack._generated.js.tree.key.to_json_key(value.key),
            "ty": destack._generated.js.tree.node.to_json_local_node_id(value.ty),
        }
    elif value.kind == "method":
        return {
            "kind": "method",
            **(
                {}
                if value.modifiers is None
                else {
                    "modifiers": destack._generated.js.tree.argument.to_json_binding_modifier(
                        value.modifiers
                    )
                }
            ),
            "key": destack._generated.js.tree.key.to_json_key(value.key),
            "signature": destack._generated.js.tree.function.to_json_function_signature(
                value.signature
            ),
        }
    elif value.kind == "callSignature":
        return {
            "kind": "callSignature",
            **(
                {}
                if value.modifiers is None
                else {
                    "modifiers": destack._generated.js.tree.argument.to_json_binding_modifier(
                        value.modifiers
                    )
                }
            ),
            "signature": to_json_function_type_declaration(value.signature),
        }
    elif value.kind == "constructSignature":
        return {
            "kind": "constructSignature",
            **(
                {}
                if value.modifiers is None
                else {
                    "modifiers": destack._generated.js.tree.argument.to_json_binding_modifier(
                        value.modifiers
                    )
                }
            ),
            "signature": to_json_constructor_type_declaration(value.signature),
        }
    elif value.kind == "indexSignature":
        return {
            "kind": "indexSignature",
            **(
                {}
                if value.modifiers is None
                else {
                    "modifiers": destack._generated.js.tree.argument.to_json_binding_modifier(
                        value.modifiers
                    )
                }
            ),
            "name": destack._generated.core.string.to_json_string_id(value.name),
            "keyType": destack._generated.js.tree.node.to_json_local_node_id(
                value.key_type
            ),
            "valueType": destack._generated.js.tree.node.to_json_local_node_id(
                value.value_type
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_type_member(value: Json) -> TypeMember:
    """Return one TypeMember from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "field":
        return TypeMemberField(
            modifiers=json_optional(
                object_,
                "modifiers",
                lambda value: (
                    destack._generated.js.tree.argument.from_json_binding_modifier(
                        value
                    )
                ),
            ),
            key=destack._generated.js.tree.key.from_json_key(
                json_field(object_, "key")
            ),
            ty=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "ty")
            ),
        )
    elif kind == "method":
        return TypeMemberMethod(
            modifiers=json_optional(
                object_,
                "modifiers",
                lambda value: (
                    destack._generated.js.tree.argument.from_json_binding_modifier(
                        value
                    )
                ),
            ),
            key=destack._generated.js.tree.key.from_json_key(
                json_field(object_, "key")
            ),
            signature=destack._generated.js.tree.function.from_json_function_signature(
                json_field(object_, "signature")
            ),
        )
    elif kind == "callSignature":
        return TypeMemberCallSignature(
            modifiers=json_optional(
                object_,
                "modifiers",
                lambda value: (
                    destack._generated.js.tree.argument.from_json_binding_modifier(
                        value
                    )
                ),
            ),
            signature=from_json_function_type_declaration(
                json_field(object_, "signature")
            ),
        )
    elif kind == "constructSignature":
        return TypeMemberConstructSignature(
            modifiers=json_optional(
                object_,
                "modifiers",
                lambda value: (
                    destack._generated.js.tree.argument.from_json_binding_modifier(
                        value
                    )
                ),
            ),
            signature=from_json_constructor_type_declaration(
                json_field(object_, "signature")
            ),
        )
    elif kind == "indexSignature":
        return TypeMemberIndexSignature(
            modifiers=json_optional(
                object_,
                "modifiers",
                lambda value: (
                    destack._generated.js.tree.argument.from_json_binding_modifier(
                        value
                    )
                ),
            ),
            name=destack._generated.core.string.from_json_string_id(
                json_field(object_, "name")
            ),
            key_type=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "keyType")
            ),
            value_type=destack._generated.js.tree.node.from_json_local_node_id(
                json_field(object_, "valueType")
            ),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


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
