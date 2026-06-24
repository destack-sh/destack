# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

from destack._impl.dir.tree.argument import (
    GenericParameterImpl,
)
from destack._impl.dir.tree.argument import (
    ParameterImpl,
)

import destack._generated.core.string
import destack._generated.dir.tree.key
import destack._generated.dir.tree.node
import destack._generated.dir.tree.property

@dataclass(frozen=True, slots=True)
class GenericParameterType(GenericParameterImpl):
    """Type parameter."""

    name: destack._generated.core.string.StringId
    variance: destack._generated.dir.tree.property.VarianceModifier | None
    constraint: destack._generated.dir.tree.node.LocalNodeId | None
    default: destack._generated.dir.tree.node.LocalNodeId | None
    is_const: bool
    kind: typing.Literal["type"] = "type"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class GenericParameterVariadicType(GenericParameterImpl):
    """Variadic type parameter."""

    name: destack._generated.core.string.StringId
    variance: destack._generated.dir.tree.property.VarianceModifier | None
    constraint: destack._generated.dir.tree.node.LocalNodeId | None
    default: destack._generated.dir.tree.node.LocalNodeId | None
    is_const: bool
    kind: typing.Literal["variadicType"] = "variadicType"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class GenericParameterValue(GenericParameterImpl):
    """Value parameter."""

    name: destack._generated.core.string.StringId
    declared_type: destack._generated.dir.tree.node.LocalNodeId | None
    default: destack._generated.dir.tree.node.LocalNodeId | None
    is_comptime: bool
    kind: typing.Literal["value"] = "value"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class GenericParameterVariadicValue(GenericParameterImpl):
    """Variadic value parameter."""

    name: destack._generated.core.string.StringId
    declared_type: destack._generated.dir.tree.node.LocalNodeId | None
    default: destack._generated.dir.tree.node.LocalNodeId | None
    is_comptime: bool
    kind: typing.Literal["variadicValue"] = "variadicValue"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class GenericParameterError(GenericParameterImpl):
    """Malformed generic parameter."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""A declared generic parameter in source."""
GenericParameter: typing.TypeAlias = (
    GenericParameterType
    | GenericParameterVariadicType
    | GenericParameterValue
    | GenericParameterVariadicValue
    | GenericParameterError
)

def encode_generic_parameter(writer: BinaryWriter, value: GenericParameter) -> None: ...
def decode_generic_parameter(reader: BinaryReader) -> GenericParameter: ...
def to_json_generic_parameter(value: GenericParameter) -> Json: ...
def from_json_generic_parameter(value: Json) -> GenericParameter: ...

@dataclass(frozen=True, slots=True)
class ParameterNamed(ParameterImpl):
    """Named scalar parameter."""

    name: destack._generated.core.string.StringId
    declared_type: destack._generated.dir.tree.node.LocalNodeId | None
    default: destack._generated.dir.tree.node.LocalNodeId | None
    is_optional: bool
    is_comptime: bool
    kind: typing.Literal["named"] = "named"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ParameterPattern(ParameterImpl):
    """Pattern parameter."""

    pattern: destack._generated.dir.tree.node.LocalNodeId
    declared_type: destack._generated.dir.tree.node.LocalNodeId | None
    default: destack._generated.dir.tree.node.LocalNodeId | None
    is_optional: bool
    is_comptime: bool
    kind: typing.Literal["pattern"] = "pattern"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ParameterVariadicNamed(ParameterImpl):
    """Variadic named parameter."""

    name: destack._generated.core.string.StringId
    declared_type: destack._generated.dir.tree.node.LocalNodeId | None
    is_comptime: bool
    kind: typing.Literal["variadicNamed"] = "variadicNamed"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ParameterVariadicPattern(ParameterImpl):
    """Variadic pattern parameter."""

    pattern: destack._generated.dir.tree.node.LocalNodeId
    declared_type: destack._generated.dir.tree.node.LocalNodeId | None
    is_comptime: bool
    kind: typing.Literal["variadicPattern"] = "variadicPattern"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ParameterError(ParameterImpl):
    """Malformed parameter."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""A parameter to a callable construct."""
Parameter: typing.TypeAlias = (
    ParameterNamed
    | ParameterPattern
    | ParameterVariadicNamed
    | ParameterVariadicPattern
    | ParameterError
)

def encode_parameter(writer: BinaryWriter, value: Parameter) -> None: ...
def decode_parameter(reader: BinaryReader) -> Parameter: ...
def to_json_parameter(value: Parameter) -> Json: ...
def from_json_parameter(value: Json) -> Parameter: ...

