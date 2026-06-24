# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.string
import destack._generated.js.tree.node

@dataclass(frozen=True, slots=True)
class BindingModifier:
    """The modifiers of a field-like item."""

    # the kind of the binding
    kind: BindingKind | None
    # the variance of a type parameter
    variance: VarianceModifier | None
    # the scope of the binding
    anchor: BindingAnchor | None
    # the mutability of the field
    mutability: destack._generated.js.tree.node.Mutability | None
    # the visibility of the field
    visibility: destack._generated.js.tree.node.Visibility | None
    # the operator to apply to the binding
    operator: BindingOperator | None
    # whether the binding uses a definite assignment assertion
    definite: bool
    # the accessor kind of the binding
    accessor: AccessorKind | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> BindingModifier: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> BindingModifier: ...

def encode_binding_modifier(writer: BinaryWriter, value: BindingModifier) -> None: ...
def decode_binding_modifier(reader: BinaryReader) -> BindingModifier: ...
def to_json_binding_modifier(value: BindingModifier) -> Json: ...
def from_json_binding_modifier(value: Json) -> BindingModifier: ...

"""The type of a binding."""
BindingKind: typing.TypeAlias = typing.Literal["must"] | typing.Literal["maybe"]

def encode_binding_kind(writer: BinaryWriter, value: BindingKind) -> None: ...
def decode_binding_kind(reader: BinaryReader) -> BindingKind: ...
def to_json_binding_kind(value: BindingKind) -> Json: ...
def from_json_binding_kind(value: Json) -> BindingKind: ...

"""Variance annotation for type parameters."""
VarianceModifier: typing.TypeAlias = (
    typing.Literal["in"] | typing.Literal["out"] | typing.Literal["inOut"]
)

def encode_variance_modifier(writer: BinaryWriter, value: VarianceModifier) -> None: ...
def decode_variance_modifier(reader: BinaryReader) -> VarianceModifier: ...
def to_json_variance_modifier(value: VarianceModifier) -> Json: ...
def from_json_variance_modifier(value: Json) -> VarianceModifier: ...

"""The scope of a binding (dynamic or static)."""
BindingAnchor: typing.TypeAlias = typing.Literal["instance"] | typing.Literal["static"]

def encode_binding_anchor(writer: BinaryWriter, value: BindingAnchor) -> None: ...
def decode_binding_anchor(reader: BinaryReader) -> BindingAnchor: ...
def to_json_binding_anchor(value: BindingAnchor) -> Json: ...
def from_json_binding_anchor(value: Json) -> BindingAnchor: ...

"""The operator to apply to the binding."""
BindingOperator: typing.TypeAlias = typing.Literal["asConst"]

def encode_binding_operator(writer: BinaryWriter, value: BindingOperator) -> None: ...
def decode_binding_operator(reader: BinaryReader) -> BindingOperator: ...
def to_json_binding_operator(value: BindingOperator) -> Json: ...
def from_json_binding_operator(value: Json) -> BindingOperator: ...

"""The accessor kind of a binding."""
AccessorKind: typing.TypeAlias = typing.Literal["accessor"]

def encode_accessor_kind(writer: BinaryWriter, value: AccessorKind) -> None: ...
def decode_accessor_kind(reader: BinaryReader) -> AccessorKind: ...
def to_json_accessor_kind(value: AccessorKind) -> Json: ...
def from_json_accessor_kind(value: Json) -> AccessorKind: ...

