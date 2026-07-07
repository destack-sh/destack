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
    json_int,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.dir.symbol.key
import destack._generated.dir.symbol.symbol
import destack._generated.dir.table.definition
import destack._generated.dir.tree.literal
import destack._generated.dir.tree.node
import destack._generated.dir.tree.operator
import destack._generated.dir.type.generic
import destack._generated.dir.type.predicate
import destack._generated.dir.type.projection
import destack._generated.dir.type.type


@dataclass(frozen=True, slots=True)
class NameResolution:
    """Target selected by lexical or path lookup."""

    # the selected symbols in declaration order
    symbols: Sequence[destack._generated.dir.symbol.symbol.GlobalSymbolId]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_name_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> NameResolution:
        """Decode one NameResolution."""
        return decode_name_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_name_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> NameResolution:
        """Return one NameResolution from one JSON value."""
        return from_json_name_resolution(value)


def encode_name_resolution(writer: BinaryWriter, value: NameResolution) -> None:
    """Encode one NameResolution."""
    writer.write_unsigned(len(value.symbols))
    for item_value_symbols_0 in value.symbols:
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(
            writer, item_value_symbols_0
        )


def decode_name_resolution(reader: BinaryReader) -> NameResolution:
    """Decode one NameResolution."""
    symbols = [
        destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
        for _ in range(reader.read_number())
    ]

    return NameResolution(
        symbols=symbols,
    )


def to_json_name_resolution(value: NameResolution) -> Json:
    """Return one JSON value for one NameResolution."""
    return {
        "symbols": [
            destack._generated.dir.symbol.symbol.to_json_global_symbol_id(item_0)
            for item_0 in value.symbols
        ],
    }


def from_json_name_resolution(value: Json) -> NameResolution:
    """Return one NameResolution from one JSON value."""
    object_ = json_object(value)

    return NameResolution(
        symbols=[
            destack._generated.dir.symbol.symbol.from_json_global_symbol_id(item_0)
            for item_0 in json_array(json_field(object_, "symbols"))
        ],
    )


@dataclass(frozen=True, slots=True)
class InstantiationResolution:
    """Explicit generic application selected at a usage site."""

    # the generic declaration being applied
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the complete selected generic argument bindings
    generic_arguments: Sequence[
        destack._generated.dir.type.generic.GenericArgumentBinding
    ]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instantiation_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> InstantiationResolution:
        """Decode one InstantiationResolution."""
        return decode_instantiation_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instantiation_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> InstantiationResolution:
        """Return one InstantiationResolution from one JSON value."""
        return from_json_instantiation_resolution(value)


def encode_instantiation_resolution(
    writer: BinaryWriter, value: InstantiationResolution
) -> None:
    """Encode one InstantiationResolution."""
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(writer, value.symbol)
    writer.write_unsigned(len(value.generic_arguments))
    for item_value_generic_arguments_0 in value.generic_arguments:
        destack._generated.dir.type.generic.encode_generic_argument_binding(
            writer, item_value_generic_arguments_0
        )


def decode_instantiation_resolution(reader: BinaryReader) -> InstantiationResolution:
    """Decode one InstantiationResolution."""
    symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    generic_arguments = [
        destack._generated.dir.type.generic.decode_generic_argument_binding(reader)
        for _ in range(reader.read_number())
    ]

    return InstantiationResolution(
        symbol=symbol,
        generic_arguments=generic_arguments,
    )


def to_json_instantiation_resolution(value: InstantiationResolution) -> Json:
    """Return one JSON value for one InstantiationResolution."""
    return {
        "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.symbol
        ),
        "genericArguments": [
            destack._generated.dir.type.generic.to_json_generic_argument_binding(item_0)
            for item_0 in value.generic_arguments
        ],
    }


def from_json_instantiation_resolution(value: Json) -> InstantiationResolution:
    """Return one InstantiationResolution from one JSON value."""
    object_ = json_object(value)

    return InstantiationResolution(
        symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "symbol")
        ),
        generic_arguments=[
            destack._generated.dir.type.generic.from_json_generic_argument_binding(
                item_0
            )
            for item_0 in json_array(json_field(object_, "genericArguments"))
        ],
    )


@dataclass(frozen=True, slots=True)
class LabelResolutionSymbol:
    """An explicit label target."""

    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    kind: typing.Literal["symbol"] = "symbol"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_label_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_label_resolution(self)


@dataclass(frozen=True, slots=True)
class LabelResolutionLoop:
    """The nearest enclosing loop target."""

    kind: typing.Literal["loop"] = "loop"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_label_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_label_resolution(self)


@dataclass(frozen=True, slots=True)
class LabelResolutionFunction:
    """The enclosing function target."""

    kind: typing.Literal["function"] = "function"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_label_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_label_resolution(self)


"""Target selected by a labeled transfer."""
LabelResolution: typing.TypeAlias = (
    LabelResolutionSymbol | LabelResolutionLoop | LabelResolutionFunction
)


def encode_label_resolution(writer: BinaryWriter, value: LabelResolution) -> None:
    """Encode one LabelResolution."""
    if value.kind == "symbol":
        writer.write_unsigned(0)
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(
            writer, value.symbol
        )
    elif value.kind == "loop":
        writer.write_unsigned(1)
    elif value.kind == "function":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_label_resolution(reader: BinaryReader) -> LabelResolution:
    """Decode one LabelResolution."""
    variant = reader.read_number()

    if variant == 0:
        symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)

        return LabelResolutionSymbol(symbol=symbol)
    elif variant == 1:
        return LabelResolutionLoop()
    elif variant == 2:
        return LabelResolutionFunction()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_label_resolution(value: LabelResolution) -> Json:
    """Return one JSON value for one LabelResolution."""
    if value.kind == "symbol":
        return {
            "kind": "symbol",
            "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
                value.symbol
            ),
        }
    elif value.kind == "loop":
        return {
            "kind": "loop",
        }
    elif value.kind == "function":
        return {
            "kind": "function",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_label_resolution(value: Json) -> LabelResolution:
    """Return one LabelResolution from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "symbol":
        return LabelResolutionSymbol(
            symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
                json_field(object_, "symbol")
            )
        )
    elif kind == "loop":
        return LabelResolutionLoop()
    elif kind == "function":
        return LabelResolutionFunction()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class ReceiverResolution:
    """Receiver selected by contextual lookup, such as `this` or `super`."""

    # the receiver syntax kind
    kind: ReceiverKind
    # the declaration that introduces the receiver
    declaration: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the receiver type after inference
    ty: destack._generated.dir.type.type.GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_receiver_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ReceiverResolution:
        """Decode one ReceiverResolution."""
        return decode_receiver_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_receiver_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> ReceiverResolution:
        """Return one ReceiverResolution from one JSON value."""
        return from_json_receiver_resolution(value)


def encode_receiver_resolution(writer: BinaryWriter, value: ReceiverResolution) -> None:
    """Encode one ReceiverResolution."""
    encode_receiver_kind(writer, value.kind)
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(
        writer, value.declaration
    )
    destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)


def decode_receiver_resolution(reader: BinaryReader) -> ReceiverResolution:
    """Decode one ReceiverResolution."""
    kind = decode_receiver_kind(reader)
    declaration = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    ty = destack._generated.dir.type.type.decode_global_type_id(reader)

    return ReceiverResolution(
        kind=kind,
        declaration=declaration,
        ty=ty,
    )


def to_json_receiver_resolution(value: ReceiverResolution) -> Json:
    """Return one JSON value for one ReceiverResolution."""
    return {
        "kind": to_json_receiver_kind(value.kind),
        "declaration": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.declaration
        ),
        "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty),
    }


def from_json_receiver_resolution(value: Json) -> ReceiverResolution:
    """Return one ReceiverResolution from one JSON value."""
    object_ = json_object(value)

    return ReceiverResolution(
        kind=from_json_receiver_kind(json_field(object_, "kind")),
        declaration=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "declaration")
        ),
        ty=destack._generated.dir.type.type.from_json_global_type_id(
            json_field(object_, "ty")
        ),
    )


"""Receiver syntax resolved by contextual lookup."""
ReceiverKind: typing.TypeAlias = typing.Literal["this"] | typing.Literal["super"]


def encode_receiver_kind(writer: BinaryWriter, value: ReceiverKind) -> None:
    """Encode one ReceiverKind."""
    if value == "this":
        writer.write_unsigned(0)
    elif value == "super":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_receiver_kind(reader: BinaryReader) -> ReceiverKind:
    """Decode one ReceiverKind."""
    variant = reader.read_number()

    if variant == 0:
        return "this"
    elif variant == 1:
        return "super"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_receiver_kind(value: ReceiverKind) -> Json:
    """Return one JSON value for one ReceiverKind."""
    return value


def from_json_receiver_kind(value: Json) -> ReceiverKind:
    """Return one ReceiverKind from one JSON value."""
    variant = json_string(value)

    if variant == "this":
        return "this"
    elif variant == "super":
        return "super"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class MemberResolution:
    """Receiver member selected at a usage site."""

    # the receiver type after inference
    receiver: destack._generated.dir.type.type.GlobalTypeId
    # the selected member target
    target: MemberTarget

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_member_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> MemberResolution:
        """Decode one MemberResolution."""
        return decode_member_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_member_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> MemberResolution:
        """Return one MemberResolution from one JSON value."""
        return from_json_member_resolution(value)


def encode_member_resolution(writer: BinaryWriter, value: MemberResolution) -> None:
    """Encode one MemberResolution."""
    destack._generated.dir.type.type.encode_global_type_id(writer, value.receiver)
    encode_member_target(writer, value.target)


def decode_member_resolution(reader: BinaryReader) -> MemberResolution:
    """Decode one MemberResolution."""
    receiver = destack._generated.dir.type.type.decode_global_type_id(reader)
    target = decode_member_target(reader)

    return MemberResolution(
        receiver=receiver,
        target=target,
    )


def to_json_member_resolution(value: MemberResolution) -> Json:
    """Return one JSON value for one MemberResolution."""
    return {
        "receiver": destack._generated.dir.type.type.to_json_global_type_id(
            value.receiver
        ),
        "target": to_json_member_target(value.target),
    }


def from_json_member_resolution(value: Json) -> MemberResolution:
    """Return one MemberResolution from one JSON value."""
    object_ = json_object(value)

    return MemberResolution(
        receiver=destack._generated.dir.type.type.from_json_global_type_id(
            json_field(object_, "receiver")
        ),
        target=from_json_member_target(json_field(object_, "target")),
    )


@dataclass(frozen=True, slots=True)
class MemberTargetField:
    """Structural field selected from a shape type."""

    field: destack._generated.dir.symbol.key.StaticKey
    kind: typing.Literal["field"] = "field"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_member_target(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_member_target(self)


@dataclass(frozen=True, slots=True)
class MemberTargetElement:
    """Structural element selected from a tuple type."""

    element: int
    kind: typing.Literal["element"] = "element"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_member_target(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_member_target(self)


@dataclass(frozen=True, slots=True)
class MemberTargetIndex:
    """Structural index signature selected from a shape type."""

    index: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["index"] = "index"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_member_target(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_member_target(self)


@dataclass(frozen=True, slots=True)
class MemberTargetSymbol:
    """Exactly one symbol-backed member selected at compile time."""

    symbol: MemberCandidate
    kind: typing.Literal["symbol"] = "symbol"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_member_target(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_member_target(self)


@dataclass(frozen=True, slots=True)
class MemberTargetExistential:
    """Existential symbol-backed candidates deferred to call selection."""

    existential: Sequence[MemberCandidate]
    kind: typing.Literal["existential"] = "existential"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_member_target(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_member_target(self)


@dataclass(frozen=True, slots=True)
class MemberTargetUniversal:
    """Universal symbol-backed candidates deferred to call selection."""

    universal: Sequence[MemberCandidate]
    kind: typing.Literal["universal"] = "universal"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_member_target(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_member_target(self)


"""Member target selected at a usage site."""
MemberTarget: typing.TypeAlias = (
    MemberTargetField
    | MemberTargetElement
    | MemberTargetIndex
    | MemberTargetSymbol
    | MemberTargetExistential
    | MemberTargetUniversal
)


def encode_member_target(writer: BinaryWriter, value: MemberTarget) -> None:
    """Encode one MemberTarget."""
    if value.kind == "field":
        writer.write_unsigned(0)
        destack._generated.dir.symbol.key.encode_static_key(writer, value.field)
    elif value.kind == "element":
        writer.write_unsigned(1)
        writer.write_unsigned(value.element)
    elif value.kind == "index":
        writer.write_unsigned(2)
        destack._generated.dir.type.type.encode_global_type_id(writer, value.index)
    elif value.kind == "symbol":
        writer.write_unsigned(3)
        encode_member_candidate(writer, value.symbol)
    elif value.kind == "existential":
        writer.write_unsigned(4)
        writer.write_unsigned(len(value.existential))
        for item_value_existential_0 in value.existential:
            encode_member_candidate(writer, item_value_existential_0)
    elif value.kind == "universal":
        writer.write_unsigned(5)
        writer.write_unsigned(len(value.universal))
        for item_value_universal_0 in value.universal:
            encode_member_candidate(writer, item_value_universal_0)
    else:
        raise SerdeError("unknown enum variant")


def decode_member_target(reader: BinaryReader) -> MemberTarget:
    """Decode one MemberTarget."""
    variant = reader.read_number()

    if variant == 0:
        field = destack._generated.dir.symbol.key.decode_static_key(reader)

        return MemberTargetField(field=field)
    elif variant == 1:
        element = reader.read_number()

        return MemberTargetElement(element=element)
    elif variant == 2:
        index = destack._generated.dir.type.type.decode_global_type_id(reader)

        return MemberTargetIndex(index=index)
    elif variant == 3:
        symbol = decode_member_candidate(reader)

        return MemberTargetSymbol(symbol=symbol)
    elif variant == 4:
        existential = [
            decode_member_candidate(reader) for _ in range(reader.read_number())
        ]

        return MemberTargetExistential(existential=existential)
    elif variant == 5:
        universal = [
            decode_member_candidate(reader) for _ in range(reader.read_number())
        ]

        return MemberTargetUniversal(universal=universal)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_member_target(value: MemberTarget) -> Json:
    """Return one JSON value for one MemberTarget."""
    if value.kind == "field":
        return {
            "kind": "field",
            "field": destack._generated.dir.symbol.key.to_json_static_key(value.field),
        }
    elif value.kind == "element":
        return {
            "kind": "element",
            "element": value.element,
        }
    elif value.kind == "index":
        return {
            "kind": "index",
            "index": destack._generated.dir.type.type.to_json_global_type_id(
                value.index
            ),
        }
    elif value.kind == "symbol":
        return {
            "kind": "symbol",
            "symbol": to_json_member_candidate(value.symbol),
        }
    elif value.kind == "existential":
        return {
            "kind": "existential",
            "existential": [
                to_json_member_candidate(item_0) for item_0 in value.existential
            ],
        }
    elif value.kind == "universal":
        return {
            "kind": "universal",
            "universal": [
                to_json_member_candidate(item_0) for item_0 in value.universal
            ],
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_member_target(value: Json) -> MemberTarget:
    """Return one MemberTarget from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "field":
        return MemberTargetField(
            field=destack._generated.dir.symbol.key.from_json_static_key(
                json_field(object_, "field")
            )
        )
    elif kind == "element":
        return MemberTargetElement(element=json_int(json_field(object_, "element")))
    elif kind == "index":
        return MemberTargetIndex(
            index=destack._generated.dir.type.type.from_json_global_type_id(
                json_field(object_, "index")
            )
        )
    elif kind == "symbol":
        return MemberTargetSymbol(
            symbol=from_json_member_candidate(json_field(object_, "symbol"))
        )
    elif kind == "existential":
        return MemberTargetExistential(
            existential=[
                from_json_member_candidate(item_0)
                for item_0 in json_array(json_field(object_, "existential"))
            ]
        )
    elif kind == "universal":
        return MemberTargetUniversal(
            universal=[
                from_json_member_candidate(item_0)
                for item_0 in json_array(json_field(object_, "universal"))
            ]
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class MemberCandidate:
    """One member candidate after receiver lookup."""

    # the receiver type that selects this candidate
    receiver: destack._generated.dir.type.type.GlobalTypeId
    # the projection steps
    adjustments: Sequence[destack._generated.dir.type.projection.Projection]
    # the declaration that exposed this member
    owner: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the selected member symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the member type applied to the matched receiver
    ty: destack._generated.dir.type.type.GlobalTypeId
    # the selected generic argument bindings needed by this member candidate
    generic_arguments: Sequence[
        destack._generated.dir.type.generic.GenericArgumentBinding
    ]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_member_candidate(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> MemberCandidate:
        """Decode one MemberCandidate."""
        return decode_member_candidate(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_member_candidate(self)

    @classmethod
    def from_json(cls, value: Json) -> MemberCandidate:
        """Return one MemberCandidate from one JSON value."""
        return from_json_member_candidate(value)


def encode_member_candidate(writer: BinaryWriter, value: MemberCandidate) -> None:
    """Encode one MemberCandidate."""
    destack._generated.dir.type.type.encode_global_type_id(writer, value.receiver)
    writer.write_unsigned(len(value.adjustments))
    for item_value_adjustments_0 in value.adjustments:
        destack._generated.dir.type.projection.encode_projection(
            writer, item_value_adjustments_0
        )
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(writer, value.owner)
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(writer, value.symbol)
    destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)
    writer.write_unsigned(len(value.generic_arguments))
    for item_value_generic_arguments_0 in value.generic_arguments:
        destack._generated.dir.type.generic.encode_generic_argument_binding(
            writer, item_value_generic_arguments_0
        )


def decode_member_candidate(reader: BinaryReader) -> MemberCandidate:
    """Decode one MemberCandidate."""
    receiver = destack._generated.dir.type.type.decode_global_type_id(reader)
    adjustments = [
        destack._generated.dir.type.projection.decode_projection(reader)
        for _ in range(reader.read_number())
    ]
    owner = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    ty = destack._generated.dir.type.type.decode_global_type_id(reader)
    generic_arguments = [
        destack._generated.dir.type.generic.decode_generic_argument_binding(reader)
        for _ in range(reader.read_number())
    ]

    return MemberCandidate(
        receiver=receiver,
        adjustments=adjustments,
        owner=owner,
        symbol=symbol,
        ty=ty,
        generic_arguments=generic_arguments,
    )


def to_json_member_candidate(value: MemberCandidate) -> Json:
    """Return one JSON value for one MemberCandidate."""
    return {
        "receiver": destack._generated.dir.type.type.to_json_global_type_id(
            value.receiver
        ),
        "adjustments": [
            destack._generated.dir.type.projection.to_json_projection(item_0)
            for item_0 in value.adjustments
        ],
        "owner": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.owner
        ),
        "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.symbol
        ),
        "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty),
        "genericArguments": [
            destack._generated.dir.type.generic.to_json_generic_argument_binding(item_0)
            for item_0 in value.generic_arguments
        ],
    }


def from_json_member_candidate(value: Json) -> MemberCandidate:
    """Return one MemberCandidate from one JSON value."""
    object_ = json_object(value)

    return MemberCandidate(
        receiver=destack._generated.dir.type.type.from_json_global_type_id(
            json_field(object_, "receiver")
        ),
        adjustments=[
            destack._generated.dir.type.projection.from_json_projection(item_0)
            for item_0 in json_array(json_field(object_, "adjustments"))
        ],
        owner=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "owner")
        ),
        symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "symbol")
        ),
        ty=destack._generated.dir.type.type.from_json_global_type_id(
            json_field(object_, "ty")
        ),
        generic_arguments=[
            destack._generated.dir.type.generic.from_json_generic_argument_binding(
                item_0
            )
            for item_0 in json_array(json_field(object_, "genericArguments"))
        ],
    )