@dataclass(frozen=True, slots=True)
class GenericArgumentType:
    """Type generic argument."""

    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["type"] = "type"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class GenericArgumentSpreadType:
    """Spread type generic argument."""

    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["spreadType"] = "spreadType"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class GenericArgumentValue:
    """Value generic argument."""

    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["value"] = "value"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class GenericArgumentSpreadValue:
    """Spread value generic argument."""

    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["spreadValue"] = "spreadValue"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class GenericArgumentAssociatedType:
    """Associated type refinement."""

    name: destack._generated.core.string.StringId
    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["associatedType"] = "associatedType"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class GenericArgumentAssociatedConst:
    """Associated compile-time constant refinement."""

    name: destack._generated.core.string.StringId
    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["associatedConst"] = "associatedConst"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class GenericArgumentError:
    """Malformed generic argument slot."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""A generic argument in static argument position."""
GenericArgument: typing.TypeAlias = (
    GenericArgumentType
    | GenericArgumentSpreadType
    | GenericArgumentValue
    | GenericArgumentSpreadValue
    | GenericArgumentAssociatedType
    | GenericArgumentAssociatedConst
    | GenericArgumentError
)

def encode_generic_argument(writer: BinaryWriter, value: GenericArgument) -> None: ...
def decode_generic_argument(reader: BinaryReader) -> GenericArgument: ...
def to_json_generic_argument(value: GenericArgument) -> Json: ...
def from_json_generic_argument(value: Json) -> GenericArgument: ...

@dataclass(frozen=True, slots=True)
class TupleElementElement:
    """One non-spread tuple element."""

    label: destack._generated.core.string.StringId | None
    value: destack._generated.dir.tree.node.LocalNodeId
    is_optional: bool
    is_readonly: bool
    kind: typing.Literal["element"] = "element"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TupleElementSpread:
    """One spread tuple element."""

    label: destack._generated.core.string.StringId | None
    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["spread"] = "spread"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TupleElementError:
    """Malformed tuple element slot."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""One tuple type element."""
TupleElement: typing.TypeAlias = (
    TupleElementElement | TupleElementSpread | TupleElementError
)

def encode_tuple_element(writer: BinaryWriter, value: TupleElement) -> None: ...
def decode_tuple_element(reader: BinaryReader) -> TupleElement: ...
def to_json_tuple_element(value: TupleElement) -> Json: ...
def from_json_tuple_element(value: Json) -> TupleElement: ...

@dataclass(frozen=True, slots=True)
class ArgumentNamed:
    """Named argument."""

    name: destack._generated.dir.tree.key.Name
    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["named"] = "named"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArgumentLabeled:
    """Labeled argument."""

    label: destack._generated.core.string.StringId
    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["labeled"] = "labeled"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArgumentPositional:
    """Positional argument."""

    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["positional"] = "positional"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArgumentSpread:
    """Spread argument."""

    label: destack._generated.core.string.StringId | None
    value: destack._generated.dir.tree.node.LocalNodeId
    kind: typing.Literal["spread"] = "spread"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArgumentError:
    """Malformed argument slot."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""An argument to a runtime call or tree construct."""
Argument: typing.TypeAlias = (
    ArgumentNamed
    | ArgumentLabeled
    | ArgumentPositional
    | ArgumentSpread
    | ArgumentError
)

def encode_argument(writer: BinaryWriter, value: Argument) -> None: ...
def decode_argument(reader: BinaryReader) -> Argument: ...
def to_json_argument(value: Argument) -> Json: ...
def from_json_argument(value: Json) -> Argument: ...

__all__ = [
    "GenericParameter",
    "encode_generic_parameter",
    "decode_generic_parameter",
    "to_json_generic_parameter",
    "from_json_generic_parameter",
    "GenericParameterType",
    "GenericParameterVariadicType",
    "GenericParameterValue",
    "GenericParameterVariadicValue",
    "GenericParameterError",
    "Parameter",
    "encode_parameter",
    "decode_parameter",
    "to_json_parameter",
    "from_json_parameter",
    "ParameterNamed",
    "ParameterPattern",
    "ParameterVariadicNamed",
    "ParameterVariadicPattern",
    "ParameterError",
    "GenericArgument",
    "encode_generic_argument",
    "decode_generic_argument",
    "to_json_generic_argument",
    "from_json_generic_argument",
    "GenericArgumentType",
    "GenericArgumentSpreadType",
    "GenericArgumentValue",
    "GenericArgumentSpreadValue",
    "GenericArgumentAssociatedType",
    "GenericArgumentAssociatedConst",
    "GenericArgumentError",
    "TupleElement",
    "encode_tuple_element",
    "decode_tuple_element",
    "to_json_tuple_element",
    "from_json_tuple_element",
    "TupleElementElement",
    "TupleElementSpread",
    "TupleElementError",
    "Argument",
    "encode_argument",
    "decode_argument",
    "to_json_argument",
    "from_json_argument",
    "ArgumentNamed",
    "ArgumentLabeled",
    "ArgumentPositional",
    "ArgumentSpread",
    "ArgumentError",
]
