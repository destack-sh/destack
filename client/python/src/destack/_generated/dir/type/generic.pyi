# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.string
import destack._generated.dir.symbol.symbol
import destack._generated.dir.tree.node
import destack._generated.dir.tree.property
import destack._generated.dir.type.type
import destack._generated.source.file.model.module

@dataclass(frozen=True, slots=True)
class GlobalGenericParameterId:
    """Global generic parameter id across modules."""

    # the module id of the global generic parameter
    module_id: destack._generated.source.file.model.module.ModuleId
    # the local generic parameter id
    local_id: LocalGenericParameterId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> GlobalGenericParameterId: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> GlobalGenericParameterId: ...

def encode_global_generic_parameter_id(
    writer: BinaryWriter, value: GlobalGenericParameterId
) -> None: ...
def decode_global_generic_parameter_id(
    reader: BinaryReader,
) -> GlobalGenericParameterId: ...
def to_json_global_generic_parameter_id(value: GlobalGenericParameterId) -> Json: ...
def from_json_global_generic_parameter_id(value: Json) -> GlobalGenericParameterId: ...

"""Unique identifier for generic parameters."""
LocalGenericParameterId: typing.TypeAlias = int

def encode_local_generic_parameter_id(
    writer: BinaryWriter, value: LocalGenericParameterId
) -> None: ...
def decode_local_generic_parameter_id(
    reader: BinaryReader,
) -> LocalGenericParameterId: ...
def to_json_local_generic_parameter_id(value: LocalGenericParameterId) -> Json: ...
def from_json_local_generic_parameter_id(value: Json) -> LocalGenericParameterId: ...

@dataclass(frozen=True, slots=True)
class GenericTemplate:
    """One declaration of generic parameters."""

    # the source node that declares this template
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the declaration symbol this template belongs to
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId | None
    # the immediately enclosing generic template
    parent: LocalGenericTemplateId | None
    # the generic parameters in declaration order
    parameters: Sequence[LocalGenericParameterId]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> GenericTemplate: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> GenericTemplate: ...

def encode_generic_template(writer: BinaryWriter, value: GenericTemplate) -> None: ...
def decode_generic_template(reader: BinaryReader) -> GenericTemplate: ...
def to_json_generic_template(value: GenericTemplate) -> Json: ...
def from_json_generic_template(value: Json) -> GenericTemplate: ...

"""Unique identifier for generic templates."""
LocalGenericTemplateId: typing.TypeAlias = int

def encode_local_generic_template_id(
    writer: BinaryWriter, value: LocalGenericTemplateId
) -> None: ...
def decode_local_generic_template_id(
    reader: BinaryReader,
) -> LocalGenericTemplateId: ...
def to_json_local_generic_template_id(value: LocalGenericTemplateId) -> Json: ...
def from_json_local_generic_template_id(value: Json) -> LocalGenericTemplateId: ...

@dataclass(frozen=True, slots=True)
class GenericParameterBinding:
    """One declaration-side generic parameter."""

    # the generic template that owns this parameter
    template: LocalGenericTemplateId
    # the parameter key
    key: GenericParameterKey
    # the parameter variance, rejected on comptime parameters
    variance: destack._generated.dir.tree.property.VarianceModifier | None
    # the optional constraint
    constraint: destack._generated.dir.type.type.GlobalTypeId | None
    # the optional default
    default: destack._generated.dir.type.type.GlobalTypeId | None
    # the parameter origin
    origin: GenericParameterOrigin
    # whether the parameter captures remaining arguments
    is_variadic: bool
    # whether arguments must solve to singleton types
    is_comptime: bool

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> GenericParameterBinding: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> GenericParameterBinding: ...

def encode_generic_parameter_binding(
    writer: BinaryWriter, value: GenericParameterBinding
) -> None: ...
def decode_generic_parameter_binding(
    reader: BinaryReader,
) -> GenericParameterBinding: ...
def to_json_generic_parameter_binding(value: GenericParameterBinding) -> Json: ...
def from_json_generic_parameter_binding(value: Json) -> GenericParameterBinding: ...