@dataclass(frozen=True, slots=True)
class CallResolution:
    """Callable selected at a call site."""

    # the selected callable target
    target: CallTarget
    # the callable type selected at the call site, when one exists
    callable_type: destack._generated.dir.type.type.GlobalTypeId | None
    # the dynamic parameter types after static substitutions
    parameters: Sequence[destack._generated.dir.type.type.GlobalTypeId]
    # the source arguments bound to selected parameters
    arguments: Sequence[destack._generated.dir.type.generic.ArgumentBinding]
    # the return type after static substitutions
    return_type: destack._generated.dir.type.type.GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_call_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CallResolution:
        """Decode one CallResolution."""
        return decode_call_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_call_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> CallResolution:
        """Return one CallResolution from one JSON value."""
        return from_json_call_resolution(value)


def encode_call_resolution(writer: BinaryWriter, value: CallResolution) -> None:
    """Encode one CallResolution."""
    encode_call_target(writer, value.target)
    if value.callable_type is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.type.type.encode_global_type_id(
            writer, value.callable_type
        )
    writer.write_unsigned(len(value.parameters))
    for item_value_parameters_0 in value.parameters:
        destack._generated.dir.type.type.encode_global_type_id(
            writer, item_value_parameters_0
        )
    writer.write_unsigned(len(value.arguments))
    for item_value_arguments_0 in value.arguments:
        destack._generated.dir.type.generic.encode_argument_binding(
            writer, item_value_arguments_0
        )
    destack._generated.dir.type.type.encode_global_type_id(writer, value.return_type)


def decode_call_resolution(reader: BinaryReader) -> CallResolution:
    """Decode one CallResolution."""
    target = decode_call_target(reader)
    callable_type = reader.read_option(
        lambda: destack._generated.dir.type.type.decode_global_type_id(reader)
    )
    parameters = [
        destack._generated.dir.type.type.decode_global_type_id(reader)
        for _ in range(reader.read_number())
    ]
    arguments = [
        destack._generated.dir.type.generic.decode_argument_binding(reader)
        for _ in range(reader.read_number())
    ]
    return_type = destack._generated.dir.type.type.decode_global_type_id(reader)

    return CallResolution(
        target=target,
        callable_type=callable_type,
        parameters=parameters,
        arguments=arguments,
        return_type=return_type,
    )


def to_json_call_resolution(value: CallResolution) -> Json:
    """Return one JSON value for one CallResolution."""
    return {
        "target": to_json_call_target(value.target),
        **(
            {}
            if value.callable_type is None
            else {
                "callableType": destack._generated.dir.type.type.to_json_global_type_id(
                    value.callable_type
                )
            }
        ),
        "parameters": [
            destack._generated.dir.type.type.to_json_global_type_id(item_0)
            for item_0 in value.parameters
        ],
        "arguments": [
            destack._generated.dir.type.generic.to_json_argument_binding(item_0)
            for item_0 in value.arguments
        ],
        "returnType": destack._generated.dir.type.type.to_json_global_type_id(
            value.return_type
        ),
    }


def from_json_call_resolution(value: Json) -> CallResolution:
    """Return one CallResolution from one JSON value."""
    object_ = json_object(value)

    return CallResolution(
        target=from_json_call_target(json_field(object_, "target")),
        callable_type=json_optional(
            object_,
            "callableType",
            lambda value: destack._generated.dir.type.type.from_json_global_type_id(
                value
            ),
        ),
        parameters=[
            destack._generated.dir.type.type.from_json_global_type_id(item_0)
            for item_0 in json_array(json_field(object_, "parameters"))
        ],
        arguments=[
            destack._generated.dir.type.generic.from_json_argument_binding(item_0)
            for item_0 in json_array(json_field(object_, "arguments"))
        ],
        return_type=destack._generated.dir.type.type.from_json_global_type_id(
            json_field(object_, "returnType")
        ),
    )


@dataclass(frozen=True, slots=True)
class CallTargetBuiltin:
    """Compiler builtin selected at a usage site."""

    builtin: BuiltinCall
    kind: typing.Literal["builtin"] = "builtin"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_call_target(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_call_target(self)


@dataclass(frozen=True, slots=True)
class CallTargetExpression:
    """Callable expression without a declaration symbol."""

    # the selected generic argument bindings, empty when not statically applied
    generic_arguments: Sequence[
        destack._generated.dir.type.generic.GenericArgumentBinding
    ]
    kind: typing.Literal["expression"] = "expression"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_call_target(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_call_target(self)


@dataclass(frozen=True, slots=True)
class CallTargetSymbol:
    """Exactly one symbol-backed callable selected at compile time."""

    symbol: CallCandidate
    kind: typing.Literal["symbol"] = "symbol"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_call_target(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_call_target(self)


@dataclass(frozen=True, slots=True)
class CallTargetUniversal:
    """Universal symbol-backed callables selected at compile time."""

    universal: Sequence[CallCandidate]
    kind: typing.Literal["universal"] = "universal"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_call_target(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_call_target(self)


"""Callable target selected at a call site."""
CallTarget: typing.TypeAlias = (
    CallTargetBuiltin | CallTargetExpression | CallTargetSymbol | CallTargetUniversal
)


def encode_call_target(writer: BinaryWriter, value: CallTarget) -> None:
    """Encode one CallTarget."""
    if value.kind == "builtin":
        writer.write_unsigned(0)
        encode_builtin_call(writer, value.builtin)
    elif value.kind == "expression":
        writer.write_unsigned(1)
        writer.write_unsigned(len(value.generic_arguments))
        for item_value_generic_arguments_0 in value.generic_arguments:
            destack._generated.dir.type.generic.encode_generic_argument_binding(
                writer, item_value_generic_arguments_0
            )
    elif value.kind == "symbol":
        writer.write_unsigned(2)
        encode_call_candidate(writer, value.symbol)
    elif value.kind == "universal":
        writer.write_unsigned(3)
        writer.write_unsigned(len(value.universal))
        for item_value_universal_0 in value.universal:
            encode_call_candidate(writer, item_value_universal_0)
    else:
        raise SerdeError("unknown enum variant")


def decode_call_target(reader: BinaryReader) -> CallTarget:
    """Decode one CallTarget."""
    variant = reader.read_number()

    if variant == 0:
        builtin = decode_builtin_call(reader)

        return CallTargetBuiltin(builtin=builtin)
    elif variant == 1:
        generic_arguments = [
            destack._generated.dir.type.generic.decode_generic_argument_binding(reader)
            for _ in range(reader.read_number())
        ]

        return CallTargetExpression(
            generic_arguments=generic_arguments,
        )
    elif variant == 2:
        symbol = decode_call_candidate(reader)

        return CallTargetSymbol(symbol=symbol)
    elif variant == 3:
        universal = [decode_call_candidate(reader) for _ in range(reader.read_number())]

        return CallTargetUniversal(universal=universal)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_call_target(value: CallTarget) -> Json:
    """Return one JSON value for one CallTarget."""
    if value.kind == "builtin":
        return {
            "kind": "builtin",
            "builtin": to_json_builtin_call(value.builtin),
        }
    elif value.kind == "expression":
        return {
            "kind": "expression",
            "genericArguments": [
                destack._generated.dir.type.generic.to_json_generic_argument_binding(
                    item_0
                )
                for item_0 in value.generic_arguments
            ],
        }
    elif value.kind == "symbol":
        return {
            "kind": "symbol",
            "symbol": to_json_call_candidate(value.symbol),
        }
    elif value.kind == "universal":
        return {
            "kind": "universal",
            "universal": [to_json_call_candidate(item_0) for item_0 in value.universal],
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_call_target(value: Json) -> CallTarget:
    """Return one CallTarget from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "builtin":
        return CallTargetBuiltin(
            builtin=from_json_builtin_call(json_field(object_, "builtin"))
        )
    elif kind == "expression":
        return CallTargetExpression(
            generic_arguments=[
                destack._generated.dir.type.generic.from_json_generic_argument_binding(
                    item_0
                )
                for item_0 in json_array(json_field(object_, "genericArguments"))
            ],
        )
    elif kind == "symbol":
        return CallTargetSymbol(
            symbol=from_json_call_candidate(json_field(object_, "symbol"))
        )
    elif kind == "universal":
        return CallTargetUniversal(
            universal=[
                from_json_call_candidate(item_0)
                for item_0 in json_array(json_field(object_, "universal"))
            ]
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class BuiltinCallUnaryOperator:
    """Builtin unary operator behavior."""

    # the source operator
    operator: destack._generated.dir.tree.operator.UnaryOperator
    kind: typing.Literal["unaryOperator"] = "unaryOperator"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_builtin_call(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_builtin_call(self)


@dataclass(frozen=True, slots=True)
class BuiltinCallBinaryOperator:
    """Builtin binary operator behavior."""

    # the source operator
    operator: destack._generated.dir.tree.operator.BinaryOperator
    kind: typing.Literal["binaryOperator"] = "binaryOperator"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_builtin_call(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_builtin_call(self)


"""Compiler builtin callable selected at a usage site."""
BuiltinCall: typing.TypeAlias = BuiltinCallUnaryOperator | BuiltinCallBinaryOperator


def encode_builtin_call(writer: BinaryWriter, value: BuiltinCall) -> None:
    """Encode one BuiltinCall."""
    if value.kind == "unaryOperator":
        writer.write_unsigned(0)
        destack._generated.dir.tree.operator.encode_unary_operator(
            writer, value.operator
        )
    elif value.kind == "binaryOperator":
        writer.write_unsigned(1)
        destack._generated.dir.tree.operator.encode_binary_operator(
            writer, value.operator
        )
    else:
        raise SerdeError("unknown enum variant")


def decode_builtin_call(reader: BinaryReader) -> BuiltinCall:
    """Decode one BuiltinCall."""
    variant = reader.read_number()

    if variant == 0:
        operator = destack._generated.dir.tree.operator.decode_unary_operator(reader)

        return BuiltinCallUnaryOperator(
            operator=operator,
        )
    elif variant == 1:
        operator = destack._generated.dir.tree.operator.decode_binary_operator(reader)

        return BuiltinCallBinaryOperator(
            operator=operator,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_builtin_call(value: BuiltinCall) -> Json:
    """Return one JSON value for one BuiltinCall."""
    if value.kind == "unaryOperator":
        return {
            "kind": "unaryOperator",
            "operator": destack._generated.dir.tree.operator.to_json_unary_operator(
                value.operator
            ),
        }
    elif value.kind == "binaryOperator":
        return {
            "kind": "binaryOperator",
            "operator": destack._generated.dir.tree.operator.to_json_binary_operator(
                value.operator
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_builtin_call(value: Json) -> BuiltinCall:
    """Return one BuiltinCall from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "unaryOperator":
        return BuiltinCallUnaryOperator(
            operator=destack._generated.dir.tree.operator.from_json_unary_operator(
                json_field(object_, "operator")
            ),
        )
    elif kind == "binaryOperator":
        return BuiltinCallBinaryOperator(
            operator=destack._generated.dir.tree.operator.from_json_binary_operator(
                json_field(object_, "operator")
            ),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class CallCandidate:
    """One callable candidate after overload selection."""

    # the receiver type that selects this candidate
    receiver: destack._generated.dir.type.type.GlobalTypeId | None
    # the projection steps
    adjustments: Sequence[destack._generated.dir.type.projection.Projection]
    # the selected callable symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the selected generic argument bindings needed by this call candidate
    generic_arguments: Sequence[
        destack._generated.dir.type.generic.GenericArgumentBinding
    ]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_call_candidate(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CallCandidate:
        """Decode one CallCandidate."""
        return decode_call_candidate(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_call_candidate(self)

    @classmethod
    def from_json(cls, value: Json) -> CallCandidate:
        """Return one CallCandidate from one JSON value."""
        return from_json_call_candidate(value)


def encode_call_candidate(writer: BinaryWriter, value: CallCandidate) -> None:
    """Encode one CallCandidate."""
    if value.receiver is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.type.type.encode_global_type_id(writer, value.receiver)
    writer.write_unsigned(len(value.adjustments))
    for item_value_adjustments_0 in value.adjustments:
        destack._generated.dir.type.projection.encode_projection(
            writer, item_value_adjustments_0
        )
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(writer, value.symbol)
    writer.write_unsigned(len(value.generic_arguments))
    for item_value_generic_arguments_0 in value.generic_arguments:
        destack._generated.dir.type.generic.encode_generic_argument_binding(
            writer, item_value_generic_arguments_0
        )


def decode_call_candidate(reader: BinaryReader) -> CallCandidate:
    """Decode one CallCandidate."""
    receiver = reader.read_option(
        lambda: destack._generated.dir.type.type.decode_global_type_id(reader)
    )
    adjustments = [
        destack._generated.dir.type.projection.decode_projection(reader)
        for _ in range(reader.read_number())
    ]
    symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    generic_arguments = [
        destack._generated.dir.type.generic.decode_generic_argument_binding(reader)
        for _ in range(reader.read_number())
    ]

    return CallCandidate(
        receiver=receiver,
        adjustments=adjustments,
        symbol=symbol,
        generic_arguments=generic_arguments,
    )


def to_json_call_candidate(value: CallCandidate) -> Json:
    """Return one JSON value for one CallCandidate."""
    return {
        **(
            {}
            if value.receiver is None
            else {
                "receiver": destack._generated.dir.type.type.to_json_global_type_id(
                    value.receiver
                )
            }
        ),
        "adjustments": [
            destack._generated.dir.type.projection.to_json_projection(item_0)
            for item_0 in value.adjustments
        ],
        "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.symbol
        ),
        "genericArguments": [
            destack._generated.dir.type.generic.to_json_generic_argument_binding(item_0)
            for item_0 in value.generic_arguments
        ],
    }


def from_json_call_candidate(value: Json) -> CallCandidate:
    """Return one CallCandidate from one JSON value."""
    object_ = json_object(value)

    return CallCandidate(
        receiver=json_optional(
            object_,
            "receiver",
            lambda value: destack._generated.dir.type.type.from_json_global_type_id(
                value
            ),
        ),
        adjustments=[
            destack._generated.dir.type.projection.from_json_projection(item_0)
            for item_0 in json_array(json_field(object_, "adjustments"))
        ],
        symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "symbol")
        ),
        generic_arguments=[
            destack._generated.dir.type.generic.from_json_generic_argument_binding(
                item_0
            )
            for item_0 in json_array(json_field(object_, "genericArguments"))
        ],
    )


@dataclass(frozen=True, slots=True)
class PlaceResolution:
    """Place selected by a checked expression."""

    # the expression node that designates the place
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the selected storage location
    storage: Storage
    # the value type stored in the place
    ty: destack._generated.dir.type.type.GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_place_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PlaceResolution:
        """Decode one PlaceResolution."""
        return decode_place_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_place_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> PlaceResolution:
        """Return one PlaceResolution from one JSON value."""
        return from_json_place_resolution(value)


def encode_place_resolution(writer: BinaryWriter, value: PlaceResolution) -> None:
    """Encode one PlaceResolution."""
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.source)
    encode_storage(writer, value.storage)
    destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)


def decode_place_resolution(reader: BinaryReader) -> PlaceResolution:
    """Decode one PlaceResolution."""
    source = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    storage = decode_storage(reader)
    ty = destack._generated.dir.type.type.decode_global_type_id(reader)

    return PlaceResolution(
        source=source,
        storage=storage,
        ty=ty,
    )


def to_json_place_resolution(value: PlaceResolution) -> Json:
    """Return one JSON value for one PlaceResolution."""
    return {
        "source": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.source
        ),
        "storage": to_json_storage(value.storage),
        "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty),
    }


def from_json_place_resolution(value: Json) -> PlaceResolution:
    """Return one PlaceResolution from one JSON value."""
    object_ = json_object(value)

    return PlaceResolution(
        source=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "source")
        ),
        storage=from_json_storage(json_field(object_, "storage")),
        ty=destack._generated.dir.type.type.from_json_global_type_id(
            json_field(object_, "ty")
        ),
    )


