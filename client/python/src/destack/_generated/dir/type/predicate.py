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
    json_field,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.dir.symbol.key
import destack._generated.dir.tree.literal
import destack._generated.dir.tree.operator
import destack._generated.dir.type.primitive
import destack._generated.dir.type.projection
import destack._generated.dir.type.type


@dataclass(frozen=True, slots=True)
class Predicate:
    """Executable predicate selected during checking."""

    # the test to execute
    test: PredicateTest
    # the narrowed value type after the predicate succeeds
    narrowed: destack._generated.dir.type.type.GlobalTypeId | None
    # the projected value available after the predicate succeeds
    projection: destack._generated.dir.type.projection.Projection | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_predicate(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Predicate:
        """Decode one Predicate."""
        return decode_predicate(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_predicate(self)

    @classmethod
    def from_json(cls, value: Json) -> Predicate:
        """Return one Predicate from one JSON value."""
        return from_json_predicate(value)


def encode_predicate(writer: BinaryWriter, value: Predicate) -> None:
    """Encode one Predicate."""
    encode_predicate_test(writer, value.test)
    if value.narrowed is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.type.type.encode_global_type_id(writer, value.narrowed)
    if value.projection is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.type.projection.encode_projection(
            writer, value.projection
        )


def decode_predicate(reader: BinaryReader) -> Predicate:
    """Decode one Predicate."""
    test = decode_predicate_test(reader)
    narrowed = reader.read_option(
        lambda: destack._generated.dir.type.type.decode_global_type_id(reader)
    )
    projection = reader.read_option(
        lambda: destack._generated.dir.type.projection.decode_projection(reader)
    )

    return Predicate(
        test=test,
        narrowed=narrowed,
        projection=projection,
    )


def to_json_predicate(value: Predicate) -> Json:
    """Return one JSON value for one Predicate."""
    return {
        "test": to_json_predicate_test(value.test),
        **(
            {}
            if value.narrowed is None
            else {
                "narrowed": destack._generated.dir.type.type.to_json_global_type_id(
                    value.narrowed
                )
            }
        ),
        **(
            {}
            if value.projection is None
            else {
                "projection": destack._generated.dir.type.projection.to_json_projection(
                    value.projection
                )
            }
        ),
    }


def from_json_predicate(value: Json) -> Predicate:
    """Return one Predicate from one JSON value."""
    object_ = json_object(value)

    return Predicate(
        test=from_json_predicate_test(json_field(object_, "test")),
        narrowed=json_optional(
            object_,
            "narrowed",
            lambda value: destack._generated.dir.type.type.from_json_global_type_id(
                value
            ),
        ),
        projection=json_optional(
            object_,
            "projection",
            lambda value: destack._generated.dir.type.projection.from_json_projection(
                value
            ),
        ),
    )


@dataclass(frozen=True, slots=True)
class PredicateTestUnary:
    """Unary test over one input value."""

    unary: PredicateUnaryTest
    kind: typing.Literal["unary"] = "unary"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_predicate_test(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_predicate_test(self)


@dataclass(frozen=True, slots=True)
class PredicateTestMembership:
    """Structural membership test over a receiver and key."""

    membership: PredicateMembershipTest
    kind: typing.Literal["membership"] = "membership"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_predicate_test(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_predicate_test(self)


@dataclass(frozen=True, slots=True)
class PredicateTestAny:
    """Predicate that accepts when any alternative accepts."""

    any: Sequence[Predicate]
    kind: typing.Literal["any"] = "any"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_predicate_test(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_predicate_test(self)


"""Predicate test selected during checking."""
PredicateTest: typing.TypeAlias = (
    PredicateTestUnary | PredicateTestMembership | PredicateTestAny
)


def encode_predicate_test(writer: BinaryWriter, value: PredicateTest) -> None:
    """Encode one PredicateTest."""
    if value.kind == "unary":
        writer.write_unsigned(0)
        encode_predicate_unary_test(writer, value.unary)
    elif value.kind == "membership":
        writer.write_unsigned(1)
        encode_predicate_membership_test(writer, value.membership)
    elif value.kind == "any":
        writer.write_unsigned(2)
        writer.write_unsigned(len(value.any))
        for item_value_any_0 in value.any:
            encode_predicate(writer, item_value_any_0)
    else:
        raise SerdeError("unknown enum variant")


def decode_predicate_test(reader: BinaryReader) -> PredicateTest:
    """Decode one PredicateTest."""
    variant = reader.read_number()

    if variant == 0:
        unary = decode_predicate_unary_test(reader)

        return PredicateTestUnary(unary=unary)
    elif variant == 1:
        membership = decode_predicate_membership_test(reader)

        return PredicateTestMembership(membership=membership)
    elif variant == 2:
        any = [decode_predicate(reader) for _ in range(reader.read_number())]

        return PredicateTestAny(any=any)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_predicate_test(value: PredicateTest) -> Json:
    """Return one JSON value for one PredicateTest."""
    if value.kind == "unary":
        return {
            "kind": "unary",
            "unary": to_json_predicate_unary_test(value.unary),
        }
    elif value.kind == "membership":
        return {
            "kind": "membership",
            "membership": to_json_predicate_membership_test(value.membership),
        }
    elif value.kind == "any":
        return {
            "kind": "any",
            "any": [to_json_predicate(item_0) for item_0 in value.any],
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_predicate_test(value: Json) -> PredicateTest:
    """Return one PredicateTest from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "unary":
        return PredicateTestUnary(
            unary=from_json_predicate_unary_test(json_field(object_, "unary"))
        )
    elif kind == "membership":
        return PredicateTestMembership(
            membership=from_json_predicate_membership_test(
                json_field(object_, "membership")
            )
        )
    elif kind == "any":
        return PredicateTestAny(
            any=[
                from_json_predicate(item_0)
                for item_0 in json_array(json_field(object_, "any"))
            ]
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class PredicateUnaryTest:
    """Unary predicate over one input value."""

    # the value tested by this predicate
    input: PredicateOperand
    # the condition applied to the projected input
    condition: PredicateCondition

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_predicate_unary_test(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PredicateUnaryTest:
        """Decode one PredicateUnaryTest."""
        return decode_predicate_unary_test(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_predicate_unary_test(self)

    @classmethod
    def from_json(cls, value: Json) -> PredicateUnaryTest:
        """Return one PredicateUnaryTest from one JSON value."""
        return from_json_predicate_unary_test(value)


def encode_predicate_unary_test(
    writer: BinaryWriter, value: PredicateUnaryTest
) -> None:
    """Encode one PredicateUnaryTest."""
    encode_predicate_operand(writer, value.input)
    encode_predicate_condition(writer, value.condition)


def decode_predicate_unary_test(reader: BinaryReader) -> PredicateUnaryTest:
    """Decode one PredicateUnaryTest."""
    input = decode_predicate_operand(reader)
    condition = decode_predicate_condition(reader)

    return PredicateUnaryTest(
        input=input,
        condition=condition,
    )


def to_json_predicate_unary_test(value: PredicateUnaryTest) -> Json:
    """Return one JSON value for one PredicateUnaryTest."""
    return {
        "input": to_json_predicate_operand(value.input),
        "condition": to_json_predicate_condition(value.condition),
    }


def from_json_predicate_unary_test(value: Json) -> PredicateUnaryTest:
    """Return one PredicateUnaryTest from one JSON value."""
    object_ = json_object(value)

    return PredicateUnaryTest(
        input=from_json_predicate_operand(json_field(object_, "input")),
        condition=from_json_predicate_condition(json_field(object_, "condition")),
    )


@dataclass(frozen=True, slots=True)
class PredicateOperand:
    """Value tested by one executable predicate."""

    # the operand value type
    ty: destack._generated.dir.type.type.GlobalTypeId
    # the projection used to compute the tested value, when one is needed
    projection: destack._generated.dir.type.projection.Projection | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_predicate_operand(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PredicateOperand:
        """Decode one PredicateOperand."""
        return decode_predicate_operand(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_predicate_operand(self)

    @classmethod
    def from_json(cls, value: Json) -> PredicateOperand:
        """Return one PredicateOperand from one JSON value."""
        return from_json_predicate_operand(value)


def encode_predicate_operand(writer: BinaryWriter, value: PredicateOperand) -> None:
    """Encode one PredicateOperand."""
    destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)
    if value.projection is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.type.projection.encode_projection(
            writer, value.projection
        )


def decode_predicate_operand(reader: BinaryReader) -> PredicateOperand:
    """Decode one PredicateOperand."""
    ty = destack._generated.dir.type.type.decode_global_type_id(reader)
    projection = reader.read_option(
        lambda: destack._generated.dir.type.projection.decode_projection(reader)
    )

    return PredicateOperand(
        ty=ty,
        projection=projection,
    )


def to_json_predicate_operand(value: PredicateOperand) -> Json:
    """Return one JSON value for one PredicateOperand."""
    return {
        "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty),
        **(
            {}
            if value.projection is None
            else {
                "projection": destack._generated.dir.type.projection.to_json_projection(
                    value.projection
                )
            }
        ),
    }


def from_json_predicate_operand(value: Json) -> PredicateOperand:
    """Return one PredicateOperand from one JSON value."""
    object_ = json_object(value)

    return PredicateOperand(
        ty=destack._generated.dir.type.type.from_json_global_type_id(
            json_field(object_, "ty")
        ),
        projection=json_optional(
            object_,
            "projection",
            lambda value: destack._generated.dir.type.projection.from_json_projection(
                value
            ),
        ),
    )


@dataclass(frozen=True, slots=True)
class PredicateConditionAlways:
    """Condition that accepts any projected input."""

    kind: typing.Literal["always"] = "always"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_predicate_condition(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_predicate_condition(self)


@dataclass(frozen=True, slots=True)
class PredicateConditionNever:
    """Condition that rejects every projected input."""

    kind: typing.Literal["never"] = "never"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_predicate_condition(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_predicate_condition(self)


@dataclass(frozen=True, slots=True)
class PredicateConditionLiteral:
    """Scalar literal condition, like `"ready"` or `0`."""

    literal: destack._generated.dir.tree.literal.ScalarLiteral
    kind: typing.Literal["literal"] = "literal"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_predicate_condition(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_predicate_condition(self)


@dataclass(frozen=True, slots=True)
class PredicateConditionRange:
    """Scalar interval condition, like `0..=255`."""

    range: PredicateRange
    kind: typing.Literal["range"] = "range"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_predicate_condition(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_predicate_condition(self)


@dataclass(frozen=True, slots=True)
class PredicateConditionPrimitive:
    """Primitive tag condition, like `string` or `int32`."""

    primitive: destack._generated.dir.type.primitive.PrimitiveType
    kind: typing.Literal["primitive"] = "primitive"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_predicate_condition(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_predicate_condition(self)


@dataclass(frozen=True, slots=True)
class PredicateConditionType:
    """Exact runtime type descriptor condition."""

    type: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["type"] = "type"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_predicate_condition(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_predicate_condition(self)


@dataclass(frozen=True, slots=True)
class PredicateConditionSubtype:
    """Runtime subtype descriptor condition."""

    subtype: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["subtype"] = "subtype"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_predicate_condition(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_predicate_condition(self)


"""Condition applied to one predicate input."""
PredicateCondition: typing.TypeAlias = (
    PredicateConditionAlways
    | PredicateConditionNever
    | PredicateConditionLiteral
    | PredicateConditionRange
    | PredicateConditionPrimitive
    | PredicateConditionType
    | PredicateConditionSubtype
)


def encode_predicate_condition(writer: BinaryWriter, value: PredicateCondition) -> None:
    """Encode one PredicateCondition."""
    if value.kind == "always":
        writer.write_unsigned(0)
    elif value.kind == "never":
        writer.write_unsigned(1)
    elif value.kind == "literal":
        writer.write_unsigned(2)
        destack._generated.dir.tree.literal.encode_scalar_literal(writer, value.literal)
    elif value.kind == "range":
        writer.write_unsigned(3)
        encode_predicate_range(writer, value.range)
    elif value.kind == "primitive":
        writer.write_unsigned(4)
        destack._generated.dir.type.primitive.encode_primitive_type(
            writer, value.primitive
        )
    elif value.kind == "type":
        writer.write_unsigned(5)
        destack._generated.dir.type.type.encode_global_type_id(writer, value.type)
    elif value.kind == "subtype":
        writer.write_unsigned(6)
        destack._generated.dir.type.type.encode_global_type_id(writer, value.subtype)
    else:
        raise SerdeError("unknown enum variant")


def decode_predicate_condition(reader: BinaryReader) -> PredicateCondition:
    """Decode one PredicateCondition."""
    variant = reader.read_number()

    if variant == 0:
        return PredicateConditionAlways()
    elif variant == 1:
        return PredicateConditionNever()
    elif variant == 2:
        literal = destack._generated.dir.tree.literal.decode_scalar_literal(reader)

        return PredicateConditionLiteral(literal=literal)
    elif variant == 3:
        range_ = decode_predicate_range(reader)

        return PredicateConditionRange(range=range_)
    elif variant == 4:
        primitive = destack._generated.dir.type.primitive.decode_primitive_type(reader)

        return PredicateConditionPrimitive(primitive=primitive)
    elif variant == 5:
        type = destack._generated.dir.type.type.decode_global_type_id(reader)

        return PredicateConditionType(type=type)
    elif variant == 6:
        subtype = destack._generated.dir.type.type.decode_global_type_id(reader)

        return PredicateConditionSubtype(subtype=subtype)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_predicate_condition(value: PredicateCondition) -> Json:
    """Return one JSON value for one PredicateCondition."""
    if value.kind == "always":
        return {
            "kind": "always",
        }
    elif value.kind == "never":
        return {
            "kind": "never",
        }
    elif value.kind == "literal":
        return {
            "kind": "literal",
            "literal": destack._generated.dir.tree.literal.to_json_scalar_literal(
                value.literal
            ),
        }
    elif value.kind == "range":
        return {
            "kind": "range",
            "range": to_json_predicate_range(value.range),
        }
    elif value.kind == "primitive":
        return {
            "kind": "primitive",
            "primitive": destack._generated.dir.type.primitive.to_json_primitive_type(
                value.primitive
            ),
        }
    elif value.kind == "type":
        return {
            "kind": "type",
            "type": destack._generated.dir.type.type.to_json_global_type_id(value.type),
        }
    elif value.kind == "subtype":
        return {
            "kind": "subtype",
            "subtype": destack._generated.dir.type.type.to_json_global_type_id(
                value.subtype
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_predicate_condition(value: Json) -> PredicateCondition:
    """Return one PredicateCondition from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "always":
        return PredicateConditionAlways()
    elif kind == "never":
        return PredicateConditionNever()
    elif kind == "literal":
        return PredicateConditionLiteral(
            literal=destack._generated.dir.tree.literal.from_json_scalar_literal(
                json_field(object_, "literal")
            )
        )
    elif kind == "range":
        return PredicateConditionRange(
            range=from_json_predicate_range(json_field(object_, "range"))
        )
    elif kind == "primitive":
        return PredicateConditionPrimitive(
            primitive=destack._generated.dir.type.primitive.from_json_primitive_type(
                json_field(object_, "primitive")
            )
        )
    elif kind == "type":
        return PredicateConditionType(
            type=destack._generated.dir.type.type.from_json_global_type_id(
                json_field(object_, "type")
            )
        )
    elif kind == "subtype":
        return PredicateConditionSubtype(
            subtype=destack._generated.dir.type.type.from_json_global_type_id(
                json_field(object_, "subtype")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class PredicateRange:
    """Scalar interval condition."""

    # the scalar domain constrained by the range
    domain: destack._generated.dir.type.type.GlobalTypeId
    # the optional committed lower bound
    start: destack._generated.dir.tree.literal.ScalarLiteral | None
    # the optional committed upper bound
    end: destack._generated.dir.tree.literal.ScalarLiteral | None
    # whether the upper bound is inclusive
    end_bound: destack._generated.dir.tree.operator.RangeEnd

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_predicate_range(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PredicateRange:
        """Decode one PredicateRange."""
        return decode_predicate_range(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_predicate_range(self)

    @classmethod
    def from_json(cls, value: Json) -> PredicateRange:
        """Return one PredicateRange from one JSON value."""
        return from_json_predicate_range(value)


def encode_predicate_range(writer: BinaryWriter, value: PredicateRange) -> None:
    """Encode one PredicateRange."""
    destack._generated.dir.type.type.encode_global_type_id(writer, value.domain)
    if value.start is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.literal.encode_scalar_literal(writer, value.start)
    if value.end is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.literal.encode_scalar_literal(writer, value.end)
    destack._generated.dir.tree.operator.encode_range_end(writer, value.end_bound)


def decode_predicate_range(reader: BinaryReader) -> PredicateRange:
    """Decode one PredicateRange."""
    domain = destack._generated.dir.type.type.decode_global_type_id(reader)
    start = reader.read_option(
        lambda: destack._generated.dir.tree.literal.decode_scalar_literal(reader)
    )
    end = reader.read_option(
        lambda: destack._generated.dir.tree.literal.decode_scalar_literal(reader)
    )
    end_bound = destack._generated.dir.tree.operator.decode_range_end(reader)

    return PredicateRange(
        domain=domain,
        start=start,
        end=end,
        end_bound=end_bound,
    )


def to_json_predicate_range(value: PredicateRange) -> Json:
    """Return one JSON value for one PredicateRange."""
    return {
        "domain": destack._generated.dir.type.type.to_json_global_type_id(value.domain),
        **(
            {}
            if value.start is None
            else {
                "start": destack._generated.dir.tree.literal.to_json_scalar_literal(
                    value.start
                )
            }
        ),
        **(
            {}
            if value.end is None
            else {
                "end": destack._generated.dir.tree.literal.to_json_scalar_literal(
                    value.end
                )
            }
        ),
        "endBound": destack._generated.dir.tree.operator.to_json_range_end(
            value.end_bound
        ),
    }


def from_json_predicate_range(value: Json) -> PredicateRange:
    """Return one PredicateRange from one JSON value."""
    object_ = json_object(value)

    return PredicateRange(
        domain=destack._generated.dir.type.type.from_json_global_type_id(
            json_field(object_, "domain")
        ),
        start=json_optional(
            object_,
            "start",
            lambda value: destack._generated.dir.tree.literal.from_json_scalar_literal(
                value
            ),
        ),
        end=json_optional(
            object_,
            "end",
            lambda value: destack._generated.dir.tree.literal.from_json_scalar_literal(
                value
            ),
        ),
        end_bound=destack._generated.dir.tree.operator.from_json_range_end(
            json_field(object_, "endBound")
        ),
    )


@dataclass(frozen=True, slots=True)
class PredicateMembershipTest:
    """Structural membership predicate."""

    # the receiver value
    receiver: PredicateOperand
    # the tested key
    key: PredicateKey

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_predicate_membership_test(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PredicateMembershipTest:
        """Decode one PredicateMembershipTest."""
        return decode_predicate_membership_test(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_predicate_membership_test(self)

    @classmethod
    def from_json(cls, value: Json) -> PredicateMembershipTest:
        """Return one PredicateMembershipTest from one JSON value."""
        return from_json_predicate_membership_test(value)


def encode_predicate_membership_test(
    writer: BinaryWriter, value: PredicateMembershipTest
) -> None:
    """Encode one PredicateMembershipTest."""
    encode_predicate_operand(writer, value.receiver)
    encode_predicate_key(writer, value.key)


def decode_predicate_membership_test(reader: BinaryReader) -> PredicateMembershipTest:
    """Decode one PredicateMembershipTest."""
    receiver = decode_predicate_operand(reader)
    key = decode_predicate_key(reader)

    return PredicateMembershipTest(
        receiver=receiver,
        key=key,
    )


def to_json_predicate_membership_test(value: PredicateMembershipTest) -> Json:
    """Return one JSON value for one PredicateMembershipTest."""
    return {
        "receiver": to_json_predicate_operand(value.receiver),
        "key": to_json_predicate_key(value.key),
    }


def from_json_predicate_membership_test(value: Json) -> PredicateMembershipTest:
    """Return one PredicateMembershipTest from one JSON value."""
    object_ = json_object(value)

    return PredicateMembershipTest(
        receiver=from_json_predicate_operand(json_field(object_, "receiver")),
        key=from_json_predicate_key(json_field(object_, "key")),
    )


@dataclass(frozen=True, slots=True)
class PredicateKeyStatic:
    """Statically known property key."""

    static: destack._generated.dir.symbol.key.StaticKey
    kind: typing.Literal["static"] = "static"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_predicate_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_predicate_key(self)


@dataclass(frozen=True, slots=True)
class PredicateKeyDynamic:
    """Runtime-computed property key."""

    dynamic: PredicateOperand
    kind: typing.Literal["dynamic"] = "dynamic"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_predicate_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_predicate_key(self)


"""Key tested by one structural membership predicate."""
PredicateKey: typing.TypeAlias = PredicateKeyStatic | PredicateKeyDynamic


def encode_predicate_key(writer: BinaryWriter, value: PredicateKey) -> None:
    """Encode one PredicateKey."""
    if value.kind == "static":
        writer.write_unsigned(0)
        destack._generated.dir.symbol.key.encode_static_key(writer, value.static)
    elif value.kind == "dynamic":
        writer.write_unsigned(1)
        encode_predicate_operand(writer, value.dynamic)
    else:
        raise SerdeError("unknown enum variant")


def decode_predicate_key(reader: BinaryReader) -> PredicateKey:
    """Decode one PredicateKey."""
    variant = reader.read_number()

    if variant == 0:
        static = destack._generated.dir.symbol.key.decode_static_key(reader)

        return PredicateKeyStatic(static=static)
    elif variant == 1:
        dynamic = decode_predicate_operand(reader)

        return PredicateKeyDynamic(dynamic=dynamic)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_predicate_key(value: PredicateKey) -> Json:
    """Return one JSON value for one PredicateKey."""
    if value.kind == "static":
        return {
            "kind": "static",
            "static": destack._generated.dir.symbol.key.to_json_static_key(
                value.static
            ),
        }
    elif value.kind == "dynamic":
        return {
            "kind": "dynamic",
            "dynamic": to_json_predicate_operand(value.dynamic),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_predicate_key(value: Json) -> PredicateKey:
    """Return one PredicateKey from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "static":
        return PredicateKeyStatic(
            static=destack._generated.dir.symbol.key.from_json_static_key(
                json_field(object_, "static")
            )
        )
    elif kind == "dynamic":
        return PredicateKeyDynamic(
            dynamic=from_json_predicate_operand(json_field(object_, "dynamic"))
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


__all__ = [
    "Predicate",
    "encode_predicate",
    "decode_predicate",
    "to_json_predicate",
    "from_json_predicate",
    "PredicateTest",
    "encode_predicate_test",
    "decode_predicate_test",
    "to_json_predicate_test",
    "from_json_predicate_test",
    "PredicateTestUnary",
    "PredicateTestMembership",
    "PredicateTestAny",
    "PredicateUnaryTest",
    "encode_predicate_unary_test",
    "decode_predicate_unary_test",
    "to_json_predicate_unary_test",
    "from_json_predicate_unary_test",
    "PredicateOperand",
    "encode_predicate_operand",
    "decode_predicate_operand",
    "to_json_predicate_operand",
    "from_json_predicate_operand",
    "PredicateCondition",
    "encode_predicate_condition",
    "decode_predicate_condition",
    "to_json_predicate_condition",
    "from_json_predicate_condition",
    "PredicateConditionAlways",
    "PredicateConditionNever",
    "PredicateConditionLiteral",
    "PredicateConditionRange",
    "PredicateConditionPrimitive",
    "PredicateConditionType",
    "PredicateConditionSubtype",
    "PredicateRange",
    "encode_predicate_range",
    "decode_predicate_range",
    "to_json_predicate_range",
    "from_json_predicate_range",
    "PredicateMembershipTest",
    "encode_predicate_membership_test",
    "decode_predicate_membership_test",
    "to_json_predicate_membership_test",
    "from_json_predicate_membership_test",
    "PredicateKey",
    "encode_predicate_key",
    "decode_predicate_key",
    "to_json_predicate_key",
    "from_json_predicate_key",
    "PredicateKeyStatic",
    "PredicateKeyDynamic",
]