@dataclass(frozen=True, slots=True)
class GenericParameterType:
    """Type parameter."""

    modifiers: BindingModifier | None
    name: destack._generated.core.string.StringId
    constraint: destack._generated.js.tree.node.LocalNodeId | None
    default: destack._generated.js.tree.node.LocalNodeId | None
    kind: typing.Literal["type"] = "type"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""One generic parameter."""
GenericParameter: typing.TypeAlias = GenericParameterType

def encode_generic_parameter(writer: BinaryWriter, value: GenericParameter) -> None: ...
def decode_generic_parameter(reader: BinaryReader) -> GenericParameter: ...
def to_json_generic_parameter(value: GenericParameter) -> Json: ...
def from_json_generic_parameter(value: Json) -> GenericParameter: ...

@dataclass(frozen=True, slots=True)
class ParameterNamed:
    """Named parameter (like `x: int32` or `Validate: boolean = true`)."""

    modifiers: BindingModifier | None
    name: destack._generated.core.string.StringId
    ty: destack._generated.js.tree.node.LocalNodeId | None
    default: destack._generated.js.tree.node.LocalNodeId | None
    kind: typing.Literal["named"] = "named"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ParameterPattern:
    """Pattern parameter (like `_` or `{ x }` or `{ x, ..rest }: MyType = Foo`)."""

    modifiers: BindingModifier | None
    pattern: destack._generated.js.tree.node.LocalNodeId
    ty: destack._generated.js.tree.node.LocalNodeId | None
    default: destack._generated.js.tree.node.LocalNodeId | None
    kind: typing.Literal["pattern"] = "pattern"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ParameterVariadicNamed:
    """Variadic parameter with a named binding (like `...args: int32[]`)."""

    modifiers: BindingModifier | None
    name: destack._generated.core.string.StringId
    ty: destack._generated.js.tree.node.LocalNodeId | None
    kind: typing.Literal["variadicNamed"] = "variadicNamed"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ParameterVariadicPattern:
    """Variadic parameter with a pattern binding (like `...[a, b]`)."""

    modifiers: BindingModifier | None
    pattern: destack._generated.js.tree.node.LocalNodeId
    ty: destack._generated.js.tree.node.LocalNodeId | None
    kind: typing.Literal["variadicPattern"] = "variadicPattern"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Named or positional parameter to some construct."""
Parameter: typing.TypeAlias = (
    ParameterNamed
    | ParameterPattern
    | ParameterVariadicNamed
    | ParameterVariadicPattern
)

def encode_parameter(writer: BinaryWriter, value: Parameter) -> None: ...
def decode_parameter(reader: BinaryReader) -> Parameter: ...
def to_json_parameter(value: Parameter) -> Json: ...
def from_json_parameter(value: Json) -> Parameter: ...

@dataclass(frozen=True, slots=True)
class ArgumentPositional:
    """Positional argument (like `1` or `foo()`)."""

    value: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["positional"] = "positional"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArgumentSpread:
    """Spread argument (like `...args`)."""

    value: destack._generated.js.tree.node.LocalNodeId
    kind: typing.Literal["spread"] = "spread"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Positional argument to some construct."""
Argument: typing.TypeAlias = ArgumentPositional | ArgumentSpread

def encode_argument(writer: BinaryWriter, value: Argument) -> None: ...
def decode_argument(reader: BinaryReader) -> Argument: ...
def to_json_argument(value: Argument) -> Json: ...
def from_json_argument(value: Json) -> Argument: ...

__all__ = [
    "BindingModifier",
    "encode_binding_modifier",
    "decode_binding_modifier",
    "to_json_binding_modifier",
    "from_json_binding_modifier",
    "BindingKind",
    "encode_binding_kind",
    "decode_binding_kind",
    "to_json_binding_kind",
    "from_json_binding_kind",
    "VarianceModifier",
    "encode_variance_modifier",
    "decode_variance_modifier",
    "to_json_variance_modifier",
    "from_json_variance_modifier",
    "BindingAnchor",
    "encode_binding_anchor",
    "decode_binding_anchor",
    "to_json_binding_anchor",
    "from_json_binding_anchor",
    "BindingOperator",
    "encode_binding_operator",
    "decode_binding_operator",
    "to_json_binding_operator",
    "from_json_binding_operator",
    "AccessorKind",
    "encode_accessor_kind",
    "decode_accessor_kind",
    "to_json_accessor_kind",
    "from_json_accessor_kind",
    "GenericParameter",
    "encode_generic_parameter",
    "decode_generic_parameter",
    "to_json_generic_parameter",
    "from_json_generic_parameter",
    "GenericParameterType",
    "Parameter",
    "encode_parameter",
    "decode_parameter",
    "to_json_parameter",
    "from_json_parameter",
    "ParameterNamed",
    "ParameterPattern",
    "ParameterVariadicNamed",
    "ParameterVariadicPattern",
    "Argument",
    "encode_argument",
    "decode_argument",
    "to_json_argument",
    "from_json_argument",
    "ArgumentPositional",
    "ArgumentSpread",
]