@dataclass(frozen=True, slots=True)
class StorageBinding:
    """Local or imported value binding."""

    # the selected binding symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    kind: typing.Literal["binding"] = "binding"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_storage(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_storage(self)


@dataclass(frozen=True, slots=True)
class StorageField:
    """Structural or nominal field storage."""

    # the receiver type
    receiver: destack._generated.dir.type.type.GlobalTypeId
    # the selected field
    field: destack._generated.dir.type.projection.ProjectionField
    kind: typing.Literal["field"] = "field"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_storage(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_storage(self)


@dataclass(frozen=True, slots=True)
class StorageProperty:
    """Accessor-backed property storage."""

    # the selected getter member, when the source operator reads first
    read: MemberResolution | None
    # the selected setter member
    write: MemberResolution
    kind: typing.Literal["property"] = "property"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_storage(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_storage(self)


@dataclass(frozen=True, slots=True)
class StorageSubscript:
    """Subscript-selected storage."""

    # the source node providing the subscript key
    index: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the selected subscript operation, when the source operator reads first
    read: destack._generated.dir.type.projection.SubscriptOperation | None
    # the selected write operation
    write: destack._generated.dir.type.projection.SubscriptOperation
    kind: typing.Literal["subscript"] = "subscript"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_storage(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_storage(self)


@dataclass(frozen=True, slots=True)
class StorageDereference:
    """Dereferenced storage."""

    # the selected dereference operation, when the source operator reads first
    read: destack._generated.dir.type.projection.DereferenceOperation | None
    # the selected write operation
    write: destack._generated.dir.type.projection.DereferenceOperation
    kind: typing.Literal["dereference"] = "dereference"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_storage(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_storage(self)


"""Writable storage location selected by a place expression."""
Storage: typing.TypeAlias = (
    StorageBinding
    | StorageField
    | StorageProperty
    | StorageSubscript
    | StorageDereference
)


def encode_storage(writer: BinaryWriter, value: Storage) -> None:
    """Encode one Storage."""
    if value.kind == "binding":
        writer.write_unsigned(0)
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(
            writer, value.symbol
        )
    elif value.kind == "field":
        writer.write_unsigned(1)
        destack._generated.dir.type.type.encode_global_type_id(writer, value.receiver)
        destack._generated.dir.type.projection.encode_projection_field(
            writer, value.field
        )
    elif value.kind == "property":
        writer.write_unsigned(2)
        if value.read is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            encode_member_resolution(writer, value.read)
        encode_member_resolution(writer, value.write)
    elif value.kind == "subscript":
        writer.write_unsigned(3)
        destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.index)
        if value.read is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.type.projection.encode_subscript_operation(
                writer, value.read
            )
        destack._generated.dir.type.projection.encode_subscript_operation(
            writer, value.write
        )
    elif value.kind == "dereference":
        writer.write_unsigned(4)
        if value.read is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.type.projection.encode_dereference_operation(
                writer, value.read
            )
        destack._generated.dir.type.projection.encode_dereference_operation(
            writer, value.write
        )
    else:
        raise SerdeError("unknown enum variant")


def decode_storage(reader: BinaryReader) -> Storage:
    """Decode one Storage."""
    variant = reader.read_number()

    if variant == 0:
        symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)

        return StorageBinding(
            symbol=symbol,
        )
    elif variant == 1:
        receiver = destack._generated.dir.type.type.decode_global_type_id(reader)
        field = destack._generated.dir.type.projection.decode_projection_field(reader)

        return StorageField(
            receiver=receiver,
            field=field,
        )
    elif variant == 2:
        read = reader.read_option(lambda: decode_member_resolution(reader))
        write = decode_member_resolution(reader)

        return StorageProperty(
            read=read,
            write=write,
        )
    elif variant == 3:
        index = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
        read = reader.read_option(
            lambda: destack._generated.dir.type.projection.decode_subscript_operation(
                reader
            )
        )
        write = destack._generated.dir.type.projection.decode_subscript_operation(
            reader
        )

        return StorageSubscript(
            index=index,
            read=read,
            write=write,
        )
    elif variant == 4:
        read = reader.read_option(
            lambda: destack._generated.dir.type.projection.decode_dereference_operation(
                reader
            )
        )
        write = destack._generated.dir.type.projection.decode_dereference_operation(
            reader
        )

        return StorageDereference(
            read=read,
            write=write,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_storage(value: Storage) -> Json:
    """Return one JSON value for one Storage."""
    if value.kind == "binding":
        return {
            "kind": "binding",
            "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
                value.symbol
            ),
        }
    elif value.kind == "field":
        return {
            "kind": "field",
            "receiver": destack._generated.dir.type.type.to_json_global_type_id(
                value.receiver
            ),
            "field": destack._generated.dir.type.projection.to_json_projection_field(
                value.field
            ),
        }
    elif value.kind == "property":
        return {
            "kind": "property",
            **(
                {}
                if value.read is None
                else {"read": to_json_member_resolution(value.read)}
            ),
            "write": to_json_member_resolution(value.write),
        }
    elif value.kind == "subscript":
        return {
            "kind": "subscript",
            "index": destack._generated.dir.tree.node.to_json_global_node_id_any(
                value.index
            ),
            **(
                {}
                if value.read is None
                else {
                    "read": destack._generated.dir.type.projection.to_json_subscript_operation(
                        value.read
                    )
                }
            ),
            "write": destack._generated.dir.type.projection.to_json_subscript_operation(
                value.write
            ),
        }
    elif value.kind == "dereference":
        return {
            "kind": "dereference",
            **(
                {}
                if value.read is None
                else {
                    "read": destack._generated.dir.type.projection.to_json_dereference_operation(
                        value.read
                    )
                }
            ),
            "write": destack._generated.dir.type.projection.to_json_dereference_operation(
                value.write
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_storage(value: Json) -> Storage:
    """Return one Storage from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "binding":
        return StorageBinding(
            symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
                json_field(object_, "symbol")
            ),
        )
    elif kind == "field":
        return StorageField(
            receiver=destack._generated.dir.type.type.from_json_global_type_id(
                json_field(object_, "receiver")
            ),
            field=destack._generated.dir.type.projection.from_json_projection_field(
                json_field(object_, "field")
            ),
        )
    elif kind == "property":
        return StorageProperty(
            read=json_optional(
                object_, "read", lambda value: from_json_member_resolution(value)
            ),
            write=from_json_member_resolution(json_field(object_, "write")),
        )
    elif kind == "subscript":
        return StorageSubscript(
            index=destack._generated.dir.tree.node.from_json_global_node_id_any(
                json_field(object_, "index")
            ),
            read=json_optional(
                object_,
                "read",
                lambda value: (
                    destack._generated.dir.type.projection.from_json_subscript_operation(
                        value
                    )
                ),
            ),
            write=destack._generated.dir.type.projection.from_json_subscript_operation(
                json_field(object_, "write")
            ),
        )
    elif kind == "dereference":
        return StorageDereference(
            read=json_optional(
                object_,
                "read",
                lambda value: (
                    destack._generated.dir.type.projection.from_json_dereference_operation(
                        value
                    )
                ),
            ),
            write=destack._generated.dir.type.projection.from_json_dereference_operation(
                json_field(object_, "write")
            ),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class GuardResolutionIs:
    """`is` guard, like `value is T`."""

    is_: IsGuardResolution
    kind: typing.Literal["is"] = "is"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_guard_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_guard_resolution(self)


@dataclass(frozen=True, slots=True)
class GuardResolutionInstanceOf:
    """`instanceof` guard, like `value instanceof User`."""

    instance_of: InstanceOfGuardResolution
    kind: typing.Literal["instanceOf"] = "instanceOf"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_guard_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_guard_resolution(self)


@dataclass(frozen=True, slots=True)
class GuardResolutionIn:
    """`in` guard, like `"name" in value`."""

    in_: InGuardResolution
    kind: typing.Literal["in"] = "in"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_guard_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_guard_resolution(self)


"""Guard expression selected during checking."""
GuardResolution: typing.TypeAlias = (
    GuardResolutionIs | GuardResolutionInstanceOf | GuardResolutionIn
)


def encode_guard_resolution(writer: BinaryWriter, value: GuardResolution) -> None:
    """Encode one GuardResolution."""
    if value.kind == "is":
        writer.write_unsigned(0)
        encode_is_guard_resolution(writer, value.is_)
    elif value.kind == "instanceOf":
        writer.write_unsigned(1)
        encode_instance_of_guard_resolution(writer, value.instance_of)
    elif value.kind == "in":
        writer.write_unsigned(2)
        encode_in_guard_resolution(writer, value.in_)
    else:
        raise SerdeError("unknown enum variant")


def decode_guard_resolution(reader: BinaryReader) -> GuardResolution:
    """Decode one GuardResolution."""
    variant = reader.read_number()

    if variant == 0:
        is_ = decode_is_guard_resolution(reader)

        return GuardResolutionIs(is_=is_)
    elif variant == 1:
        instance_of = decode_instance_of_guard_resolution(reader)

        return GuardResolutionInstanceOf(instance_of=instance_of)
    elif variant == 2:
        in_ = decode_in_guard_resolution(reader)

        return GuardResolutionIn(in_=in_)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_guard_resolution(value: GuardResolution) -> Json:
    """Return one JSON value for one GuardResolution."""
    if value.kind == "is":
        return {
            "kind": "is",
            "is": to_json_is_guard_resolution(value.is_),
        }
    elif value.kind == "instanceOf":
        return {
            "kind": "instanceOf",
            "instance_of": to_json_instance_of_guard_resolution(value.instance_of),
        }
    elif value.kind == "in":
        return {
            "kind": "in",
            "in": to_json_in_guard_resolution(value.in_),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_guard_resolution(value: Json) -> GuardResolution:
    """Return one GuardResolution from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "is":
        return GuardResolutionIs(
            is_=from_json_is_guard_resolution(json_field(object_, "is"))
        )
    elif kind == "instanceOf":
        return GuardResolutionInstanceOf(
            instance_of=from_json_instance_of_guard_resolution(
                json_field(object_, "instance_of")
            )
        )
    elif kind == "in":
        return GuardResolutionIn(
            in_=from_json_in_guard_resolution(json_field(object_, "in"))
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class IsGuardResolution:
    """`is` guard selected during checking."""

    # the tested value type
    value_type: destack._generated.dir.type.type.GlobalTypeId
    # the tested target type
    target_type: destack._generated.dir.type.type.GlobalTypeId
    # the executable predicate
    predicate: destack._generated.dir.type.predicate.Predicate

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_is_guard_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> IsGuardResolution:
        """Decode one IsGuardResolution."""
        return decode_is_guard_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_is_guard_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> IsGuardResolution:
        """Return one IsGuardResolution from one JSON value."""
        return from_json_is_guard_resolution(value)


def encode_is_guard_resolution(writer: BinaryWriter, value: IsGuardResolution) -> None:
    """Encode one IsGuardResolution."""
    destack._generated.dir.type.type.encode_global_type_id(writer, value.value_type)
    destack._generated.dir.type.type.encode_global_type_id(writer, value.target_type)
    destack._generated.dir.type.predicate.encode_predicate(writer, value.predicate)


def decode_is_guard_resolution(reader: BinaryReader) -> IsGuardResolution:
    """Decode one IsGuardResolution."""
    value_type = destack._generated.dir.type.type.decode_global_type_id(reader)
    target_type = destack._generated.dir.type.type.decode_global_type_id(reader)
    predicate = destack._generated.dir.type.predicate.decode_predicate(reader)

    return IsGuardResolution(
        value_type=value_type,
        target_type=target_type,
        predicate=predicate,
    )


def to_json_is_guard_resolution(value: IsGuardResolution) -> Json:
    """Return one JSON value for one IsGuardResolution."""
    return {
        "valueType": destack._generated.dir.type.type.to_json_global_type_id(
            value.value_type
        ),
        "targetType": destack._generated.dir.type.type.to_json_global_type_id(
            value.target_type
        ),
        "predicate": destack._generated.dir.type.predicate.to_json_predicate(
            value.predicate
        ),
    }


def from_json_is_guard_resolution(value: Json) -> IsGuardResolution:
    """Return one IsGuardResolution from one JSON value."""
    object_ = json_object(value)

    return IsGuardResolution(
        value_type=destack._generated.dir.type.type.from_json_global_type_id(
            json_field(object_, "valueType")
        ),
        target_type=destack._generated.dir.type.type.from_json_global_type_id(
            json_field(object_, "targetType")
        ),
        predicate=destack._generated.dir.type.predicate.from_json_predicate(
            json_field(object_, "predicate")
        ),
    )


@dataclass(frozen=True, slots=True)
class InstanceOfGuardResolution:
    """`instanceof` guard selected during checking."""

    # the tested value type
    value_type: destack._generated.dir.type.type.GlobalTypeId
    # the selected right-hand-side declaration
    target: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the selected instance type tested at runtime
    target_type: destack._generated.dir.type.type.GlobalTypeId
    # the executable predicate
    predicate: destack._generated.dir.type.predicate.Predicate

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_instance_of_guard_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> InstanceOfGuardResolution:
        """Decode one InstanceOfGuardResolution."""
        return decode_instance_of_guard_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_instance_of_guard_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> InstanceOfGuardResolution:
        """Return one InstanceOfGuardResolution from one JSON value."""
        return from_json_instance_of_guard_resolution(value)


def encode_instance_of_guard_resolution(
    writer: BinaryWriter, value: InstanceOfGuardResolution
) -> None:
    """Encode one InstanceOfGuardResolution."""
    destack._generated.dir.type.type.encode_global_type_id(writer, value.value_type)
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(writer, value.target)
    destack._generated.dir.type.type.encode_global_type_id(writer, value.target_type)
    destack._generated.dir.type.predicate.encode_predicate(writer, value.predicate)


def decode_instance_of_guard_resolution(
    reader: BinaryReader,
) -> InstanceOfGuardResolution:
    """Decode one InstanceOfGuardResolution."""
    value_type = destack._generated.dir.type.type.decode_global_type_id(reader)
    target = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    target_type = destack._generated.dir.type.type.decode_global_type_id(reader)
    predicate = destack._generated.dir.type.predicate.decode_predicate(reader)

    return InstanceOfGuardResolution(
        value_type=value_type,
        target=target,
        target_type=target_type,
        predicate=predicate,
    )


def to_json_instance_of_guard_resolution(value: InstanceOfGuardResolution) -> Json:
    """Return one JSON value for one InstanceOfGuardResolution."""
    return {
        "valueType": destack._generated.dir.type.type.to_json_global_type_id(
            value.value_type
        ),
        "target": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.target
        ),
        "targetType": destack._generated.dir.type.type.to_json_global_type_id(
            value.target_type
        ),
        "predicate": destack._generated.dir.type.predicate.to_json_predicate(
            value.predicate
        ),
    }


def from_json_instance_of_guard_resolution(value: Json) -> InstanceOfGuardResolution:
    """Return one InstanceOfGuardResolution from one JSON value."""
    object_ = json_object(value)

    return InstanceOfGuardResolution(
        value_type=destack._generated.dir.type.type.from_json_global_type_id(
            json_field(object_, "valueType")
        ),
        target=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "target")
        ),
        target_type=destack._generated.dir.type.type.from_json_global_type_id(
            json_field(object_, "targetType")
        ),
        predicate=destack._generated.dir.type.predicate.from_json_predicate(
            json_field(object_, "predicate")
        ),
    )


@dataclass(frozen=True, slots=True)
class InGuardResolution:
    """`in` guard selected during checking."""

    # the tested key type
    key_type: destack._generated.dir.type.type.GlobalTypeId
    # the tested receiver type
    receiver_type: destack._generated.dir.type.type.GlobalTypeId
    # the executable predicate
    predicate: destack._generated.dir.type.predicate.Predicate

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_in_guard_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> InGuardResolution:
        """Decode one InGuardResolution."""
        return decode_in_guard_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_in_guard_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> InGuardResolution:
        """Return one InGuardResolution from one JSON value."""
        return from_json_in_guard_resolution(value)


def encode_in_guard_resolution(writer: BinaryWriter, value: InGuardResolution) -> None:
    """Encode one InGuardResolution."""
    destack._generated.dir.type.type.encode_global_type_id(writer, value.key_type)
    destack._generated.dir.type.type.encode_global_type_id(writer, value.receiver_type)
    destack._generated.dir.type.predicate.encode_predicate(writer, value.predicate)


def decode_in_guard_resolution(reader: BinaryReader) -> InGuardResolution:
    """Decode one InGuardResolution."""
    key_type = destack._generated.dir.type.type.decode_global_type_id(reader)
    receiver_type = destack._generated.dir.type.type.decode_global_type_id(reader)
    predicate = destack._generated.dir.type.predicate.decode_predicate(reader)

    return InGuardResolution(
        key_type=key_type,
        receiver_type=receiver_type,
        predicate=predicate,
    )


def to_json_in_guard_resolution(value: InGuardResolution) -> Json:
    """Return one JSON value for one InGuardResolution."""
    return {
        "keyType": destack._generated.dir.type.type.to_json_global_type_id(
            value.key_type
        ),
        "receiverType": destack._generated.dir.type.type.to_json_global_type_id(
            value.receiver_type
        ),
        "predicate": destack._generated.dir.type.predicate.to_json_predicate(
            value.predicate
        ),
    }


def from_json_in_guard_resolution(value: Json) -> InGuardResolution:
    """Return one InGuardResolution from one JSON value."""
    object_ = json_object(value)

    return InGuardResolution(
        key_type=destack._generated.dir.type.type.from_json_global_type_id(
            json_field(object_, "keyType")
        ),
        receiver_type=destack._generated.dir.type.type.from_json_global_type_id(
            json_field(object_, "receiverType")
        ),
        predicate=destack._generated.dir.type.predicate.from_json_predicate(
            json_field(object_, "predicate")
        ),
    )


@dataclass(frozen=True, slots=True)
class ConstructResolution:
    """Construct expression selected at a usage site."""

    # the selected construct target
    target: ConstructTarget
    # the dynamic parameter types after static substitutions
    parameters: Sequence[destack._generated.dir.type.type.GlobalTypeId]
    # the source arguments bound to selected parameters
    arguments: Sequence[destack._generated.dir.type.generic.ArgumentBinding]
    # the return type after static substitutions
    return_type: destack._generated.dir.type.type.GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_construct_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ConstructResolution:
        """Decode one ConstructResolution."""
        return decode_construct_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_construct_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> ConstructResolution:
        """Return one ConstructResolution from one JSON value."""
        return from_json_construct_resolution(value)


def encode_construct_resolution(
    writer: BinaryWriter, value: ConstructResolution
) -> None:
    """Encode one ConstructResolution."""
    encode_construct_target(writer, value.target)
    writer.write_unsigned(len(value.parameters))
    for item_value_parameters_0 in value.parameters:
        destack._generated.dir.type.type.encode_global_type_id(
            writer, item_value_parameters_0
        )
    writer.write_unsigned(len(value.arguments))
    for item_value_arguments_0 in value.arguments:
        destack._generated.dir.type.generic.encode_argument_binding(
            writer, item_value_arguments_0
        )
    destack._generated.dir.type.type.encode_global_type_id(writer, value.return_type)


def decode_construct_resolution(reader: BinaryReader) -> ConstructResolution:
    """Decode one ConstructResolution."""
    target = decode_construct_target(reader)
    parameters = [
        destack._generated.dir.type.type.decode_global_type_id(reader)
        for _ in range(reader.read_number())
    ]
    arguments = [
        destack._generated.dir.type.generic.decode_argument_binding(reader)
        for _ in range(reader.read_number())
    ]
    return_type = destack._generated.dir.type.type.decode_global_type_id(reader)

    return ConstructResolution(
        target=target,
        parameters=parameters,
        arguments=arguments,
        return_type=return_type,
    )


def to_json_construct_resolution(value: ConstructResolution) -> Json:
    """Return one JSON value for one ConstructResolution."""
    return {
        "target": to_json_construct_target(value.target),
        "parameters": [
            destack._generated.dir.type.type.to_json_global_type_id(item_0)
            for item_0 in value.parameters
        ],
        "arguments": [
            destack._generated.dir.type.generic.to_json_argument_binding(item_0)
            for item_0 in value.arguments
        ],
        "returnType": destack._generated.dir.type.type.to_json_global_type_id(
            value.return_type
        ),
    }


def from_json_construct_resolution(value: Json) -> ConstructResolution:
    """Return one ConstructResolution from one JSON value."""
    object_ = json_object(value)

    return ConstructResolution(
        target=from_json_construct_target(json_field(object_, "target")),
        parameters=[
            destack._generated.dir.type.type.from_json_global_type_id(item_0)
            for item_0 in json_array(json_field(object_, "parameters"))
        ],
        arguments=[
            destack._generated.dir.type.generic.from_json_argument_binding(item_0)
            for item_0 in json_array(json_field(object_, "arguments"))
        ],
        return_type=destack._generated.dir.type.type.from_json_global_type_id(
            json_field(object_, "returnType")
        ),
    )


@dataclass(frozen=True, slots=True)
class ConstructTargetClass:
    """Class construction selected at compile time."""

    class_: ClassConstructCandidate
    kind: typing.Literal["class"] = "class"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_construct_target(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_construct_target(self)


@dataclass(frozen=True, slots=True)
class ConstructTargetNewtype:
    """Newtype wrapper constructor selected at compile time."""

    newtype: NewtypeConstructCandidate
    kind: typing.Literal["newtype"] = "newtype"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_construct_target(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_construct_target(self)


@dataclass(frozen=True, slots=True)
class ConstructTargetVariant:
    """Tagged union variant constructor selected at compile time."""

    variant: VariantConstructCandidate
    kind: typing.Literal["variant"] = "variant"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_construct_target(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_construct_target(self)


"""Construct target selected at a usage site."""
ConstructTarget: typing.TypeAlias = (
    ConstructTargetClass | ConstructTargetNewtype | ConstructTargetVariant
)


def encode_construct_target(writer: BinaryWriter, value: ConstructTarget) -> None:
    """Encode one ConstructTarget."""
    if value.kind == "class":
        writer.write_unsigned(0)
        encode_class_construct_candidate(writer, value.class_)
    elif value.kind == "newtype":
        writer.write_unsigned(1)
        encode_newtype_construct_candidate(writer, value.newtype)
    elif value.kind == "variant":
        writer.write_unsigned(2)
        encode_variant_construct_candidate(writer, value.variant)
    else:
        raise SerdeError("unknown enum variant")


def decode_construct_target(reader: BinaryReader) -> ConstructTarget:
    """Decode one ConstructTarget."""
    variant = reader.read_number()

    if variant == 0:
        class_ = decode_class_construct_candidate(reader)

        return ConstructTargetClass(class_=class_)
    elif variant == 1:
        newtype = decode_newtype_construct_candidate(reader)

        return ConstructTargetNewtype(newtype=newtype)
    elif variant == 2:
        variant = decode_variant_construct_candidate(reader)

        return ConstructTargetVariant(variant=variant)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_construct_target(value: ConstructTarget) -> Json:
    """Return one JSON value for one ConstructTarget."""
    if value.kind == "class":
        return {
            "kind": "class",
            "class": to_json_class_construct_candidate(value.class_),
        }
    elif value.kind == "newtype":
        return {
            "kind": "newtype",
            "newtype": to_json_newtype_construct_candidate(value.newtype),
        }
    elif value.kind == "variant":
        return {
            "kind": "variant",
            "variant": to_json_variant_construct_candidate(value.variant),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_construct_target(value: Json) -> ConstructTarget:
    """Return one ConstructTarget from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "class":
        return ConstructTargetClass(
            class_=from_json_class_construct_candidate(json_field(object_, "class"))
        )
    elif kind == "newtype":
        return ConstructTargetNewtype(
            newtype=from_json_newtype_construct_candidate(
                json_field(object_, "newtype")
            )
        )
    elif kind == "variant":
        return ConstructTargetVariant(
            variant=from_json_variant_construct_candidate(
                json_field(object_, "variant")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class ClassConstructCandidate:
    """One class construction candidate after overload selection."""

    # the selected class symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the selected class constructor
    constructor: destack._generated.dir.table.definition.ClassConstructor
    # the selected generic argument bindings for the class symbol
    generic_arguments: Sequence[
        destack._generated.dir.type.generic.GenericArgumentBinding
    ]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_class_construct_candidate(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ClassConstructCandidate:
        """Decode one ClassConstructCandidate."""
        return decode_class_construct_candidate(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_class_construct_candidate(self)

    @classmethod
    def from_json(cls, value: Json) -> ClassConstructCandidate:
        """Return one ClassConstructCandidate from one JSON value."""
        return from_json_class_construct_candidate(value)


def encode_class_construct_candidate(
    writer: BinaryWriter, value: ClassConstructCandidate
) -> None:
    """Encode one ClassConstructCandidate."""
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(writer, value.symbol)
    destack._generated.dir.table.definition.encode_class_constructor(
        writer, value.constructor
    )
    writer.write_unsigned(len(value.generic_arguments))
    for item_value_generic_arguments_0 in value.generic_arguments:
        destack._generated.dir.type.generic.encode_generic_argument_binding(
            writer, item_value_generic_arguments_0
        )


def decode_class_construct_candidate(reader: BinaryReader) -> ClassConstructCandidate:
    """Decode one ClassConstructCandidate."""
    symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    constructor = destack._generated.dir.table.definition.decode_class_constructor(
        reader
    )
    generic_arguments = [
        destack._generated.dir.type.generic.decode_generic_argument_binding(reader)
        for _ in range(reader.read_number())
    ]

    return ClassConstructCandidate(
        symbol=symbol,
        constructor=constructor,
        generic_arguments=generic_arguments,
    )


def to_json_class_construct_candidate(value: ClassConstructCandidate) -> Json:
    """Return one JSON value for one ClassConstructCandidate."""
    return {
        "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.symbol
        ),
        "constructor": destack._generated.dir.table.definition.to_json_class_constructor(
            value.constructor
        ),
        "genericArguments": [
            destack._generated.dir.type.generic.to_json_generic_argument_binding(item_0)
            for item_0 in value.generic_arguments
        ],
    }


def from_json_class_construct_candidate(value: Json) -> ClassConstructCandidate:
    """Return one ClassConstructCandidate from one JSON value."""
    object_ = json_object(value)

    return ClassConstructCandidate(
        symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "symbol")
        ),
        constructor=destack._generated.dir.table.definition.from_json_class_constructor(
            json_field(object_, "constructor")
        ),
        generic_arguments=[
            destack._generated.dir.type.generic.from_json_generic_argument_binding(
                item_0
            )
            for item_0 in json_array(json_field(object_, "genericArguments"))
        ],
    )


@dataclass(frozen=True, slots=True)
class NewtypeConstructCandidate:
    """One newtype construction candidate after overload selection."""

    # the selected newtype symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the selected generic argument bindings for the newtype symbol
    generic_arguments: Sequence[
        destack._generated.dir.type.generic.GenericArgumentBinding
    ]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_newtype_construct_candidate(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> NewtypeConstructCandidate:
        """Decode one NewtypeConstructCandidate."""
        return decode_newtype_construct_candidate(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_newtype_construct_candidate(self)

    @classmethod
    def from_json(cls, value: Json) -> NewtypeConstructCandidate:
        """Return one NewtypeConstructCandidate from one JSON value."""
        return from_json_newtype_construct_candidate(value)


def encode_newtype_construct_candidate(
    writer: BinaryWriter, value: NewtypeConstructCandidate
) -> None:
    """Encode one NewtypeConstructCandidate."""
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(writer, value.symbol)
    writer.write_unsigned(len(value.generic_arguments))
    for item_value_generic_arguments_0 in value.generic_arguments:
        destack._generated.dir.type.generic.encode_generic_argument_binding(
            writer, item_value_generic_arguments_0
        )


def decode_newtype_construct_candidate(
    reader: BinaryReader,
) -> NewtypeConstructCandidate:
    """Decode one NewtypeConstructCandidate."""
    symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    generic_arguments = [
        destack._generated.dir.type.generic.decode_generic_argument_binding(reader)
        for _ in range(reader.read_number())
    ]

    return NewtypeConstructCandidate(
        symbol=symbol,
        generic_arguments=generic_arguments,
    )


def to_json_newtype_construct_candidate(value: NewtypeConstructCandidate) -> Json:
    """Return one JSON value for one NewtypeConstructCandidate."""
    return {
        "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.symbol
        ),
        "genericArguments": [
            destack._generated.dir.type.generic.to_json_generic_argument_binding(item_0)
            for item_0 in value.generic_arguments
        ],
    }


def from_json_newtype_construct_candidate(value: Json) -> NewtypeConstructCandidate:
    """Return one NewtypeConstructCandidate from one JSON value."""
    object_ = json_object(value)

    return NewtypeConstructCandidate(
        symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "symbol")
        ),
        generic_arguments=[
            destack._generated.dir.type.generic.from_json_generic_argument_binding(
                item_0
            )
            for item_0 in json_array(json_field(object_, "genericArguments"))
        ],
    )


@dataclass(frozen=True, slots=True)
class VariantConstructCandidate:
    """One tagged variant construction candidate after checking."""

    # the selected tagged case
    case: destack._generated.dir.type.projection.VariantCase
    # the selected generic argument bindings for the owner symbol
    generic_arguments: Sequence[
        destack._generated.dir.type.generic.GenericArgumentBinding
    ]
    # the discriminant value injected by the constructor
    discriminant: destack._generated.dir.tree.literal.ScalarLiteral

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_variant_construct_candidate(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> VariantConstructCandidate:
        """Decode one VariantConstructCandidate."""
        return decode_variant_construct_candidate(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_variant_construct_candidate(self)

    @classmethod
    def from_json(cls, value: Json) -> VariantConstructCandidate:
        """Return one VariantConstructCandidate from one JSON value."""
        return from_json_variant_construct_candidate(value)


def encode_variant_construct_candidate(
    writer: BinaryWriter, value: VariantConstructCandidate
) -> None:
    """Encode one VariantConstructCandidate."""
    destack._generated.dir.type.projection.encode_variant_case(writer, value.case)
    writer.write_unsigned(len(value.generic_arguments))
    for item_value_generic_arguments_0 in value.generic_arguments:
        destack._generated.dir.type.generic.encode_generic_argument_binding(
            writer, item_value_generic_arguments_0
        )
    destack._generated.dir.tree.literal.encode_scalar_literal(
        writer, value.discriminant
    )


def decode_variant_construct_candidate(
    reader: BinaryReader,
) -> VariantConstructCandidate:
    """Decode one VariantConstructCandidate."""
    case = destack._generated.dir.type.projection.decode_variant_case(reader)
    generic_arguments = [
        destack._generated.dir.type.generic.decode_generic_argument_binding(reader)
        for _ in range(reader.read_number())
    ]
    discriminant = destack._generated.dir.tree.literal.decode_scalar_literal(reader)

    return VariantConstructCandidate(
        case=case,
        generic_arguments=generic_arguments,
        discriminant=discriminant,
    )


def to_json_variant_construct_candidate(value: VariantConstructCandidate) -> Json:
    """Return one JSON value for one VariantConstructCandidate."""
    return {
        "case": destack._generated.dir.type.projection.to_json_variant_case(value.case),
        "genericArguments": [
            destack._generated.dir.type.generic.to_json_generic_argument_binding(item_0)
            for item_0 in value.generic_arguments
        ],
        "discriminant": destack._generated.dir.tree.literal.to_json_scalar_literal(
            value.discriminant
        ),
    }


def from_json_variant_construct_candidate(value: Json) -> VariantConstructCandidate:
    """Return one VariantConstructCandidate from one JSON value."""
    object_ = json_object(value)

    return VariantConstructCandidate(
        case=destack._generated.dir.type.projection.from_json_variant_case(
            json_field(object_, "case")
        ),
        generic_arguments=[
            destack._generated.dir.type.generic.from_json_generic_argument_binding(
                item_0
            )
            for item_0 in json_array(json_field(object_, "genericArguments"))
        ],
        discriminant=destack._generated.dir.tree.literal.from_json_scalar_literal(
            json_field(object_, "discriminant")
        ),
    )


@dataclass(frozen=True, slots=True)
class PatternResolutionIgnore:
    """Pattern that accepts the input without binding, like `_`."""

    kind: typing.Literal["ignore"] = "ignore"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_resolution(self)


@dataclass(frozen=True, slots=True)
class PatternResolutionBind:
    """Pattern that binds a symbol, like `value`."""

    bind: PatternBindingResolution
    kind: typing.Literal["bind"] = "bind"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_resolution(self)


@dataclass(frozen=True, slots=True)
class PatternResolutionMust:
    """Pattern that requires a successful nested match, like `value!`."""

    must: PatternMustResolution
    kind: typing.Literal["must"] = "must"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_resolution(self)


@dataclass(frozen=True, slots=True)
class PatternResolutionDefault:
    """Pattern that uses a default value when the selected value is undefined."""

    default: PatternDefaultResolution
    kind: typing.Literal["default"] = "default"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_resolution(self)


@dataclass(frozen=True, slots=True)
class PatternResolutionTest:
    """Pattern that tests one executable predicate."""

    test: PatternPredicateResolution
    kind: typing.Literal["test"] = "test"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_resolution(self)


@dataclass(frozen=True, slots=True)
class PatternResolutionProject:
    """Pattern that projects the input before matching, like `*Point { x, y }`."""

    project: PatternProjectionResolution
    kind: typing.Literal["project"] = "project"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_resolution(self)


@dataclass(frozen=True, slots=True)
class PatternResolutionDestructure:
    """Pattern that destructures projected child values."""

    destructure: PatternDestructureResolution
    kind: typing.Literal["destructure"] = "destructure"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_resolution(self)


@dataclass(frozen=True, slots=True)
class PatternResolutionOr:
    """Pattern that accepts one of several branches, like `0 | 1 | 2`."""

    or_: PatternOrResolution
    kind: typing.Literal["or"] = "or"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_resolution(self)


"""Pattern meaning selected during checking."""
PatternResolution: typing.TypeAlias = (
    PatternResolutionIgnore
    | PatternResolutionBind
    | PatternResolutionMust
    | PatternResolutionDefault
    | PatternResolutionTest
    | PatternResolutionProject
    | PatternResolutionDestructure
    | PatternResolutionOr
)


def encode_pattern_resolution(writer: BinaryWriter, value: PatternResolution) -> None:
    """Encode one PatternResolution."""
    if value.kind == "ignore":
        writer.write_unsigned(0)
    elif value.kind == "bind":
        writer.write_unsigned(1)
        encode_pattern_binding_resolution(writer, value.bind)
    elif value.kind == "must":
        writer.write_unsigned(2)
        encode_pattern_must_resolution(writer, value.must)
    elif value.kind == "default":
        writer.write_unsigned(3)
        encode_pattern_default_resolution(writer, value.default)
    elif value.kind == "test":
        writer.write_unsigned(4)
        encode_pattern_predicate_resolution(writer, value.test)
    elif value.kind == "project":
        writer.write_unsigned(5)
        encode_pattern_projection_resolution(writer, value.project)
    elif value.kind == "destructure":
        writer.write_unsigned(6)
        encode_pattern_destructure_resolution(writer, value.destructure)
    elif value.kind == "or":
        writer.write_unsigned(7)
        encode_pattern_or_resolution(writer, value.or_)
    else:
        raise SerdeError("unknown enum variant")


def decode_pattern_resolution(reader: BinaryReader) -> PatternResolution:
    """Decode one PatternResolution."""
    variant = reader.read_number()

    if variant == 0:
        return PatternResolutionIgnore()
    elif variant == 1:
        bind = decode_pattern_binding_resolution(reader)

        return PatternResolutionBind(bind=bind)
    elif variant == 2:
        must = decode_pattern_must_resolution(reader)

        return PatternResolutionMust(must=must)
    elif variant == 3:
        default = decode_pattern_default_resolution(reader)

        return PatternResolutionDefault(default=default)
    elif variant == 4:
        test = decode_pattern_predicate_resolution(reader)

        return PatternResolutionTest(test=test)
    elif variant == 5:
        project = decode_pattern_projection_resolution(reader)

        return PatternResolutionProject(project=project)
    elif variant == 6:
        destructure = decode_pattern_destructure_resolution(reader)

        return PatternResolutionDestructure(destructure=destructure)
    elif variant == 7:
        or_ = decode_pattern_or_resolution(reader)

        return PatternResolutionOr(or_=or_)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_pattern_resolution(value: PatternResolution) -> Json:
    """Return one JSON value for one PatternResolution."""
    if value.kind == "ignore":
        return {
            "kind": "ignore",
        }
    elif value.kind == "bind":
        return {
            "kind": "bind",
            "bind": to_json_pattern_binding_resolution(value.bind),
        }
    elif value.kind == "must":
        return {
            "kind": "must",
            "must": to_json_pattern_must_resolution(value.must),
        }
    elif value.kind == "default":
        return {
            "kind": "default",
            "default": to_json_pattern_default_resolution(value.default),
        }
    elif value.kind == "test":
        return {
            "kind": "test",
            "test": to_json_pattern_predicate_resolution(value.test),
        }
    elif value.kind == "project":
        return {
            "kind": "project",
            "project": to_json_pattern_projection_resolution(value.project),
        }
    elif value.kind == "destructure":
        return {
            "kind": "destructure",
            "destructure": to_json_pattern_destructure_resolution(value.destructure),
        }
    elif value.kind == "or":
        return {
            "kind": "or",
            "or": to_json_pattern_or_resolution(value.or_),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_pattern_resolution(value: Json) -> PatternResolution:
    """Return one PatternResolution from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "ignore":
        return PatternResolutionIgnore()
    elif kind == "bind":
        return PatternResolutionBind(
            bind=from_json_pattern_binding_resolution(json_field(object_, "bind"))
        )
    elif kind == "must":
        return PatternResolutionMust(
            must=from_json_pattern_must_resolution(json_field(object_, "must"))
        )
    elif kind == "default":
        return PatternResolutionDefault(
            default=from_json_pattern_default_resolution(json_field(object_, "default"))
        )
    elif kind == "test":
        return PatternResolutionTest(
            test=from_json_pattern_predicate_resolution(json_field(object_, "test"))
        )
    elif kind == "project":
        return PatternResolutionProject(
            project=from_json_pattern_projection_resolution(
                json_field(object_, "project")
            )
        )
    elif kind == "destructure":
        return PatternResolutionDestructure(
            destructure=from_json_pattern_destructure_resolution(
                json_field(object_, "destructure")
            )
        )
    elif kind == "or":
        return PatternResolutionOr(
            or_=from_json_pattern_or_resolution(json_field(object_, "or"))
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class PatternBindingResolution:
    """Symbol binding introduced by one pattern."""

    # the bound symbol, when the binding has a user-visible name
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId | None
    # the nested pattern matched after binding
    pattern: destack._generated.dir.tree.node.GlobalNodeIdAny | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_binding_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternBindingResolution:
        """Decode one PatternBindingResolution."""
        return decode_pattern_binding_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_binding_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> PatternBindingResolution:
        """Return one PatternBindingResolution from one JSON value."""
        return from_json_pattern_binding_resolution(value)


def encode_pattern_binding_resolution(
    writer: BinaryWriter, value: PatternBindingResolution
) -> None:
    """Encode one PatternBindingResolution."""
    if value.symbol is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(
            writer, value.symbol
        )
    if value.pattern is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.node.encode_global_node_id_any(
            writer, value.pattern
        )


def decode_pattern_binding_resolution(reader: BinaryReader) -> PatternBindingResolution:
    """Decode one PatternBindingResolution."""
    symbol = reader.read_option(
        lambda: destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    )
    pattern = reader.read_option(
        lambda: destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    )

    return PatternBindingResolution(
        symbol=symbol,
        pattern=pattern,
    )


def to_json_pattern_binding_resolution(value: PatternBindingResolution) -> Json:
    """Return one JSON value for one PatternBindingResolution."""
    return {
        **(
            {}
            if value.symbol is None
            else {
                "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
                    value.symbol
                )
            }
        ),
        **(
            {}
            if value.pattern is None
            else {
                "pattern": destack._generated.dir.tree.node.to_json_global_node_id_any(
                    value.pattern
                )
            }
        ),
    }


def from_json_pattern_binding_resolution(value: Json) -> PatternBindingResolution:
    """Return one PatternBindingResolution from one JSON value."""
    object_ = json_object(value)

    return PatternBindingResolution(
        symbol=json_optional(
            object_,
            "symbol",
            lambda value: (
                destack._generated.dir.symbol.symbol.from_json_global_symbol_id(value)
            ),
        ),
        pattern=json_optional(
            object_,
            "pattern",
            lambda value: destack._generated.dir.tree.node.from_json_global_node_id_any(
                value
            ),
        ),
    )


@dataclass(frozen=True, slots=True)
class PatternMustResolution:
    """Required nested pattern selected during checking."""

    # the nested pattern that must match
    pattern: destack._generated.dir.tree.node.GlobalNodeIdAny

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_must_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternMustResolution:
        """Decode one PatternMustResolution."""
        return decode_pattern_must_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_must_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> PatternMustResolution:
        """Return one PatternMustResolution from one JSON value."""
        return from_json_pattern_must_resolution(value)


def encode_pattern_must_resolution(
    writer: BinaryWriter, value: PatternMustResolution
) -> None:
    """Encode one PatternMustResolution."""
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.pattern)


def decode_pattern_must_resolution(reader: BinaryReader) -> PatternMustResolution:
    """Decode one PatternMustResolution."""
    pattern = destack._generated.dir.tree.node.decode_global_node_id_any(reader)

    return PatternMustResolution(
        pattern=pattern,
    )


def to_json_pattern_must_resolution(value: PatternMustResolution) -> Json:
    """Return one JSON value for one PatternMustResolution."""
    return {
        "pattern": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.pattern
        ),
    }


def from_json_pattern_must_resolution(value: Json) -> PatternMustResolution:
    """Return one PatternMustResolution from one JSON value."""
    object_ = json_object(value)

    return PatternMustResolution(
        pattern=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "pattern")
        ),
    )


@dataclass(frozen=True, slots=True)
class PatternDefaultResolution:
    """Defaulted nested pattern selected during checking."""

    # the nested pattern
    pattern: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the default expression
    value: destack._generated.dir.tree.node.GlobalNodeIdAny

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_default_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternDefaultResolution:
        """Decode one PatternDefaultResolution."""
        return decode_pattern_default_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_default_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> PatternDefaultResolution:
        """Return one PatternDefaultResolution from one JSON value."""
        return from_json_pattern_default_resolution(value)


def encode_pattern_default_resolution(
    writer: BinaryWriter, value: PatternDefaultResolution
) -> None:
    """Encode one PatternDefaultResolution."""
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.pattern)
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.value)