@dataclass(frozen=True, slots=True)
class GenericParameterKeySymbol:
    """Explicit source symbol, like the `T` in `<T>`."""

    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    kind: typing.Literal["symbol"] = "symbol"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class GenericParameterKeyGenerated:
    """Generated checked parameter key, like the induced `T0` or `L0`."""

    generated: destack._generated.core.string.StringId
    kind: typing.Literal["generated"] = "generated"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""User-visible key of one generic parameter."""
GenericParameterKey: typing.TypeAlias = (
    GenericParameterKeySymbol | GenericParameterKeyGenerated
)

def encode_generic_parameter_key(
    writer: BinaryWriter, value: GenericParameterKey
) -> None: ...
def decode_generic_parameter_key(reader: BinaryReader) -> GenericParameterKey: ...
def to_json_generic_parameter_key(value: GenericParameterKey) -> Json: ...
def from_json_generic_parameter_key(value: Json) -> GenericParameterKey: ...

@dataclass(frozen=True, slots=True)
class GenericParameterOriginExplicit:
    """The parameter was written in source, like the `T` in `<T extends Clone>`."""

    kind: typing.Literal["explicit"] = "explicit"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class GenericParameterOriginInduced:
    """The parameter was induced by check, like the hidden `T0` a"""

    induced: GenericParameterInduction
    kind: typing.Literal["induced"] = "induced"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Source that introduced one generic parameter."""
GenericParameterOrigin: typing.TypeAlias = (
    GenericParameterOriginExplicit | GenericParameterOriginInduced
)

def encode_generic_parameter_origin(
    writer: BinaryWriter, value: GenericParameterOrigin
) -> None: ...
def decode_generic_parameter_origin(reader: BinaryReader) -> GenericParameterOrigin: ...
def to_json_generic_parameter_origin(value: GenericParameterOrigin) -> Json: ...
def from_json_generic_parameter_origin(value: Json) -> GenericParameterOrigin: ...

"""Reason one generic parameter was induced."""
GenericParameterInduction: typing.TypeAlias = (
    typing.Literal["parameterConstraint"]
    | typing.Literal["storageConstraint"]
    | typing.Literal["form"]
    | typing.Literal["comptime"]
)

def encode_generic_parameter_induction(
    writer: BinaryWriter, value: GenericParameterInduction
) -> None: ...
def decode_generic_parameter_induction(
    reader: BinaryReader,
) -> GenericParameterInduction: ...
def to_json_generic_parameter_induction(value: GenericParameterInduction) -> Json: ...
def from_json_generic_parameter_induction(value: Json) -> GenericParameterInduction: ...

__all__ = [
    "GlobalGenericParameterId",
    "encode_global_generic_parameter_id",
    "decode_global_generic_parameter_id",
    "to_json_global_generic_parameter_id",
    "from_json_global_generic_parameter_id",
    "LocalGenericParameterId",
    "encode_local_generic_parameter_id",
    "decode_local_generic_parameter_id",
    "to_json_local_generic_parameter_id",
    "from_json_local_generic_parameter_id",
    "GenericTemplate",
    "encode_generic_template",
    "decode_generic_template",
    "to_json_generic_template",
    "from_json_generic_template",
    "LocalGenericTemplateId",
    "encode_local_generic_template_id",
    "decode_local_generic_template_id",
    "to_json_local_generic_template_id",
    "from_json_local_generic_template_id",
    "GenericParameterBinding",
    "encode_generic_parameter_binding",
    "decode_generic_parameter_binding",
    "to_json_generic_parameter_binding",
    "from_json_generic_parameter_binding",
    "GenericParameterKey",
    "encode_generic_parameter_key",
    "decode_generic_parameter_key",
    "to_json_generic_parameter_key",
    "from_json_generic_parameter_key",
    "GenericParameterKeySymbol",
    "GenericParameterKeyGenerated",
    "GenericParameterOrigin",
    "encode_generic_parameter_origin",
    "decode_generic_parameter_origin",
    "to_json_generic_parameter_origin",
    "from_json_generic_parameter_origin",
    "GenericParameterOriginExplicit",
    "GenericParameterOriginInduced",
    "GenericParameterInduction",
    "encode_generic_parameter_induction",
    "decode_generic_parameter_induction",
    "to_json_generic_parameter_induction",
    "from_json_generic_parameter_induction",
]
