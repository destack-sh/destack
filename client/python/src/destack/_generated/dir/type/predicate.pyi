# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Predicate: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Predicate: ...

def encode_predicate(writer: BinaryWriter, value: Predicate) -> None: ...
def decode_predicate(reader: BinaryReader) -> Predicate: ...
def to_json_predicate(value: Predicate) -> Json: ...
def from_json_predicate(value: Json) -> Predicate: ...

@dataclass(frozen=True, slots=True)
class PredicateTestUnary:
    """Unary test over one input value."""

    unary: PredicateUnaryTest
    kind: typing.Literal["unary"] = "unary"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PredicateTestMembership:
    """Structural membership test over a receiver and key."""

    membership: PredicateMembershipTest
    kind: typing.Literal["membership"] = "membership"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PredicateTestAny:
    """Predicate that accepts when any alternative accepts."""

    any: Sequence[Predicate]
    kind: typing.Literal["any"] = "any"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Predicate test selected during checking."""
PredicateTest: typing.TypeAlias = (
    PredicateTestUnary | PredicateTestMembership | PredicateTestAny
)

def encode_predicate_test(writer: BinaryWriter, value: PredicateTest) -> None: ...
def decode_predicate_test(reader: BinaryReader) -> PredicateTest: ...
def to_json_predicate_test(value: PredicateTest) -> Json: ...
def from_json_predicate_test(value: Json) -> PredicateTest: ...

@dataclass(frozen=True, slots=True)
class PredicateUnaryTest:
    """Unary predicate over one input value."""

    # the value tested by this predicate
    input: PredicateOperand
    # the condition applied to the projected input
    condition: PredicateCondition

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PredicateUnaryTest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PredicateUnaryTest: ...

def encode_predicate_unary_test(
    writer: BinaryWriter, value: PredicateUnaryTest
) -> None: ...
def decode_predicate_unary_test(reader: BinaryReader) -> PredicateUnaryTest: ...
def to_json_predicate_unary_test(value: PredicateUnaryTest) -> Json: ...
def from_json_predicate_unary_test(value: Json) -> PredicateUnaryTest: ...

@dataclass(frozen=True, slots=True)
class PredicateOperand:
    """Value tested by one executable predicate."""

    # the operand value type
    ty: destack._generated.dir.type.type.GlobalTypeId
    # the projection used to compute the tested value, when one is needed
    projection: destack._generated.dir.type.projection.Projection | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PredicateOperand: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PredicateOperand: ...

def encode_predicate_operand(writer: BinaryWriter, value: PredicateOperand) -> None: ...
def decode_predicate_operand(reader: BinaryReader) -> PredicateOperand: ...
def to_json_predicate_operand(value: PredicateOperand) -> Json: ...
def from_json_predicate_operand(value: Json) -> PredicateOperand: ...

@dataclass(frozen=True, slots=True)
class PredicateConditionAlways:
    """Condition that accepts any projected input."""

    kind: typing.Literal["always"] = "always"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PredicateConditionNever:
    """Condition that rejects every projected input."""

    kind: typing.Literal["never"] = "never"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PredicateConditionLiteral:
    """Scalar literal condition, like `"ready"` or `0`."""

    literal: destack._generated.dir.tree.literal.ScalarLiteral
    kind: typing.Literal["literal"] = "literal"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PredicateConditionRange:
    """Scalar interval condition, like `0..=255`."""

    range: PredicateRange
    kind: typing.Literal["range"] = "range"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PredicateConditionPrimitive:
    """Primitive tag condition, like `string` or `int32`."""

    primitive: destack._generated.dir.type.primitive.PrimitiveType
    kind: typing.Literal["primitive"] = "primitive"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PredicateConditionType:
    """Exact runtime type descriptor condition."""

    type: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["type"] = "type"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PredicateConditionSubtype:
    """Runtime subtype descriptor condition."""

    subtype: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["subtype"] = "subtype"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

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

def encode_predicate_condition(
    writer: BinaryWriter, value: PredicateCondition
) -> None: ...
def decode_predicate_condition(reader: BinaryReader) -> PredicateCondition: ...
def to_json_predicate_condition(value: PredicateCondition) -> Json: ...
def from_json_predicate_condition(value: Json) -> PredicateCondition: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PredicateRange: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PredicateRange: ...

def encode_predicate_range(writer: BinaryWriter, value: PredicateRange) -> None: ...
def decode_predicate_range(reader: BinaryReader) -> PredicateRange: ...
def to_json_predicate_range(value: PredicateRange) -> Json: ...
def from_json_predicate_range(value: Json) -> PredicateRange: ...

@dataclass(frozen=True, slots=True)
class PredicateMembershipTest:
    """Structural membership predicate."""

    # the receiver value
    receiver: PredicateOperand
    # the tested key
    key: PredicateKey

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PredicateMembershipTest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PredicateMembershipTest: ...

def encode_predicate_membership_test(
    writer: BinaryWriter, value: PredicateMembershipTest
) -> None: ...
def decode_predicate_membership_test(
    reader: BinaryReader,
) -> PredicateMembershipTest: ...
def to_json_predicate_membership_test(value: PredicateMembershipTest) -> Json: ...
def from_json_predicate_membership_test(value: Json) -> PredicateMembershipTest: ...

@dataclass(frozen=True, slots=True)
class PredicateKeyStatic:
    """Statically known property key."""

    static: destack._generated.dir.symbol.key.StaticKey
    kind: typing.Literal["static"] = "static"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class PredicateKeyDynamic:
    """Runtime-computed property key."""

    dynamic: PredicateOperand
    kind: typing.Literal["dynamic"] = "dynamic"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Key tested by one structural membership predicate."""
PredicateKey: typing.TypeAlias = PredicateKeyStatic | PredicateKeyDynamic

def encode_predicate_key(writer: BinaryWriter, value: PredicateKey) -> None: ...
def decode_predicate_key(reader: BinaryReader) -> PredicateKey: ...
def to_json_predicate_key(value: PredicateKey) -> Json: ...
def from_json_predicate_key(value: Json) -> PredicateKey: ...

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