def decode_pattern_default_resolution(reader: BinaryReader) -> PatternDefaultResolution:
    """Decode one PatternDefaultResolution."""
    pattern = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    value_ = destack._generated.dir.tree.node.decode_global_node_id_any(reader)

    return PatternDefaultResolution(
        pattern=pattern,
        value=value_,
    )


def to_json_pattern_default_resolution(value: PatternDefaultResolution) -> Json:
    """Return one JSON value for one PatternDefaultResolution."""
    return {
        "pattern": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.pattern
        ),
        "value": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.value
        ),
    }


def from_json_pattern_default_resolution(value: Json) -> PatternDefaultResolution:
    """Return one PatternDefaultResolution from one JSON value."""
    object_ = json_object(value)

    return PatternDefaultResolution(
        pattern=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "pattern")
        ),
        value=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "value")
        ),
    )


@dataclass(frozen=True, slots=True)
class PatternPredicateResolution:
    """Executable predicate selected by one pattern."""

    # the executable predicate
    predicate: destack._generated.dir.type.predicate.Predicate

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_predicate_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternPredicateResolution:
        """Decode one PatternPredicateResolution."""
        return decode_pattern_predicate_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_predicate_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> PatternPredicateResolution:
        """Return one PatternPredicateResolution from one JSON value."""
        return from_json_pattern_predicate_resolution(value)


def encode_pattern_predicate_resolution(
    writer: BinaryWriter, value: PatternPredicateResolution
) -> None:
    """Encode one PatternPredicateResolution."""
    destack._generated.dir.type.predicate.encode_predicate(writer, value.predicate)


def decode_pattern_predicate_resolution(
    reader: BinaryReader,
) -> PatternPredicateResolution:
    """Decode one PatternPredicateResolution."""
    predicate = destack._generated.dir.type.predicate.decode_predicate(reader)

    return PatternPredicateResolution(
        predicate=predicate,
    )


def to_json_pattern_predicate_resolution(value: PatternPredicateResolution) -> Json:
    """Return one JSON value for one PatternPredicateResolution."""
    return {
        "predicate": destack._generated.dir.type.predicate.to_json_predicate(
            value.predicate
        ),
    }


def from_json_pattern_predicate_resolution(value: Json) -> PatternPredicateResolution:
    """Return one PatternPredicateResolution from one JSON value."""
    object_ = json_object(value)

    return PatternPredicateResolution(
        predicate=destack._generated.dir.type.predicate.from_json_predicate(
            json_field(object_, "predicate")
        ),
    )


@dataclass(frozen=True, slots=True)
class PatternProjectionResolution:
    """Projection selected by one pattern."""

    # the selected projection
    projection: destack._generated.dir.type.projection.Projection
    # the pattern matched after projection
    pattern: destack._generated.dir.tree.node.GlobalNodeIdAny | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_projection_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternProjectionResolution:
        """Decode one PatternProjectionResolution."""
        return decode_pattern_projection_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_projection_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> PatternProjectionResolution:
        """Return one PatternProjectionResolution from one JSON value."""
        return from_json_pattern_projection_resolution(value)


def encode_pattern_projection_resolution(
    writer: BinaryWriter, value: PatternProjectionResolution
) -> None:
    """Encode one PatternProjectionResolution."""
    destack._generated.dir.type.projection.encode_projection(writer, value.projection)
    if value.pattern is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.node.encode_global_node_id_any(
            writer, value.pattern
        )


def decode_pattern_projection_resolution(
    reader: BinaryReader,
) -> PatternProjectionResolution:
    """Decode one PatternProjectionResolution."""
    projection = destack._generated.dir.type.projection.decode_projection(reader)
    pattern = reader.read_option(
        lambda: destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    )

    return PatternProjectionResolution(
        projection=projection,
        pattern=pattern,
    )


def to_json_pattern_projection_resolution(value: PatternProjectionResolution) -> Json:
    """Return one JSON value for one PatternProjectionResolution."""
    return {
        "projection": destack._generated.dir.type.projection.to_json_projection(
            value.projection
        ),
        **(
            {}
            if value.pattern is None
            else {
                "pattern": destack._generated.dir.tree.node.to_json_global_node_id_any(
                    value.pattern
                )
            }
        ),
    }


def from_json_pattern_projection_resolution(value: Json) -> PatternProjectionResolution:
    """Return one PatternProjectionResolution from one JSON value."""
    object_ = json_object(value)

    return PatternProjectionResolution(
        projection=destack._generated.dir.type.projection.from_json_projection(
            json_field(object_, "projection")
        ),
        pattern=json_optional(
            object_,
            "pattern",
            lambda value: destack._generated.dir.tree.node.from_json_global_node_id_any(
                value
            ),
        ),
    )


@dataclass(frozen=True, slots=True)
class PatternDestructureResolutionTuple:
    """Tuple-shaped destructuring, like `(x, y)`."""

    tuple: PatternTupleDestructureResolution
    kind: typing.Literal["tuple"] = "tuple"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_destructure_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_destructure_resolution(self)


@dataclass(frozen=True, slots=True)
class PatternDestructureResolutionObject:
    """Object-shaped destructuring, like `{ name }`."""

    object: PatternObjectDestructureResolution
    kind: typing.Literal["object"] = "object"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_destructure_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_destructure_resolution(self)


@dataclass(frozen=True, slots=True)
class PatternDestructureResolutionNominal:
    """Symbol-backed nominal destructuring, like `Point { x, y }`."""

    nominal: PatternNominalDestructureResolution
    kind: typing.Literal["nominal"] = "nominal"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_destructure_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_destructure_resolution(self)


@dataclass(frozen=True, slots=True)
class PatternDestructureResolutionSequence:
    """Sequence destructuring, like `[head, ...tail]`."""

    sequence: PatternSequenceDestructureResolution
    kind: typing.Literal["sequence"] = "sequence"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_destructure_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_destructure_resolution(self)


@dataclass(frozen=True, slots=True)
class PatternDestructureResolutionVariant:
    """Tagged variant destructuring, like `Status.Ok(value)`."""

    variant: PatternVariantDestructureResolution
    kind: typing.Literal["variant"] = "variant"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_destructure_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_destructure_resolution(self)


"""Destructuring selected by one pattern."""
PatternDestructureResolution: typing.TypeAlias = (
    PatternDestructureResolutionTuple
    | PatternDestructureResolutionObject
    | PatternDestructureResolutionNominal
    | PatternDestructureResolutionSequence
    | PatternDestructureResolutionVariant
)


def encode_pattern_destructure_resolution(
    writer: BinaryWriter, value: PatternDestructureResolution
) -> None:
    """Encode one PatternDestructureResolution."""
    if value.kind == "tuple":
        writer.write_unsigned(0)
        encode_pattern_tuple_destructure_resolution(writer, value.tuple)
    elif value.kind == "object":
        writer.write_unsigned(1)
        encode_pattern_object_destructure_resolution(writer, value.object)
    elif value.kind == "nominal":
        writer.write_unsigned(2)
        encode_pattern_nominal_destructure_resolution(writer, value.nominal)
    elif value.kind == "sequence":
        writer.write_unsigned(3)
        encode_pattern_sequence_destructure_resolution(writer, value.sequence)
    elif value.kind == "variant":
        writer.write_unsigned(4)
        encode_pattern_variant_destructure_resolution(writer, value.variant)
    else:
        raise SerdeError("unknown enum variant")


def decode_pattern_destructure_resolution(
    reader: BinaryReader,
) -> PatternDestructureResolution:
    """Decode one PatternDestructureResolution."""
    variant = reader.read_number()

    if variant == 0:
        tuple = decode_pattern_tuple_destructure_resolution(reader)

        return PatternDestructureResolutionTuple(tuple=tuple)
    elif variant == 1:
        object = decode_pattern_object_destructure_resolution(reader)

        return PatternDestructureResolutionObject(object=object)
    elif variant == 2:
        nominal = decode_pattern_nominal_destructure_resolution(reader)

        return PatternDestructureResolutionNominal(nominal=nominal)
    elif variant == 3:
        sequence = decode_pattern_sequence_destructure_resolution(reader)

        return PatternDestructureResolutionSequence(sequence=sequence)
    elif variant == 4:
        variant = decode_pattern_variant_destructure_resolution(reader)

        return PatternDestructureResolutionVariant(variant=variant)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_pattern_destructure_resolution(value: PatternDestructureResolution) -> Json:
    """Return one JSON value for one PatternDestructureResolution."""
    if value.kind == "tuple":
        return {
            "kind": "tuple",
            "tuple": to_json_pattern_tuple_destructure_resolution(value.tuple),
        }
    elif value.kind == "object":
        return {
            "kind": "object",
            "object": to_json_pattern_object_destructure_resolution(value.object),
        }
    elif value.kind == "nominal":
        return {
            "kind": "nominal",
            "nominal": to_json_pattern_nominal_destructure_resolution(value.nominal),
        }
    elif value.kind == "sequence":
        return {
            "kind": "sequence",
            "sequence": to_json_pattern_sequence_destructure_resolution(value.sequence),
        }
    elif value.kind == "variant":
        return {
            "kind": "variant",
            "variant": to_json_pattern_variant_destructure_resolution(value.variant),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_pattern_destructure_resolution(
    value: Json,
) -> PatternDestructureResolution:
    """Return one PatternDestructureResolution from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "tuple":
        return PatternDestructureResolutionTuple(
            tuple=from_json_pattern_tuple_destructure_resolution(
                json_field(object_, "tuple")
            )
        )
    elif kind == "object":
        return PatternDestructureResolutionObject(
            object=from_json_pattern_object_destructure_resolution(
                json_field(object_, "object")
            )
        )
    elif kind == "nominal":
        return PatternDestructureResolutionNominal(
            nominal=from_json_pattern_nominal_destructure_resolution(
                json_field(object_, "nominal")
            )
        )
    elif kind == "sequence":
        return PatternDestructureResolutionSequence(
            sequence=from_json_pattern_sequence_destructure_resolution(
                json_field(object_, "sequence")
            )
        )
    elif kind == "variant":
        return PatternDestructureResolutionVariant(
            variant=from_json_pattern_variant_destructure_resolution(
                json_field(object_, "variant")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class PatternTupleDestructureResolution:
    """Tuple destructuring selected by one pattern."""

    # the tuple fields in source order
    fields: Sequence[PatternFieldResolution]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_tuple_destructure_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternTupleDestructureResolution:
        """Decode one PatternTupleDestructureResolution."""
        return decode_pattern_tuple_destructure_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_tuple_destructure_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> PatternTupleDestructureResolution:
        """Return one PatternTupleDestructureResolution from one JSON value."""
        return from_json_pattern_tuple_destructure_resolution(value)


def encode_pattern_tuple_destructure_resolution(
    writer: BinaryWriter, value: PatternTupleDestructureResolution
) -> None:
    """Encode one PatternTupleDestructureResolution."""
    writer.write_unsigned(len(value.fields))
    for item_value_fields_0 in value.fields:
        encode_pattern_field_resolution(writer, item_value_fields_0)


def decode_pattern_tuple_destructure_resolution(
    reader: BinaryReader,
) -> PatternTupleDestructureResolution:
    """Decode one PatternTupleDestructureResolution."""
    fields = [
        decode_pattern_field_resolution(reader) for _ in range(reader.read_number())
    ]

    return PatternTupleDestructureResolution(
        fields=fields,
    )


def to_json_pattern_tuple_destructure_resolution(
    value: PatternTupleDestructureResolution,
) -> Json:
    """Return one JSON value for one PatternTupleDestructureResolution."""
    return {
        "fields": [to_json_pattern_field_resolution(item_0) for item_0 in value.fields],
    }


def from_json_pattern_tuple_destructure_resolution(
    value: Json,
) -> PatternTupleDestructureResolution:
    """Return one PatternTupleDestructureResolution from one JSON value."""
    object_ = json_object(value)

    return PatternTupleDestructureResolution(
        fields=[
            from_json_pattern_field_resolution(item_0)
            for item_0 in json_array(json_field(object_, "fields"))
        ],
    )


@dataclass(frozen=True, slots=True)
class PatternFieldResolution:
    """One destructured pattern field."""

    # the source node that introduces the field
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the selected field projection
    projection: destack._generated.dir.type.projection.Projection
    # the nested pattern matched for the field
    pattern: destack._generated.dir.tree.node.GlobalNodeIdAny | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_field_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternFieldResolution:
        """Decode one PatternFieldResolution."""
        return decode_pattern_field_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_field_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> PatternFieldResolution:
        """Return one PatternFieldResolution from one JSON value."""
        return from_json_pattern_field_resolution(value)


def encode_pattern_field_resolution(
    writer: BinaryWriter, value: PatternFieldResolution
) -> None:
    """Encode one PatternFieldResolution."""
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.source)
    destack._generated.dir.type.projection.encode_projection(writer, value.projection)
    if value.pattern is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.node.encode_global_node_id_any(
            writer, value.pattern
        )


def decode_pattern_field_resolution(reader: BinaryReader) -> PatternFieldResolution:
    """Decode one PatternFieldResolution."""
    source = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    projection = destack._generated.dir.type.projection.decode_projection(reader)
    pattern = reader.read_option(
        lambda: destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    )

    return PatternFieldResolution(
        source=source,
        projection=projection,
        pattern=pattern,
    )


def to_json_pattern_field_resolution(value: PatternFieldResolution) -> Json:
    """Return one JSON value for one PatternFieldResolution."""
    return {
        "source": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.source
        ),
        "projection": destack._generated.dir.type.projection.to_json_projection(
            value.projection
        ),
        **(
            {}
            if value.pattern is None
            else {
                "pattern": destack._generated.dir.tree.node.to_json_global_node_id_any(
                    value.pattern
                )
            }
        ),
    }


def from_json_pattern_field_resolution(value: Json) -> PatternFieldResolution:
    """Return one PatternFieldResolution from one JSON value."""
    object_ = json_object(value)

    return PatternFieldResolution(
        source=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "source")
        ),
        projection=destack._generated.dir.type.projection.from_json_projection(
            json_field(object_, "projection")
        ),
        pattern=json_optional(
            object_,
            "pattern",
            lambda value: destack._generated.dir.tree.node.from_json_global_node_id_any(
                value
            ),
        ),
    )


@dataclass(frozen=True, slots=True)
class PatternObjectDestructureResolution:
    """Object destructuring selected by one pattern."""

    # the object fields in source order
    fields: Sequence[PatternFieldResolution]
    # the rest field, when present
    rest: PatternFieldResolution | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_object_destructure_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternObjectDestructureResolution:
        """Decode one PatternObjectDestructureResolution."""
        return decode_pattern_object_destructure_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_object_destructure_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> PatternObjectDestructureResolution:
        """Return one PatternObjectDestructureResolution from one JSON value."""
        return from_json_pattern_object_destructure_resolution(value)


def encode_pattern_object_destructure_resolution(
    writer: BinaryWriter, value: PatternObjectDestructureResolution
) -> None:
    """Encode one PatternObjectDestructureResolution."""
    writer.write_unsigned(len(value.fields))
    for item_value_fields_0 in value.fields:
        encode_pattern_field_resolution(writer, item_value_fields_0)
    if value.rest is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_pattern_field_resolution(writer, value.rest)


def decode_pattern_object_destructure_resolution(
    reader: BinaryReader,
) -> PatternObjectDestructureResolution:
    """Decode one PatternObjectDestructureResolution."""
    fields = [
        decode_pattern_field_resolution(reader) for _ in range(reader.read_number())
    ]
    rest = reader.read_option(lambda: decode_pattern_field_resolution(reader))

    return PatternObjectDestructureResolution(
        fields=fields,
        rest=rest,
    )


def to_json_pattern_object_destructure_resolution(
    value: PatternObjectDestructureResolution,
) -> Json:
    """Return one JSON value for one PatternObjectDestructureResolution."""
    return {
        "fields": [to_json_pattern_field_resolution(item_0) for item_0 in value.fields],
        **(
            {}
            if value.rest is None
            else {"rest": to_json_pattern_field_resolution(value.rest)}
        ),
    }


def from_json_pattern_object_destructure_resolution(
    value: Json,
) -> PatternObjectDestructureResolution:
    """Return one PatternObjectDestructureResolution from one JSON value."""
    object_ = json_object(value)

    return PatternObjectDestructureResolution(
        fields=[
            from_json_pattern_field_resolution(item_0)
            for item_0 in json_array(json_field(object_, "fields"))
        ],
        rest=json_optional(
            object_, "rest", lambda value: from_json_pattern_field_resolution(value)
        ),
    )


@dataclass(frozen=True, slots=True)
class PatternNominalDestructureResolution:
    """Nominal destructuring selected by one pattern."""

    # the selected nominal symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the selected generic argument bindings for the nominal symbol
    generic_arguments: Sequence[
        destack._generated.dir.type.generic.GenericArgumentBinding
    ]
    # the nominal fields in source order
    fields: Sequence[PatternFieldResolution]
    # the rest field, when present
    rest: PatternFieldResolution | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_nominal_destructure_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternNominalDestructureResolution:
        """Decode one PatternNominalDestructureResolution."""
        return decode_pattern_nominal_destructure_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_nominal_destructure_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> PatternNominalDestructureResolution:
        """Return one PatternNominalDestructureResolution from one JSON value."""
        return from_json_pattern_nominal_destructure_resolution(value)


def encode_pattern_nominal_destructure_resolution(
    writer: BinaryWriter, value: PatternNominalDestructureResolution
) -> None:
    """Encode one PatternNominalDestructureResolution."""
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(writer, value.symbol)
    writer.write_unsigned(len(value.generic_arguments))
    for item_value_generic_arguments_0 in value.generic_arguments:
        destack._generated.dir.type.generic.encode_generic_argument_binding(
            writer, item_value_generic_arguments_0
        )
    writer.write_unsigned(len(value.fields))
    for item_value_fields_0 in value.fields:
        encode_pattern_field_resolution(writer, item_value_fields_0)
    if value.rest is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_pattern_field_resolution(writer, value.rest)


def decode_pattern_nominal_destructure_resolution(
    reader: BinaryReader,
) -> PatternNominalDestructureResolution:
    """Decode one PatternNominalDestructureResolution."""
    symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    generic_arguments = [
        destack._generated.dir.type.generic.decode_generic_argument_binding(reader)
        for _ in range(reader.read_number())
    ]
    fields = [
        decode_pattern_field_resolution(reader) for _ in range(reader.read_number())
    ]
    rest = reader.read_option(lambda: decode_pattern_field_resolution(reader))

    return PatternNominalDestructureResolution(
        symbol=symbol,
        generic_arguments=generic_arguments,
        fields=fields,
        rest=rest,
    )


def to_json_pattern_nominal_destructure_resolution(
    value: PatternNominalDestructureResolution,
) -> Json:
    """Return one JSON value for one PatternNominalDestructureResolution."""
    return {
        "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.symbol
        ),
        "genericArguments": [
            destack._generated.dir.type.generic.to_json_generic_argument_binding(item_0)
            for item_0 in value.generic_arguments
        ],
        "fields": [to_json_pattern_field_resolution(item_0) for item_0 in value.fields],
        **(
            {}
            if value.rest is None
            else {"rest": to_json_pattern_field_resolution(value.rest)}
        ),
    }


def from_json_pattern_nominal_destructure_resolution(
    value: Json,
) -> PatternNominalDestructureResolution:
    """Return one PatternNominalDestructureResolution from one JSON value."""
    object_ = json_object(value)

    return PatternNominalDestructureResolution(
        symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "symbol")
        ),
        generic_arguments=[
            destack._generated.dir.type.generic.from_json_generic_argument_binding(
                item_0
            )
            for item_0 in json_array(json_field(object_, "genericArguments"))
        ],
        fields=[
            from_json_pattern_field_resolution(item_0)
            for item_0 in json_array(json_field(object_, "fields"))
        ],
        rest=json_optional(
            object_, "rest", lambda value: from_json_pattern_field_resolution(value)
        ),
    )


@dataclass(frozen=True, slots=True)
class PatternSequenceDestructureResolution:
    """Sequence destructuring selected by one pattern."""

    # the arity requirement introduced by the pattern
    arity: PatternSequenceArity
    # the fixed fields in source order
    fields: Sequence[PatternFieldResolution]
    # the rest field, when present
    rest: PatternFieldResolution | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_sequence_destructure_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternSequenceDestructureResolution:
        """Decode one PatternSequenceDestructureResolution."""
        return decode_pattern_sequence_destructure_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_sequence_destructure_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> PatternSequenceDestructureResolution:
        """Return one PatternSequenceDestructureResolution from one JSON value."""
        return from_json_pattern_sequence_destructure_resolution(value)


def encode_pattern_sequence_destructure_resolution(
    writer: BinaryWriter, value: PatternSequenceDestructureResolution
) -> None:
    """Encode one PatternSequenceDestructureResolution."""
    encode_pattern_sequence_arity(writer, value.arity)
    writer.write_unsigned(len(value.fields))
    for item_value_fields_0 in value.fields:
        encode_pattern_field_resolution(writer, item_value_fields_0)
    if value.rest is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_pattern_field_resolution(writer, value.rest)


def decode_pattern_sequence_destructure_resolution(
    reader: BinaryReader,
) -> PatternSequenceDestructureResolution:
    """Decode one PatternSequenceDestructureResolution."""
    arity = decode_pattern_sequence_arity(reader)
    fields = [
        decode_pattern_field_resolution(reader) for _ in range(reader.read_number())
    ]
    rest = reader.read_option(lambda: decode_pattern_field_resolution(reader))

    return PatternSequenceDestructureResolution(
        arity=arity,
        fields=fields,
        rest=rest,
    )


def to_json_pattern_sequence_destructure_resolution(
    value: PatternSequenceDestructureResolution,
) -> Json:
    """Return one JSON value for one PatternSequenceDestructureResolution."""
    return {
        "arity": to_json_pattern_sequence_arity(value.arity),
        "fields": [to_json_pattern_field_resolution(item_0) for item_0 in value.fields],
        **(
            {}
            if value.rest is None
            else {"rest": to_json_pattern_field_resolution(value.rest)}
        ),
    }


def from_json_pattern_sequence_destructure_resolution(
    value: Json,
) -> PatternSequenceDestructureResolution:
    """Return one PatternSequenceDestructureResolution from one JSON value."""
    object_ = json_object(value)

    return PatternSequenceDestructureResolution(
        arity=from_json_pattern_sequence_arity(json_field(object_, "arity")),
        fields=[
            from_json_pattern_field_resolution(item_0)
            for item_0 in json_array(json_field(object_, "fields"))
        ],
        rest=json_optional(
            object_, "rest", lambda value: from_json_pattern_field_resolution(value)
        ),
    )


@dataclass(frozen=True, slots=True)
class PatternSequenceArity:
    """Arity requirement introduced by one sequence pattern."""

    # the minimum accepted source length
    minimum: int
    # the maximum accepted source length, when bounded
    maximum: int | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_sequence_arity(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternSequenceArity:
        """Decode one PatternSequenceArity."""
        return decode_pattern_sequence_arity(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_sequence_arity(self)

    @classmethod
    def from_json(cls, value: Json) -> PatternSequenceArity:
        """Return one PatternSequenceArity from one JSON value."""
        return from_json_pattern_sequence_arity(value)


def encode_pattern_sequence_arity(
    writer: BinaryWriter, value: PatternSequenceArity
) -> None:
    """Encode one PatternSequenceArity."""
    writer.write_unsigned(value.minimum)
    if value.maximum is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(value.maximum)


def decode_pattern_sequence_arity(reader: BinaryReader) -> PatternSequenceArity:
    """Decode one PatternSequenceArity."""
    minimum = reader.read_number()
    maximum = reader.read_option(lambda: reader.read_number())

    return PatternSequenceArity(
        minimum=minimum,
        maximum=maximum,
    )


def to_json_pattern_sequence_arity(value: PatternSequenceArity) -> Json:
    """Return one JSON value for one PatternSequenceArity."""
    return {
        "minimum": value.minimum,
        **({} if value.maximum is None else {"maximum": value.maximum}),
    }


def from_json_pattern_sequence_arity(value: Json) -> PatternSequenceArity:
    """Return one PatternSequenceArity from one JSON value."""
    object_ = json_object(value)

    return PatternSequenceArity(
        minimum=json_int(json_field(object_, "minimum")),
        maximum=json_optional(object_, "maximum", lambda value: json_int(value)),
    )


@dataclass(frozen=True, slots=True)
class PatternVariantDestructureResolution:
    """Tagged variant destructuring selected by one pattern."""

    # the selected variant predicate
    predicate: destack._generated.dir.type.predicate.Predicate
    # the selected variant payload projection
    projection: destack._generated.dir.type.projection.Projection
    # the payload fields in source order
    fields: Sequence[PatternFieldResolution]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_variant_destructure_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternVariantDestructureResolution:
        """Decode one PatternVariantDestructureResolution."""
        return decode_pattern_variant_destructure_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_variant_destructure_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> PatternVariantDestructureResolution:
        """Return one PatternVariantDestructureResolution from one JSON value."""
        return from_json_pattern_variant_destructure_resolution(value)


def encode_pattern_variant_destructure_resolution(
    writer: BinaryWriter, value: PatternVariantDestructureResolution
) -> None:
    """Encode one PatternVariantDestructureResolution."""
    destack._generated.dir.type.predicate.encode_predicate(writer, value.predicate)
    destack._generated.dir.type.projection.encode_projection(writer, value.projection)
    writer.write_unsigned(len(value.fields))
    for item_value_fields_0 in value.fields:
        encode_pattern_field_resolution(writer, item_value_fields_0)


def decode_pattern_variant_destructure_resolution(
    reader: BinaryReader,
) -> PatternVariantDestructureResolution:
    """Decode one PatternVariantDestructureResolution."""
    predicate = destack._generated.dir.type.predicate.decode_predicate(reader)
    projection = destack._generated.dir.type.projection.decode_projection(reader)
    fields = [
        decode_pattern_field_resolution(reader) for _ in range(reader.read_number())
    ]

    return PatternVariantDestructureResolution(
        predicate=predicate,
        projection=projection,
        fields=fields,
    )


def to_json_pattern_variant_destructure_resolution(
    value: PatternVariantDestructureResolution,
) -> Json:
    """Return one JSON value for one PatternVariantDestructureResolution."""
    return {
        "predicate": destack._generated.dir.type.predicate.to_json_predicate(
            value.predicate
        ),
        "projection": destack._generated.dir.type.projection.to_json_projection(
            value.projection
        ),
        "fields": [to_json_pattern_field_resolution(item_0) for item_0 in value.fields],
    }


def from_json_pattern_variant_destructure_resolution(
    value: Json,
) -> PatternVariantDestructureResolution:
    """Return one PatternVariantDestructureResolution from one JSON value."""
    object_ = json_object(value)

    return PatternVariantDestructureResolution(
        predicate=destack._generated.dir.type.predicate.from_json_predicate(
            json_field(object_, "predicate")
        ),
        projection=destack._generated.dir.type.projection.from_json_projection(
            json_field(object_, "projection")
        ),
        fields=[
            from_json_pattern_field_resolution(item_0)
            for item_0 in json_array(json_field(object_, "fields"))
        ],
    )


@dataclass(frozen=True, slots=True)
class PatternOrResolution:
    """Or-pattern branches selected during checking."""

    # the branch pattern nodes
    patterns: Sequence[destack._generated.dir.tree.node.GlobalNodeIdAny]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_or_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternOrResolution:
        """Decode one PatternOrResolution."""
        return decode_pattern_or_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_or_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> PatternOrResolution:
        """Return one PatternOrResolution from one JSON value."""
        return from_json_pattern_or_resolution(value)


def encode_pattern_or_resolution(
    writer: BinaryWriter, value: PatternOrResolution
) -> None:
    """Encode one PatternOrResolution."""
    writer.write_unsigned(len(value.patterns))
    for item_value_patterns_0 in value.patterns:
        destack._generated.dir.tree.node.encode_global_node_id_any(
            writer, item_value_patterns_0
        )


def decode_pattern_or_resolution(reader: BinaryReader) -> PatternOrResolution:
    """Decode one PatternOrResolution."""
    patterns = [
        destack._generated.dir.tree.node.decode_global_node_id_any(reader)
        for _ in range(reader.read_number())
    ]

    return PatternOrResolution(
        patterns=patterns,
    )


def to_json_pattern_or_resolution(value: PatternOrResolution) -> Json:
    """Return one JSON value for one PatternOrResolution."""
    return {
        "patterns": [
            destack._generated.dir.tree.node.to_json_global_node_id_any(item_0)
            for item_0 in value.patterns
        ],
    }


def from_json_pattern_or_resolution(value: Json) -> PatternOrResolution:
    """Return one PatternOrResolution from one JSON value."""
    object_ = json_object(value)

    return PatternOrResolution(
        patterns=[
            destack._generated.dir.tree.node.from_json_global_node_id_any(item_0)
            for item_0 in json_array(json_field(object_, "patterns"))
        ],
    )


@dataclass(frozen=True, slots=True)
class AssignPatternResolutionPlace:
    """Direct writable place target, like `value` or `object.field`."""

    place: PlaceResolution
    kind: typing.Literal["place"] = "place"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_assign_pattern_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_assign_pattern_resolution(self)


@dataclass(frozen=True, slots=True)
class AssignPatternResolutionDefault:
    """Defaulted assignment target, like `value = fallback`."""

    default: AssignPatternDefaultResolution
    kind: typing.Literal["default"] = "default"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_assign_pattern_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_assign_pattern_resolution(self)


@dataclass(frozen=True, slots=True)
class AssignPatternResolutionSequence:
    """Ordered destructuring target, like `[head, ...tail]`."""

    sequence: AssignPatternSequenceResolution
    kind: typing.Literal["sequence"] = "sequence"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_assign_pattern_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_assign_pattern_resolution(self)


@dataclass(frozen=True, slots=True)
class AssignPatternResolutionTuple:
    """Tuple destructuring target, like `(x, y)` or `(x,)`."""

    tuple: AssignPatternTupleResolution
    kind: typing.Literal["tuple"] = "tuple"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_assign_pattern_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_assign_pattern_resolution(self)


@dataclass(frozen=True, slots=True)
class AssignPatternResolutionObject:
    """Object destructuring target, like `{ name, age: years }`."""

    object: AssignPatternObjectResolution
    kind: typing.Literal["object"] = "object"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_assign_pattern_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_assign_pattern_resolution(self)


"""Assignment target meaning selected during checking."""
AssignPatternResolution: typing.TypeAlias = (
    AssignPatternResolutionPlace
    | AssignPatternResolutionDefault
    | AssignPatternResolutionSequence
    | AssignPatternResolutionTuple
    | AssignPatternResolutionObject
)


def encode_assign_pattern_resolution(
    writer: BinaryWriter, value: AssignPatternResolution
) -> None:
    """Encode one AssignPatternResolution."""
    if value.kind == "place":
        writer.write_unsigned(0)
        encode_place_resolution(writer, value.place)
    elif value.kind == "default":
        writer.write_unsigned(1)
        encode_assign_pattern_default_resolution(writer, value.default)
    elif value.kind == "sequence":
        writer.write_unsigned(2)
        encode_assign_pattern_sequence_resolution(writer, value.sequence)
    elif value.kind == "tuple":
        writer.write_unsigned(3)
        encode_assign_pattern_tuple_resolution(writer, value.tuple)
    elif value.kind == "object":
        writer.write_unsigned(4)
        encode_assign_pattern_object_resolution(writer, value.object)
    else:
        raise SerdeError("unknown enum variant")


def decode_assign_pattern_resolution(reader: BinaryReader) -> AssignPatternResolution:
    """Decode one AssignPatternResolution."""
    variant = reader.read_number()

    if variant == 0:
        place = decode_place_resolution(reader)

        return AssignPatternResolutionPlace(place=place)
    elif variant == 1:
        default = decode_assign_pattern_default_resolution(reader)

        return AssignPatternResolutionDefault(default=default)
    elif variant == 2:
        sequence = decode_assign_pattern_sequence_resolution(reader)

        return AssignPatternResolutionSequence(sequence=sequence)
    elif variant == 3:
        tuple = decode_assign_pattern_tuple_resolution(reader)

        return AssignPatternResolutionTuple(tuple=tuple)
    elif variant == 4:
        object = decode_assign_pattern_object_resolution(reader)

        return AssignPatternResolutionObject(object=object)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_assign_pattern_resolution(value: AssignPatternResolution) -> Json:
    """Return one JSON value for one AssignPatternResolution."""
    if value.kind == "place":
        return {
            "kind": "place",
            "place": to_json_place_resolution(value.place),
        }
    elif value.kind == "default":
        return {
            "kind": "default",
            "default": to_json_assign_pattern_default_resolution(value.default),
        }
    elif value.kind == "sequence":
        return {
            "kind": "sequence",
            "sequence": to_json_assign_pattern_sequence_resolution(value.sequence),
        }
    elif value.kind == "tuple":
        return {
            "kind": "tuple",
            "tuple": to_json_assign_pattern_tuple_resolution(value.tuple),
        }
    elif value.kind == "object":
        return {
            "kind": "object",
            "object": to_json_assign_pattern_object_resolution(value.object),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_assign_pattern_resolution(value: Json) -> AssignPatternResolution:
    """Return one AssignPatternResolution from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "place":
        return AssignPatternResolutionPlace(
            place=from_json_place_resolution(json_field(object_, "place"))
        )
    elif kind == "default":
        return AssignPatternResolutionDefault(
            default=from_json_assign_pattern_default_resolution(
                json_field(object_, "default")
            )
        )
    elif kind == "sequence":
        return AssignPatternResolutionSequence(
            sequence=from_json_assign_pattern_sequence_resolution(
                json_field(object_, "sequence")
            )
        )
    elif kind == "tuple":
        return AssignPatternResolutionTuple(
            tuple=from_json_assign_pattern_tuple_resolution(
                json_field(object_, "tuple")
            )
        )
    elif kind == "object":
        return AssignPatternResolutionObject(
            object=from_json_assign_pattern_object_resolution(
                json_field(object_, "object")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class AssignPatternDefaultResolution:
    """Defaulted assignment target selected during checking."""

    # the nested assignment target
    pattern: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the fallback expression
    value: destack._generated.dir.tree.node.GlobalNodeIdAny

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_assign_pattern_default_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> AssignPatternDefaultResolution:
        """Decode one AssignPatternDefaultResolution."""
        return decode_assign_pattern_default_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_assign_pattern_default_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> AssignPatternDefaultResolution:
        """Return one AssignPatternDefaultResolution from one JSON value."""
        return from_json_assign_pattern_default_resolution(value)


def encode_assign_pattern_default_resolution(
    writer: BinaryWriter, value: AssignPatternDefaultResolution
) -> None:
    """Encode one AssignPatternDefaultResolution."""
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.pattern)
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.value)


def decode_assign_pattern_default_resolution(
    reader: BinaryReader,
) -> AssignPatternDefaultResolution:
    """Decode one AssignPatternDefaultResolution."""
    pattern = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    value_ = destack._generated.dir.tree.node.decode_global_node_id_any(reader)

    return AssignPatternDefaultResolution(
        pattern=pattern,
        value=value_,
    )


def to_json_assign_pattern_default_resolution(
    value: AssignPatternDefaultResolution,
) -> Json:
    """Return one JSON value for one AssignPatternDefaultResolution."""
    return {
        "pattern": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.pattern
        ),
        "value": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.value
        ),
    }


def from_json_assign_pattern_default_resolution(
    value: Json,
) -> AssignPatternDefaultResolution:
    """Return one AssignPatternDefaultResolution from one JSON value."""
    object_ = json_object(value)

    return AssignPatternDefaultResolution(
        pattern=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "pattern")
        ),
        value=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "value")
        ),
    )


@dataclass(frozen=True, slots=True)
class AssignPatternSequenceResolution:
    """Ordered assignment destructuring selected during checking."""

    # the sequence arity required by the assignment target
    arity: PatternSequenceArity
    # the fixed fields in source order
    fields: Sequence[AssignPatternFieldResolution]
    # the rest target, when present
    rest: AssignPatternFieldResolution | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_assign_pattern_sequence_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> AssignPatternSequenceResolution:
        """Decode one AssignPatternSequenceResolution."""
        return decode_assign_pattern_sequence_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_assign_pattern_sequence_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> AssignPatternSequenceResolution:
        """Return one AssignPatternSequenceResolution from one JSON value."""
        return from_json_assign_pattern_sequence_resolution(value)


def encode_assign_pattern_sequence_resolution(
    writer: BinaryWriter, value: AssignPatternSequenceResolution
) -> None:
    """Encode one AssignPatternSequenceResolution."""
    encode_pattern_sequence_arity(writer, value.arity)
    writer.write_unsigned(len(value.fields))
    for item_value_fields_0 in value.fields:
        encode_assign_pattern_field_resolution(writer, item_value_fields_0)
    if value.rest is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_assign_pattern_field_resolution(writer, value.rest)


def decode_assign_pattern_sequence_resolution(
    reader: BinaryReader,
) -> AssignPatternSequenceResolution:
    """Decode one AssignPatternSequenceResolution."""
    arity = decode_pattern_sequence_arity(reader)
    fields = [
        decode_assign_pattern_field_resolution(reader)
        for _ in range(reader.read_number())
    ]
    rest = reader.read_option(lambda: decode_assign_pattern_field_resolution(reader))

    return AssignPatternSequenceResolution(
        arity=arity,
        fields=fields,
        rest=rest,
    )


def to_json_assign_pattern_sequence_resolution(
    value: AssignPatternSequenceResolution,
) -> Json:
    """Return one JSON value for one AssignPatternSequenceResolution."""
    return {
        "arity": to_json_pattern_sequence_arity(value.arity),
        "fields": [
            to_json_assign_pattern_field_resolution(item_0) for item_0 in value.fields
        ],
        **(
            {}
            if value.rest is None
            else {"rest": to_json_assign_pattern_field_resolution(value.rest)}
        ),
    }


def from_json_assign_pattern_sequence_resolution(
    value: Json,
) -> AssignPatternSequenceResolution:
    """Return one AssignPatternSequenceResolution from one JSON value."""
    object_ = json_object(value)

    return AssignPatternSequenceResolution(
        arity=from_json_pattern_sequence_arity(json_field(object_, "arity")),
        fields=[
            from_json_assign_pattern_field_resolution(item_0)
            for item_0 in json_array(json_field(object_, "fields"))
        ],
        rest=json_optional(
            object_,
            "rest",
            lambda value: from_json_assign_pattern_field_resolution(value),
        ),
    )


@dataclass(frozen=True, slots=True)
class AssignPatternFieldResolution:
    """One destructured assignment field."""

    # the source node that introduces the field
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the selected field projection
    projection: destack._generated.dir.type.projection.Projection
    # the nested assignment target
    pattern: destack._generated.dir.tree.node.GlobalNodeIdAny | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_assign_pattern_field_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> AssignPatternFieldResolution:
        """Decode one AssignPatternFieldResolution."""
        return decode_assign_pattern_field_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_assign_pattern_field_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> AssignPatternFieldResolution:
        """Return one AssignPatternFieldResolution from one JSON value."""
        return from_json_assign_pattern_field_resolution(value)


def encode_assign_pattern_field_resolution(
    writer: BinaryWriter, value: AssignPatternFieldResolution
) -> None:
    """Encode one AssignPatternFieldResolution."""
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.source)
    destack._generated.dir.type.projection.encode_projection(writer, value.projection)
    if value.pattern is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.node.encode_global_node_id_any(
            writer, value.pattern
        )


def decode_assign_pattern_field_resolution(
    reader: BinaryReader,
) -> AssignPatternFieldResolution:
    """Decode one AssignPatternFieldResolution."""
    source = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    projection = destack._generated.dir.type.projection.decode_projection(reader)
    pattern = reader.read_option(
        lambda: destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    )

    return AssignPatternFieldResolution(
        source=source,
        projection=projection,
        pattern=pattern,
    )


def to_json_assign_pattern_field_resolution(
    value: AssignPatternFieldResolution,
) -> Json:
    """Return one JSON value for one AssignPatternFieldResolution."""
    return {
        "source": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.source
        ),
        "projection": destack._generated.dir.type.projection.to_json_projection(
            value.projection
        ),
        **(
            {}
            if value.pattern is None
            else {
                "pattern": destack._generated.dir.tree.node.to_json_global_node_id_any(
                    value.pattern
                )
            }
        ),
    }


def from_json_assign_pattern_field_resolution(
    value: Json,
) -> AssignPatternFieldResolution:
    """Return one AssignPatternFieldResolution from one JSON value."""
    object_ = json_object(value)

    return AssignPatternFieldResolution(
        source=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "source")
        ),
        projection=destack._generated.dir.type.projection.from_json_projection(
            json_field(object_, "projection")
        ),
        pattern=json_optional(
            object_,
            "pattern",
            lambda value: destack._generated.dir.tree.node.from_json_global_node_id_any(
                value
            ),
        ),
    )


@dataclass(frozen=True, slots=True)
class AssignPatternTupleResolution:
    """Tuple assignment destructuring selected during checking."""

    # the projected tuple fields in source order
    fields: Sequence[AssignPatternFieldResolution]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_assign_pattern_tuple_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> AssignPatternTupleResolution:
        """Decode one AssignPatternTupleResolution."""
        return decode_assign_pattern_tuple_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_assign_pattern_tuple_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> AssignPatternTupleResolution:
        """Return one AssignPatternTupleResolution from one JSON value."""
        return from_json_assign_pattern_tuple_resolution(value)


def encode_assign_pattern_tuple_resolution(
    writer: BinaryWriter, value: AssignPatternTupleResolution
) -> None:
    """Encode one AssignPatternTupleResolution."""
    writer.write_unsigned(len(value.fields))
    for item_value_fields_0 in value.fields:
        encode_assign_pattern_field_resolution(writer, item_value_fields_0)


def decode_assign_pattern_tuple_resolution(
    reader: BinaryReader,
) -> AssignPatternTupleResolution:
    """Decode one AssignPatternTupleResolution."""
    fields = [
        decode_assign_pattern_field_resolution(reader)
        for _ in range(reader.read_number())
    ]

    return AssignPatternTupleResolution(
        fields=fields,
    )


def to_json_assign_pattern_tuple_resolution(
    value: AssignPatternTupleResolution,
) -> Json:
    """Return one JSON value for one AssignPatternTupleResolution."""
    return {
        "fields": [
            to_json_assign_pattern_field_resolution(item_0) for item_0 in value.fields
        ],
    }


def from_json_assign_pattern_tuple_resolution(
    value: Json,
) -> AssignPatternTupleResolution:
    """Return one AssignPatternTupleResolution from one JSON value."""
    object_ = json_object(value)

    return AssignPatternTupleResolution(
        fields=[
            from_json_assign_pattern_field_resolution(item_0)
            for item_0 in json_array(json_field(object_, "fields"))
        ],
    )


@dataclass(frozen=True, slots=True)
class AssignPatternObjectResolution:
    """Object assignment destructuring selected during checking."""

    # the named fields in source order
    fields: Sequence[AssignPatternFieldResolution]
    # the rest target, when present
    rest: AssignPatternRestResolution | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_assign_pattern_object_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> AssignPatternObjectResolution:
        """Decode one AssignPatternObjectResolution."""
        return decode_assign_pattern_object_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_assign_pattern_object_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> AssignPatternObjectResolution:
        """Return one AssignPatternObjectResolution from one JSON value."""
        return from_json_assign_pattern_object_resolution(value)


def encode_assign_pattern_object_resolution(
    writer: BinaryWriter, value: AssignPatternObjectResolution
) -> None:
    """Encode one AssignPatternObjectResolution."""
    writer.write_unsigned(len(value.fields))
    for item_value_fields_0 in value.fields:
        encode_assign_pattern_field_resolution(writer, item_value_fields_0)
    if value.rest is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_assign_pattern_rest_resolution(writer, value.rest)


def decode_assign_pattern_object_resolution(
    reader: BinaryReader,
) -> AssignPatternObjectResolution:
    """Decode one AssignPatternObjectResolution."""
    fields = [
        decode_assign_pattern_field_resolution(reader)
        for _ in range(reader.read_number())
    ]
    rest = reader.read_option(lambda: decode_assign_pattern_rest_resolution(reader))

    return AssignPatternObjectResolution(
        fields=fields,
        rest=rest,
    )


def to_json_assign_pattern_object_resolution(
    value: AssignPatternObjectResolution,
) -> Json:
    """Return one JSON value for one AssignPatternObjectResolution."""
    return {
        "fields": [
            to_json_assign_pattern_field_resolution(item_0) for item_0 in value.fields
        ],
        **(
            {}
            if value.rest is None
            else {"rest": to_json_assign_pattern_rest_resolution(value.rest)}
        ),
    }


def from_json_assign_pattern_object_resolution(
    value: Json,
) -> AssignPatternObjectResolution:
    """Return one AssignPatternObjectResolution from one JSON value."""
    object_ = json_object(value)

    return AssignPatternObjectResolution(
        fields=[
            from_json_assign_pattern_field_resolution(item_0)
            for item_0 in json_array(json_field(object_, "fields"))
        ],
        rest=json_optional(
            object_,
            "rest",
            lambda value: from_json_assign_pattern_rest_resolution(value),
        ),
    )


@dataclass(frozen=True, slots=True)
class AssignPatternRestResolution:
    """Rest field selected by one assignment destructuring pattern."""

    # the source node that introduces the rest field
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the materialized rest projection
    projection: destack._generated.dir.type.projection.Projection
    # the nested assignment target
    pattern: destack._generated.dir.tree.node.GlobalNodeIdAny | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_assign_pattern_rest_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> AssignPatternRestResolution:
        """Decode one AssignPatternRestResolution."""
        return decode_assign_pattern_rest_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_assign_pattern_rest_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> AssignPatternRestResolution:
        """Return one AssignPatternRestResolution from one JSON value."""
        return from_json_assign_pattern_rest_resolution(value)


def encode_assign_pattern_rest_resolution(
    writer: BinaryWriter, value: AssignPatternRestResolution
) -> None:
    """Encode one AssignPatternRestResolution."""
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.source)
    destack._generated.dir.type.projection.encode_projection(writer, value.projection)
    if value.pattern is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.node.encode_global_node_id_any(
            writer, value.pattern
        )


def decode_assign_pattern_rest_resolution(
    reader: BinaryReader,
) -> AssignPatternRestResolution:
    """Decode one AssignPatternRestResolution."""
    source = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    projection = destack._generated.dir.type.projection.decode_projection(reader)
    pattern = reader.read_option(
        lambda: destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    )

    return AssignPatternRestResolution(
        source=source,
        projection=projection,
        pattern=pattern,
    )


def to_json_assign_pattern_rest_resolution(value: AssignPatternRestResolution) -> Json:
    """Return one JSON value for one AssignPatternRestResolution."""
    return {
        "source": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.source
        ),
        "projection": destack._generated.dir.type.projection.to_json_projection(
            value.projection
        ),
        **(
            {}
            if value.pattern is None
            else {
                "pattern": destack._generated.dir.tree.node.to_json_global_node_id_any(
                    value.pattern
                )
            }
        ),
    }


def from_json_assign_pattern_rest_resolution(
    value: Json,
) -> AssignPatternRestResolution:
    """Return one AssignPatternRestResolution from one JSON value."""
    object_ = json_object(value)

    return AssignPatternRestResolution(
        source=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "source")
        ),
        projection=destack._generated.dir.type.projection.from_json_projection(
            json_field(object_, "projection")
        ),
        pattern=json_optional(
            object_,
            "pattern",
            lambda value: destack._generated.dir.tree.node.from_json_global_node_id_any(
                value
            ),
        ),
    )


__all__ = [
    "NameResolution",
    "encode_name_resolution",
    "decode_name_resolution",
    "to_json_name_resolution",
    "from_json_name_resolution",
    "InstantiationResolution",
    "encode_instantiation_resolution",
    "decode_instantiation_resolution",
    "to_json_instantiation_resolution",
    "from_json_instantiation_resolution",
    "LabelResolution",
    "encode_label_resolution",
    "decode_label_resolution",
    "to_json_label_resolution",
    "from_json_label_resolution",
    "LabelResolutionSymbol",
    "LabelResolutionLoop",
    "LabelResolutionFunction",
    "ReceiverResolution",
    "encode_receiver_resolution",
    "decode_receiver_resolution",
    "to_json_receiver_resolution",
    "from_json_receiver_resolution",
    "ReceiverKind",
    "encode_receiver_kind",
    "decode_receiver_kind",
    "to_json_receiver_kind",
    "from_json_receiver_kind",
    "MemberResolution",
    "encode_member_resolution",
    "decode_member_resolution",
    "to_json_member_resolution",
    "from_json_member_resolution",
    "MemberTarget",
    "encode_member_target",
    "decode_member_target",
    "to_json_member_target",
    "from_json_member_target",
    "MemberTargetField",
    "MemberTargetElement",
    "MemberTargetIndex",
    "MemberTargetSymbol",
    "MemberTargetExistential",
    "MemberTargetUniversal",
    "MemberCandidate",
    "encode_member_candidate",
    "decode_member_candidate",
    "to_json_member_candidate",
    "from_json_member_candidate",
    "CallResolution",
    "encode_call_resolution",
    "decode_call_resolution",
    "to_json_call_resolution",
    "from_json_call_resolution",
    "CallTarget",
    "encode_call_target",
    "decode_call_target",
    "to_json_call_target",
    "from_json_call_target",
    "CallTargetBuiltin",
    "CallTargetExpression",
    "CallTargetSymbol",
    "CallTargetUniversal",
    "BuiltinCall",
    "encode_builtin_call",
    "decode_builtin_call",
    "to_json_builtin_call",
    "from_json_builtin_call",
    "BuiltinCallUnaryOperator",
    "BuiltinCallBinaryOperator",
    "CallCandidate",
    "encode_call_candidate",
    "decode_call_candidate",
    "to_json_call_candidate",
    "from_json_call_candidate",
    "PlaceResolution",
    "encode_place_resolution",
    "decode_place_resolution",
    "to_json_place_resolution",
    "from_json_place_resolution",
    "Storage",
    "encode_storage",
    "decode_storage",
    "to_json_storage",
    "from_json_storage",
    "StorageBinding",
    "StorageField",
    "StorageProperty",
    "StorageSubscript",
    "StorageDereference",
    "GuardResolution",
    "encode_guard_resolution",
    "decode_guard_resolution",
    "to_json_guard_resolution",
    "from_json_guard_resolution",
    "GuardResolutionIs",
    "GuardResolutionInstanceOf",
    "GuardResolutionIn",
    "IsGuardResolution",
    "encode_is_guard_resolution",
    "decode_is_guard_resolution",
    "to_json_is_guard_resolution",
    "from_json_is_guard_resolution",
    "InstanceOfGuardResolution",
    "encode_instance_of_guard_resolution",
    "decode_instance_of_guard_resolution",
    "to_json_instance_of_guard_resolution",
    "from_json_instance_of_guard_resolution",
    "InGuardResolution",
    "encode_in_guard_resolution",
    "decode_in_guard_resolution",
    "to_json_in_guard_resolution",
    "from_json_in_guard_resolution",
    "ConstructResolution",
    "encode_construct_resolution",
    "decode_construct_resolution",
    "to_json_construct_resolution",
    "from_json_construct_resolution",
    "ConstructTarget",
    "encode_construct_target",
    "decode_construct_target",
    "to_json_construct_target",
    "from_json_construct_target",
    "ConstructTargetClass",
    "ConstructTargetNewtype",
    "ConstructTargetVariant",
    "ClassConstructCandidate",
    "encode_class_construct_candidate",
    "decode_class_construct_candidate",
    "to_json_class_construct_candidate",
    "from_json_class_construct_candidate",
    "NewtypeConstructCandidate",
    "encode_newtype_construct_candidate",
    "decode_newtype_construct_candidate",
    "to_json_newtype_construct_candidate",
    "from_json_newtype_construct_candidate",
    "VariantConstructCandidate",
    "encode_variant_construct_candidate",
    "decode_variant_construct_candidate",
    "to_json_variant_construct_candidate",
    "from_json_variant_construct_candidate",
    "PatternResolution",
    "encode_pattern_resolution",
    "decode_pattern_resolution",
    "to_json_pattern_resolution",
    "from_json_pattern_resolution",
    "PatternResolutionIgnore",
    "PatternResolutionBind",
    "PatternResolutionMust",
    "PatternResolutionDefault",
    "PatternResolutionTest",
    "PatternResolutionProject",
    "PatternResolutionDestructure",
    "PatternResolutionOr",
    "PatternBindingResolution",
    "encode_pattern_binding_resolution",
    "decode_pattern_binding_resolution",
    "to_json_pattern_binding_resolution",
    "from_json_pattern_binding_resolution",
    "PatternMustResolution",
    "encode_pattern_must_resolution",
    "decode_pattern_must_resolution",
    "to_json_pattern_must_resolution",
    "from_json_pattern_must_resolution",
    "PatternDefaultResolution",
    "encode_pattern_default_resolution",
    "decode_pattern_default_resolution",
    "to_json_pattern_default_resolution",
    "from_json_pattern_default_resolution",
    "PatternPredicateResolution",
    "encode_pattern_predicate_resolution",
    "decode_pattern_predicate_resolution",
    "to_json_pattern_predicate_resolution",
    "from_json_pattern_predicate_resolution",
    "PatternProjectionResolution",
    "encode_pattern_projection_resolution",
    "decode_pattern_projection_resolution",
    "to_json_pattern_projection_resolution",
    "from_json_pattern_projection_resolution",
    "PatternDestructureResolution",
    "encode_pattern_destructure_resolution",
    "decode_pattern_destructure_resolution",
    "to_json_pattern_destructure_resolution",
    "from_json_pattern_destructure_resolution",
    "PatternDestructureResolutionTuple",
    "PatternDestructureResolutionObject",
    "PatternDestructureResolutionNominal",
    "PatternDestructureResolutionSequence",
    "PatternDestructureResolutionVariant",
    "PatternTupleDestructureResolution",
    "encode_pattern_tuple_destructure_resolution",
    "decode_pattern_tuple_destructure_resolution",
    "to_json_pattern_tuple_destructure_resolution",
    "from_json_pattern_tuple_destructure_resolution",
    "PatternFieldResolution",
    "encode_pattern_field_resolution",
    "decode_pattern_field_resolution",
    "to_json_pattern_field_resolution",
    "from_json_pattern_field_resolution",
    "PatternObjectDestructureResolution",
    "encode_pattern_object_destructure_resolution",
    "decode_pattern_object_destructure_resolution",
    "to_json_pattern_object_destructure_resolution",
    "from_json_pattern_object_destructure_resolution",
    "PatternNominalDestructureResolution",
    "encode_pattern_nominal_destructure_resolution",
    "decode_pattern_nominal_destructure_resolution",
    "to_json_pattern_nominal_destructure_resolution",
    "from_json_pattern_nominal_destructure_resolution",
    "PatternSequenceDestructureResolution",
    "encode_pattern_sequence_destructure_resolution",
    "decode_pattern_sequence_destructure_resolution",
    "to_json_pattern_sequence_destructure_resolution",
    "from_json_pattern_sequence_destructure_resolution",
    "PatternSequenceArity",
    "encode_pattern_sequence_arity",
    "decode_pattern_sequence_arity",
    "to_json_pattern_sequence_arity",
    "from_json_pattern_sequence_arity",
    "PatternVariantDestructureResolution",
    "encode_pattern_variant_destructure_resolution",
    "decode_pattern_variant_destructure_resolution",
    "to_json_pattern_variant_destructure_resolution",
    "from_json_pattern_variant_destructure_resolution",
    "PatternOrResolution",
    "encode_pattern_or_resolution",
    "decode_pattern_or_resolution",
    "to_json_pattern_or_resolution",
    "from_json_pattern_or_resolution",
    "AssignPatternResolution",
    "encode_assign_pattern_resolution",
    "decode_assign_pattern_resolution",
    "to_json_assign_pattern_resolution",
    "from_json_assign_pattern_resolution",
    "AssignPatternResolutionPlace",
    "AssignPatternResolutionDefault",
    "AssignPatternResolutionSequence",
    "AssignPatternResolutionTuple",
    "AssignPatternResolutionObject",
    "AssignPatternDefaultResolution",
    "encode_assign_pattern_default_resolution",
    "decode_assign_pattern_default_resolution",
    "to_json_assign_pattern_default_resolution",
    "from_json_assign_pattern_default_resolution",
    "AssignPatternSequenceResolution",
    "encode_assign_pattern_sequence_resolution",
    "decode_assign_pattern_sequence_resolution",
    "to_json_assign_pattern_sequence_resolution",
    "from_json_assign_pattern_sequence_resolution",
    "AssignPatternFieldResolution",
    "encode_assign_pattern_field_resolution",
    "decode_assign_pattern_field_resolution",
    "to_json_assign_pattern_field_resolution",
    "from_json_assign_pattern_field_resolution",
    "AssignPatternTupleResolution",
    "encode_assign_pattern_tuple_resolution",
    "decode_assign_pattern_tuple_resolution",
    "to_json_assign_pattern_tuple_resolution",
    "from_json_assign_pattern_tuple_resolution",
    "AssignPatternObjectResolution",
    "encode_assign_pattern_object_resolution",
    "decode_assign_pattern_object_resolution",
    "to_json_assign_pattern_object_resolution",
    "from_json_assign_pattern_object_resolution",
    "AssignPatternRestResolution",
    "encode_assign_pattern_rest_resolution",
    "decode_assign_pattern_rest_resolution",
    "to_json_assign_pattern_rest_resolution",
    "from_json_assign_pattern_rest_resolution",
]
