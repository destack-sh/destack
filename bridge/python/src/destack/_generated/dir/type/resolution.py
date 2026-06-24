# generated bridge target, do not edit

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
import destack._generated.dir.tree.literal
import destack._generated.dir.tree.node
import destack._generated.dir.tree.operator
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
    owner: destack._generated.dir.symbol.symbol.GlobalSymbolId
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
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(writer, value.owner)
    destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)


def decode_receiver_resolution(reader: BinaryReader) -> ReceiverResolution:
    """Decode one ReceiverResolution."""
    kind = decode_receiver_kind(reader)
    owner = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    ty = destack._generated.dir.type.type.decode_global_type_id(reader)

    return ReceiverResolution(
        kind=kind,
        owner=owner,
        ty=ty,
    )


def to_json_receiver_resolution(value: ReceiverResolution) -> Json:
    """Return one JSON value for one ReceiverResolution."""
    return {
        "kind": to_json_receiver_kind(value.kind),
        "owner": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.owner
        ),
        "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty),
    }


def from_json_receiver_resolution(value: Json) -> ReceiverResolution:
    """Return one ReceiverResolution from one JSON value."""
    object_ = json_object(value)

    return ReceiverResolution(
        kind=from_json_receiver_kind(json_field(object_, "kind")),
        owner=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "owner")
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
    # the selected member symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the member type applied to the matched receiver
    ty: destack._generated.dir.type.type.GlobalTypeId
    # the generic arguments of the member symbol, empty when not statically applied
    arguments: Sequence[destack._generated.dir.type.type.GlobalTypeId]

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
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(writer, value.symbol)
    destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)
    writer.write_unsigned(len(value.arguments))
    for item_value_arguments_0 in value.arguments:
        destack._generated.dir.type.type.encode_global_type_id(
            writer, item_value_arguments_0
        )


def decode_member_candidate(reader: BinaryReader) -> MemberCandidate:
    """Decode one MemberCandidate."""
    receiver = destack._generated.dir.type.type.decode_global_type_id(reader)
    symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    ty = destack._generated.dir.type.type.decode_global_type_id(reader)
    arguments = [
        destack._generated.dir.type.type.decode_global_type_id(reader)
        for _ in range(reader.read_number())
    ]

    return MemberCandidate(
        receiver=receiver,
        symbol=symbol,
        ty=ty,
        arguments=arguments,
    )


def to_json_member_candidate(value: MemberCandidate) -> Json:
    """Return one JSON value for one MemberCandidate."""
    return {
        "receiver": destack._generated.dir.type.type.to_json_global_type_id(
            value.receiver
        ),
        "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.symbol
        ),
        "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty),
        "arguments": [
            destack._generated.dir.type.type.to_json_global_type_id(item_0)
            for item_0 in value.arguments
        ],
    }


def from_json_member_candidate(value: Json) -> MemberCandidate:
    """Return one MemberCandidate from one JSON value."""
    object_ = json_object(value)

    return MemberCandidate(
        receiver=destack._generated.dir.type.type.from_json_global_type_id(
            json_field(object_, "receiver")
        ),
        symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "symbol")
        ),
        ty=destack._generated.dir.type.type.from_json_global_type_id(
            json_field(object_, "ty")
        ),
        arguments=[
            destack._generated.dir.type.type.from_json_global_type_id(item_0)
            for item_0 in json_array(json_field(object_, "arguments"))
        ],
    )


@dataclass(frozen=True, slots=True)
class CallResolution:
    """Callable selected at a call site."""

    # the selected callable target
    target: CallTarget
    # the dynamic parameter types after static substitutions
    parameters: Sequence[destack._generated.dir.type.type.GlobalTypeId]
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
    writer.write_unsigned(len(value.parameters))
    for item_value_parameters_0 in value.parameters:
        destack._generated.dir.type.type.encode_global_type_id(
            writer, item_value_parameters_0
        )
    destack._generated.dir.type.type.encode_global_type_id(writer, value.return_type)


def decode_call_resolution(reader: BinaryReader) -> CallResolution:
    """Decode one CallResolution."""
    target = decode_call_target(reader)
    parameters = [
        destack._generated.dir.type.type.decode_global_type_id(reader)
        for _ in range(reader.read_number())
    ]
    return_type = destack._generated.dir.type.type.decode_global_type_id(reader)

    return CallResolution(
        target=target,
        parameters=parameters,
        return_type=return_type,
    )


def to_json_call_resolution(value: CallResolution) -> Json:
    """Return one JSON value for one CallResolution."""
    return {
        "target": to_json_call_target(value.target),
        "parameters": [
            destack._generated.dir.type.type.to_json_global_type_id(item_0)
            for item_0 in value.parameters
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
        parameters=[
            destack._generated.dir.type.type.from_json_global_type_id(item_0)
            for item_0 in json_array(json_field(object_, "parameters"))
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

    # the generic arguments of the callable value, empty when not statically applied
    arguments: Sequence[destack._generated.dir.type.type.GlobalTypeId]
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
        writer.write_unsigned(len(value.arguments))
        for item_value_arguments_0 in value.arguments:
            destack._generated.dir.type.type.encode_global_type_id(
                writer, item_value_arguments_0
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
        arguments = [
            destack._generated.dir.type.type.decode_global_type_id(reader)
            for _ in range(reader.read_number())
        ]

        return CallTargetExpression(
            arguments=arguments,
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
            "arguments": [
                destack._generated.dir.type.type.to_json_global_type_id(item_0)
                for item_0 in value.arguments
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
            arguments=[
                destack._generated.dir.type.type.from_json_global_type_id(item_0)
                for item_0 in json_array(json_field(object_, "arguments"))
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
    # the selected callable symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the generic arguments of the callable symbol, empty when not statically applied
    arguments: Sequence[destack._generated.dir.type.type.GlobalTypeId]

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
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(writer, value.symbol)
    writer.write_unsigned(len(value.arguments))
    for item_value_arguments_0 in value.arguments:
        destack._generated.dir.type.type.encode_global_type_id(
            writer, item_value_arguments_0
        )


def decode_call_candidate(reader: BinaryReader) -> CallCandidate:
    """Decode one CallCandidate."""
    receiver = reader.read_option(
        lambda: destack._generated.dir.type.type.decode_global_type_id(reader)
    )
    symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    arguments = [
        destack._generated.dir.type.type.decode_global_type_id(reader)
        for _ in range(reader.read_number())
    ]

    return CallCandidate(
        receiver=receiver,
        symbol=symbol,
        arguments=arguments,
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
        "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.symbol
        ),
        "arguments": [
            destack._generated.dir.type.type.to_json_global_type_id(item_0)
            for item_0 in value.arguments
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
        symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "symbol")
        ),
        arguments=[
            destack._generated.dir.type.type.from_json_global_type_id(item_0)
            for item_0 in json_array(json_field(object_, "arguments"))
        ],
    )


@dataclass(frozen=True, slots=True)
class ReadWriteResolution:
    """Paired accessor calls for one place read and written together."""

    # the read accessor call
    read: CallResolution
    # the write accessor call
    write: CallResolution

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_read_write_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ReadWriteResolution:
        """Decode one ReadWriteResolution."""
        return decode_read_write_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_read_write_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> ReadWriteResolution:
        """Return one ReadWriteResolution from one JSON value."""
        return from_json_read_write_resolution(value)


def encode_read_write_resolution(
    writer: BinaryWriter, value: ReadWriteResolution
) -> None:
    """Encode one ReadWriteResolution."""
    encode_call_resolution(writer, value.read)
    encode_call_resolution(writer, value.write)


def decode_read_write_resolution(reader: BinaryReader) -> ReadWriteResolution:
    """Decode one ReadWriteResolution."""
    read = decode_call_resolution(reader)
    write = decode_call_resolution(reader)

    return ReadWriteResolution(
        read=read,
        write=write,
    )


def to_json_read_write_resolution(value: ReadWriteResolution) -> Json:
    """Return one JSON value for one ReadWriteResolution."""
    return {
        "read": to_json_call_resolution(value.read),
        "write": to_json_call_resolution(value.write),
    }


def from_json_read_write_resolution(value: Json) -> ReadWriteResolution:
    """Return one ReadWriteResolution from one JSON value."""
    object_ = json_object(value)

    return ReadWriteResolution(
        read=from_json_call_resolution(json_field(object_, "read")),
        write=from_json_call_resolution(json_field(object_, "write")),
    )


@dataclass(frozen=True, slots=True)
class ConstructResolution:
    """Construct expression selected at a usage site."""

    # the selected construct target
    target: ConstructTarget
    # the dynamic parameter types after static substitutions
    parameters: Sequence[destack._generated.dir.type.type.GlobalTypeId]
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
    destack._generated.dir.type.type.encode_global_type_id(writer, value.return_type)


def decode_construct_resolution(reader: BinaryReader) -> ConstructResolution:
    """Decode one ConstructResolution."""
    target = decode_construct_target(reader)
    parameters = [
        destack._generated.dir.type.type.decode_global_type_id(reader)
        for _ in range(reader.read_number())
    ]
    return_type = destack._generated.dir.type.type.decode_global_type_id(reader)

    return ConstructResolution(
        target=target,
        parameters=parameters,
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


"""Construct target selected at a usage site."""
ConstructTarget: typing.TypeAlias = ConstructTargetClass | ConstructTargetNewtype


def encode_construct_target(writer: BinaryWriter, value: ConstructTarget) -> None:
    """Encode one ConstructTarget."""
    if value.kind == "class":
        writer.write_unsigned(0)
        encode_class_construct_candidate(writer, value.class_)
    elif value.kind == "newtype":
        writer.write_unsigned(1)
        encode_newtype_construct_candidate(writer, value.newtype)
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
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class ClassConstructCandidate:
    """One class construction candidate after overload selection."""

    # the selected class symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the selected explicit constructor symbol, when declared
    constructor: destack._generated.dir.symbol.symbol.GlobalSymbolId | None
    # the generic arguments of the class symbol, empty when not statically applied
    arguments: Sequence[destack._generated.dir.type.type.GlobalTypeId]

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
    if value.constructor is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(
            writer, value.constructor
        )
    writer.write_unsigned(len(value.arguments))
    for item_value_arguments_0 in value.arguments:
        destack._generated.dir.type.type.encode_global_type_id(
            writer, item_value_arguments_0
        )


def decode_class_construct_candidate(reader: BinaryReader) -> ClassConstructCandidate:
    """Decode one ClassConstructCandidate."""
    symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    constructor = reader.read_option(
        lambda: destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    )
    arguments = [
        destack._generated.dir.type.type.decode_global_type_id(reader)
        for _ in range(reader.read_number())
    ]

    return ClassConstructCandidate(
        symbol=symbol,
        constructor=constructor,
        arguments=arguments,
    )


def to_json_class_construct_candidate(value: ClassConstructCandidate) -> Json:
    """Return one JSON value for one ClassConstructCandidate."""
    return {
        "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.symbol
        ),
        **(
            {}
            if value.constructor is None
            else {
                "constructor": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
                    value.constructor
                )
            }
        ),
        "arguments": [
            destack._generated.dir.type.type.to_json_global_type_id(item_0)
            for item_0 in value.arguments
        ],
    }


def from_json_class_construct_candidate(value: Json) -> ClassConstructCandidate:
    """Return one ClassConstructCandidate from one JSON value."""
    object_ = json_object(value)

    return ClassConstructCandidate(
        symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "symbol")
        ),
        constructor=json_optional(
            object_,
            "constructor",
            lambda value: (
                destack._generated.dir.symbol.symbol.from_json_global_symbol_id(value)
            ),
        ),
        arguments=[
            destack._generated.dir.type.type.from_json_global_type_id(item_0)
            for item_0 in json_array(json_field(object_, "arguments"))
        ],
    )


@dataclass(frozen=True, slots=True)
class NewtypeConstructCandidate:
    """One newtype construction candidate after overload selection."""

    # the selected newtype symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the generic arguments of the newtype symbol, empty when not statically applied
    arguments: Sequence[destack._generated.dir.type.type.GlobalTypeId]

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
    writer.write_unsigned(len(value.arguments))
    for item_value_arguments_0 in value.arguments:
        destack._generated.dir.type.type.encode_global_type_id(
            writer, item_value_arguments_0
        )


def decode_newtype_construct_candidate(
    reader: BinaryReader,
) -> NewtypeConstructCandidate:
    """Decode one NewtypeConstructCandidate."""
    symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    arguments = [
        destack._generated.dir.type.type.decode_global_type_id(reader)
        for _ in range(reader.read_number())
    ]

    return NewtypeConstructCandidate(
        symbol=symbol,
        arguments=arguments,
    )


def to_json_newtype_construct_candidate(value: NewtypeConstructCandidate) -> Json:
    """Return one JSON value for one NewtypeConstructCandidate."""
    return {
        "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.symbol
        ),
        "arguments": [
            destack._generated.dir.type.type.to_json_global_type_id(item_0)
            for item_0 in value.arguments
        ],
    }


def from_json_newtype_construct_candidate(value: Json) -> NewtypeConstructCandidate:
    """Return one NewtypeConstructCandidate from one JSON value."""
    object_ = json_object(value)

    return NewtypeConstructCandidate(
        symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "symbol")
        ),
        arguments=[
            destack._generated.dir.type.type.from_json_global_type_id(item_0)
            for item_0 in json_array(json_field(object_, "arguments"))
        ],
    )


@dataclass(frozen=True, slots=True)
class PatternResolutionWildcard:
    """Pattern that accepts the input without binding, like `_`."""

    kind: typing.Literal["wildcard"] = "wildcard"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_resolution(self)


@dataclass(frozen=True, slots=True)
class PatternResolutionBinding:
    """Pattern that binds a symbol, like `value`."""

    binding: PatternBindingResolution
    kind: typing.Literal["binding"] = "binding"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_resolution(self)


@dataclass(frozen=True, slots=True)
class PatternResolutionLiteral:
    """Pattern that accepts one static literal value, like `"ok"` or `0`."""

    literal: PatternLiteralResolution
    kind: typing.Literal["literal"] = "literal"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_resolution(self)


@dataclass(frozen=True, slots=True)
class PatternResolutionRange:
    """Pattern that accepts one scalar interval, like `0..10` or `..=255`."""

    range: PatternRangeResolution
    kind: typing.Literal["range"] = "range"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_resolution(self)


@dataclass(frozen=True, slots=True)
class PatternResolutionTuple:
    """Pattern that destructures a tuple-shaped input, like `(x, y)`."""

    tuple: PatternTupleResolution
    kind: typing.Literal["tuple"] = "tuple"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_resolution(self)


@dataclass(frozen=True, slots=True)
class PatternResolutionSequence:
    """Pattern that destructures an ordered collection, like `[head, ...tail]`."""

    sequence: PatternSequenceResolution
    kind: typing.Literal["sequence"] = "sequence"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_resolution(self)


@dataclass(frozen=True, slots=True)
class PatternResolutionShape:
    """Pattern that destructures a structural input, like `{ kind: "ok", value }`."""

    shape: PatternShapeResolution
    kind: typing.Literal["shape"] = "shape"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_resolution(self)


@dataclass(frozen=True, slots=True)
class PatternResolutionNominal:
    """Pattern that destructures a symbol-backed nominal input, like `Point { x, y }`."""

    nominal: PatternNominalResolution
    kind: typing.Literal["nominal"] = "nominal"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_resolution(self)


@dataclass(frozen=True, slots=True)
class PatternResolutionNewtype:
    """Pattern that unwraps a symbol-backed newtype input, like `UserId(value)`."""

    newtype: PatternNewtypeResolution
    kind: typing.Literal["newtype"] = "newtype"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_resolution(self)


@dataclass(frozen=True, slots=True)
class PatternResolutionVariant:
    """Pattern that selects a symbol-backed variant input, like `State.Ready`."""

    variant: PatternVariantResolution
    kind: typing.Literal["variant"] = "variant"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_resolution(self)


@dataclass(frozen=True, slots=True)
class PatternResolutionUnion:
    """Pattern that accepts one of several alternatives, like `0 | 1 | 2`."""

    union: PatternUnionResolution
    kind: typing.Literal["union"] = "union"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_resolution(self)


@dataclass(frozen=True, slots=True)
class PatternResolutionBorrow:
    """Pattern that borrows the input before matching, like `&readonly value`."""

    borrow: PatternBorrowResolution
    kind: typing.Literal["borrow"] = "borrow"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_resolution(self)


@dataclass(frozen=True, slots=True)
class PatternResolutionMove:
    """Pattern that moves the input before matching, like `^value`."""

    move_file: PatternMoveResolution
    kind: typing.Literal["move"] = "move"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_resolution(self)


@dataclass(frozen=True, slots=True)
class PatternResolutionDereference:
    """Pattern that dereferences the input before matching, like `*Point { x, y }`."""

    dereference: PatternDereferenceResolution
    kind: typing.Literal["dereference"] = "dereference"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_resolution(self)


"""Pattern meaning selected during checking."""
PatternResolution: typing.TypeAlias = (
    PatternResolutionWildcard
    | PatternResolutionBinding
    | PatternResolutionLiteral
    | PatternResolutionRange
    | PatternResolutionTuple
    | PatternResolutionSequence
    | PatternResolutionShape
    | PatternResolutionNominal
    | PatternResolutionNewtype
    | PatternResolutionVariant
    | PatternResolutionUnion
    | PatternResolutionBorrow
    | PatternResolutionMove
    | PatternResolutionDereference
)


def encode_pattern_resolution(writer: BinaryWriter, value: PatternResolution) -> None:
    """Encode one PatternResolution."""
    if value.kind == "wildcard":
        writer.write_unsigned(0)
    elif value.kind == "binding":
        writer.write_unsigned(1)
        encode_pattern_binding_resolution(writer, value.binding)
    elif value.kind == "literal":
        writer.write_unsigned(2)
        encode_pattern_literal_resolution(writer, value.literal)
    elif value.kind == "range":
        writer.write_unsigned(3)
        encode_pattern_range_resolution(writer, value.range)
    elif value.kind == "tuple":
        writer.write_unsigned(4)
        encode_pattern_tuple_resolution(writer, value.tuple)
    elif value.kind == "sequence":
        writer.write_unsigned(5)
        encode_pattern_sequence_resolution(writer, value.sequence)
    elif value.kind == "shape":
        writer.write_unsigned(6)
        encode_pattern_shape_resolution(writer, value.shape)
    elif value.kind == "nominal":
        writer.write_unsigned(7)
        encode_pattern_nominal_resolution(writer, value.nominal)
    elif value.kind == "newtype":
        writer.write_unsigned(8)
        encode_pattern_newtype_resolution(writer, value.newtype)
    elif value.kind == "variant":
        writer.write_unsigned(9)
        encode_pattern_variant_resolution(writer, value.variant)
    elif value.kind == "union":
        writer.write_unsigned(10)
        encode_pattern_union_resolution(writer, value.union)
    elif value.kind == "borrow":
        writer.write_unsigned(11)
        encode_pattern_borrow_resolution(writer, value.borrow)
    elif value.kind == "move":
        writer.write_unsigned(12)
        encode_pattern_move_resolution(writer, value.move_file)
    elif value.kind == "dereference":
        writer.write_unsigned(13)
        encode_pattern_dereference_resolution(writer, value.dereference)
    else:
        raise SerdeError("unknown enum variant")


def decode_pattern_resolution(reader: BinaryReader) -> PatternResolution:
    """Decode one PatternResolution."""
    variant = reader.read_number()

    if variant == 0:
        return PatternResolutionWildcard()
    elif variant == 1:
        binding = decode_pattern_binding_resolution(reader)

        return PatternResolutionBinding(binding=binding)
    elif variant == 2:
        literal = decode_pattern_literal_resolution(reader)

        return PatternResolutionLiteral(literal=literal)
    elif variant == 3:
        range_ = decode_pattern_range_resolution(reader)

        return PatternResolutionRange(range=range_)
    elif variant == 4:
        tuple = decode_pattern_tuple_resolution(reader)

        return PatternResolutionTuple(tuple=tuple)
    elif variant == 5:
        sequence = decode_pattern_sequence_resolution(reader)

        return PatternResolutionSequence(sequence=sequence)
    elif variant == 6:
        shape = decode_pattern_shape_resolution(reader)

        return PatternResolutionShape(shape=shape)
    elif variant == 7:
        nominal = decode_pattern_nominal_resolution(reader)

        return PatternResolutionNominal(nominal=nominal)
    elif variant == 8:
        newtype = decode_pattern_newtype_resolution(reader)

        return PatternResolutionNewtype(newtype=newtype)
    elif variant == 9:
        variant = decode_pattern_variant_resolution(reader)

        return PatternResolutionVariant(variant=variant)
    elif variant == 10:
        union = decode_pattern_union_resolution(reader)

        return PatternResolutionUnion(union=union)
    elif variant == 11:
        borrow = decode_pattern_borrow_resolution(reader)

        return PatternResolutionBorrow(borrow=borrow)
    elif variant == 12:
        move_file = decode_pattern_move_resolution(reader)

        return PatternResolutionMove(move_file=move_file)
    elif variant == 13:
        dereference = decode_pattern_dereference_resolution(reader)

        return PatternResolutionDereference(dereference=dereference)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_pattern_resolution(value: PatternResolution) -> Json:
    """Return one JSON value for one PatternResolution."""
    if value.kind == "wildcard":
        return {
            "kind": "wildcard",
        }
    elif value.kind == "binding":
        return {
            "kind": "binding",
            "binding": to_json_pattern_binding_resolution(value.binding),
        }
    elif value.kind == "literal":
        return {
            "kind": "literal",
            "literal": to_json_pattern_literal_resolution(value.literal),
        }
    elif value.kind == "range":
        return {
            "kind": "range",
            "range": to_json_pattern_range_resolution(value.range),
        }
    elif value.kind == "tuple":
        return {
            "kind": "tuple",
            "tuple": to_json_pattern_tuple_resolution(value.tuple),
        }
    elif value.kind == "sequence":
        return {
            "kind": "sequence",
            "sequence": to_json_pattern_sequence_resolution(value.sequence),
        }
    elif value.kind == "shape":
        return {
            "kind": "shape",
            "shape": to_json_pattern_shape_resolution(value.shape),
        }
    elif value.kind == "nominal":
        return {
            "kind": "nominal",
            "nominal": to_json_pattern_nominal_resolution(value.nominal),
        }
    elif value.kind == "newtype":
        return {
            "kind": "newtype",
            "newtype": to_json_pattern_newtype_resolution(value.newtype),
        }
    elif value.kind == "variant":
        return {
            "kind": "variant",
            "variant": to_json_pattern_variant_resolution(value.variant),
        }
    elif value.kind == "union":
        return {
            "kind": "union",
            "union": to_json_pattern_union_resolution(value.union),
        }
    elif value.kind == "borrow":
        return {
            "kind": "borrow",
            "borrow": to_json_pattern_borrow_resolution(value.borrow),
        }
    elif value.kind == "move":
        return {
            "kind": "move",
            "move_file": to_json_pattern_move_resolution(value.move_file),
        }
    elif value.kind == "dereference":
        return {
            "kind": "dereference",
            "dereference": to_json_pattern_dereference_resolution(value.dereference),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_pattern_resolution(value: Json) -> PatternResolution:
    """Return one PatternResolution from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "wildcard":
        return PatternResolutionWildcard()
    elif kind == "binding":
        return PatternResolutionBinding(
            binding=from_json_pattern_binding_resolution(json_field(object_, "binding"))
        )
    elif kind == "literal":
        return PatternResolutionLiteral(
            literal=from_json_pattern_literal_resolution(json_field(object_, "literal"))
        )
    elif kind == "range":
        return PatternResolutionRange(
            range=from_json_pattern_range_resolution(json_field(object_, "range"))
        )
    elif kind == "tuple":
        return PatternResolutionTuple(
            tuple=from_json_pattern_tuple_resolution(json_field(object_, "tuple"))
        )
    elif kind == "sequence":
        return PatternResolutionSequence(
            sequence=from_json_pattern_sequence_resolution(
                json_field(object_, "sequence")
            )
        )
    elif kind == "shape":
        return PatternResolutionShape(
            shape=from_json_pattern_shape_resolution(json_field(object_, "shape"))
        )
    elif kind == "nominal":
        return PatternResolutionNominal(
            nominal=from_json_pattern_nominal_resolution(json_field(object_, "nominal"))
        )
    elif kind == "newtype":
        return PatternResolutionNewtype(
            newtype=from_json_pattern_newtype_resolution(json_field(object_, "newtype"))
        )
    elif kind == "variant":
        return PatternResolutionVariant(
            variant=from_json_pattern_variant_resolution(json_field(object_, "variant"))
        )
    elif kind == "union":
        return PatternResolutionUnion(
            union=from_json_pattern_union_resolution(json_field(object_, "union"))
        )
    elif kind == "borrow":
        return PatternResolutionBorrow(
            borrow=from_json_pattern_borrow_resolution(json_field(object_, "borrow"))
        )
    elif kind == "move":
        return PatternResolutionMove(
            move_file=from_json_pattern_move_resolution(
                json_field(object_, "move_file")
            )
        )
    elif kind == "dereference":
        return PatternResolutionDereference(
            dereference=from_json_pattern_dereference_resolution(
                json_field(object_, "dereference")
            )
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
class PatternLiteralResolution:
    """Static literal selected by one pattern."""

    # the committed literal value
    value: destack._generated.dir.tree.literal.ScalarLiteral

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_literal_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternLiteralResolution:
        """Decode one PatternLiteralResolution."""
        return decode_pattern_literal_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_literal_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> PatternLiteralResolution:
        """Return one PatternLiteralResolution from one JSON value."""
        return from_json_pattern_literal_resolution(value)


def encode_pattern_literal_resolution(
    writer: BinaryWriter, value: PatternLiteralResolution
) -> None:
    """Encode one PatternLiteralResolution."""
    destack._generated.dir.tree.literal.encode_scalar_literal(writer, value.value)


def decode_pattern_literal_resolution(reader: BinaryReader) -> PatternLiteralResolution:
    """Decode one PatternLiteralResolution."""
    value_ = destack._generated.dir.tree.literal.decode_scalar_literal(reader)

    return PatternLiteralResolution(
        value=value_,
    )


def to_json_pattern_literal_resolution(value: PatternLiteralResolution) -> Json:
    """Return one JSON value for one PatternLiteralResolution."""
    return {
        "value": destack._generated.dir.tree.literal.to_json_scalar_literal(
            value.value
        ),
    }


def from_json_pattern_literal_resolution(value: Json) -> PatternLiteralResolution:
    """Return one PatternLiteralResolution from one JSON value."""
    object_ = json_object(value)

    return PatternLiteralResolution(
        value=destack._generated.dir.tree.literal.from_json_scalar_literal(
            json_field(object_, "value")
        ),
    )


@dataclass(frozen=True, slots=True)
class PatternRangeResolution:
    """Scalar range selected by one pattern."""

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
        encode_pattern_range_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternRangeResolution:
        """Decode one PatternRangeResolution."""
        return decode_pattern_range_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_range_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> PatternRangeResolution:
        """Return one PatternRangeResolution from one JSON value."""
        return from_json_pattern_range_resolution(value)


def encode_pattern_range_resolution(
    writer: BinaryWriter, value: PatternRangeResolution
) -> None:
    """Encode one PatternRangeResolution."""
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


def decode_pattern_range_resolution(reader: BinaryReader) -> PatternRangeResolution:
    """Decode one PatternRangeResolution."""
    domain = destack._generated.dir.type.type.decode_global_type_id(reader)
    start = reader.read_option(
        lambda: destack._generated.dir.tree.literal.decode_scalar_literal(reader)
    )
    end = reader.read_option(
        lambda: destack._generated.dir.tree.literal.decode_scalar_literal(reader)
    )
    end_bound = destack._generated.dir.tree.operator.decode_range_end(reader)

    return PatternRangeResolution(
        domain=domain,
        start=start,
        end=end,
        end_bound=end_bound,
    )


def to_json_pattern_range_resolution(value: PatternRangeResolution) -> Json:
    """Return one JSON value for one PatternRangeResolution."""
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


def from_json_pattern_range_resolution(value: Json) -> PatternRangeResolution:
    """Return one PatternRangeResolution from one JSON value."""
    object_ = json_object(value)

    return PatternRangeResolution(
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
class PatternTupleResolution:
    """Tuple fields selected by one pattern."""

    # the tuple field mapping in source order
    fields: Sequence[PatternFieldResolution]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_tuple_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternTupleResolution:
        """Decode one PatternTupleResolution."""
        return decode_pattern_tuple_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_tuple_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> PatternTupleResolution:
        """Return one PatternTupleResolution from one JSON value."""
        return from_json_pattern_tuple_resolution(value)


def encode_pattern_tuple_resolution(
    writer: BinaryWriter, value: PatternTupleResolution
) -> None:
    """Encode one PatternTupleResolution."""
    writer.write_unsigned(len(value.fields))
    for item_value_fields_0 in value.fields:
        encode_pattern_field_resolution(writer, item_value_fields_0)


def decode_pattern_tuple_resolution(reader: BinaryReader) -> PatternTupleResolution:
    """Decode one PatternTupleResolution."""
    fields = [
        decode_pattern_field_resolution(reader) for _ in range(reader.read_number())
    ]

    return PatternTupleResolution(
        fields=fields,
    )


def to_json_pattern_tuple_resolution(value: PatternTupleResolution) -> Json:
    """Return one JSON value for one PatternTupleResolution."""
    return {
        "fields": [to_json_pattern_field_resolution(item_0) for item_0 in value.fields],
    }


def from_json_pattern_tuple_resolution(value: Json) -> PatternTupleResolution:
    """Return one PatternTupleResolution from one JSON value."""
    object_ = json_object(value)

    return PatternTupleResolution(
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
    # the selected field target
    target: PatternFieldTarget
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
    encode_pattern_field_target(writer, value.target)
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
    target = decode_pattern_field_target(reader)
    pattern = reader.read_option(
        lambda: destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    )

    return PatternFieldResolution(
        source=source,
        target=target,
        pattern=pattern,
    )


def to_json_pattern_field_resolution(value: PatternFieldResolution) -> Json:
    """Return one JSON value for one PatternFieldResolution."""
    return {
        "source": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.source
        ),
        "target": to_json_pattern_field_target(value.target),
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
        target=from_json_pattern_field_target(json_field(object_, "target")),
        pattern=json_optional(
            object_,
            "pattern",
            lambda value: destack._generated.dir.tree.node.from_json_global_node_id_any(
                value
            ),
        ),
    )


@dataclass(frozen=True, slots=True)
class PatternFieldTargetKey:
    """Named or symbolic field target."""

    key: destack._generated.dir.symbol.key.StaticKey
    kind: typing.Literal["key"] = "key"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_field_target(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_field_target(self)


@dataclass(frozen=True, slots=True)
class PatternFieldTargetIndex:
    """Positional field target."""

    index: int
    kind: typing.Literal["index"] = "index"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_field_target(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_field_target(self)


"""Field target selected by one destructuring pattern."""
PatternFieldTarget: typing.TypeAlias = PatternFieldTargetKey | PatternFieldTargetIndex


def encode_pattern_field_target(
    writer: BinaryWriter, value: PatternFieldTarget
) -> None:
    """Encode one PatternFieldTarget."""
    if value.kind == "key":
        writer.write_unsigned(0)
        destack._generated.dir.symbol.key.encode_static_key(writer, value.key)
    elif value.kind == "index":
        writer.write_unsigned(1)
        writer.write_unsigned(value.index)
    else:
        raise SerdeError("unknown enum variant")


def decode_pattern_field_target(reader: BinaryReader) -> PatternFieldTarget:
    """Decode one PatternFieldTarget."""
    variant = reader.read_number()

    if variant == 0:
        key = destack._generated.dir.symbol.key.decode_static_key(reader)

        return PatternFieldTargetKey(key=key)
    elif variant == 1:
        index = reader.read_number()

        return PatternFieldTargetIndex(index=index)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_pattern_field_target(value: PatternFieldTarget) -> Json:
    """Return one JSON value for one PatternFieldTarget."""
    if value.kind == "key":
        return {
            "kind": "key",
            "key": destack._generated.dir.symbol.key.to_json_static_key(value.key),
        }
    elif value.kind == "index":
        return {
            "kind": "index",
            "index": value.index,
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_pattern_field_target(value: Json) -> PatternFieldTarget:
    """Return one PatternFieldTarget from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "key":
        return PatternFieldTargetKey(
            key=destack._generated.dir.symbol.key.from_json_static_key(
                json_field(object_, "key")
            )
        )
    elif kind == "index":
        return PatternFieldTargetIndex(index=json_int(json_field(object_, "index")))
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class PatternSequenceResolutionArray:
    """Dynamically sized array pattern, like `[head, ...tail]` over `T[]`."""

    # the fixed prefix and suffix fields
    fields: Sequence[PatternFieldResolution]
    # the rest field, when present
    rest: PatternRestResolution | None
    kind: typing.Literal["array"] = "array"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_sequence_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_sequence_resolution(self)


@dataclass(frozen=True, slots=True)
class PatternSequenceResolutionSlice:
    """Borrowed slice pattern, like `[head, ...tail]` over `[T]`."""

    # the fixed prefix and suffix fields
    fields: Sequence[PatternFieldResolution]
    # the rest field, when present
    rest: PatternRestResolution | None
    kind: typing.Literal["slice"] = "slice"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_sequence_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_sequence_resolution(self)


@dataclass(frozen=True, slots=True)
class PatternSequenceResolutionFixedArray:
    """Fixed-size array pattern, like `[a, b, c]` over `[T; 3]`."""

    # the fixed element fields
    fields: Sequence[PatternFieldResolution]
    # the committed array length singleton
    length: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["fixedArray"] = "fixedArray"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_sequence_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_sequence_resolution(self)


"""Ordered collection selected by one pattern."""
PatternSequenceResolution: typing.TypeAlias = (
    PatternSequenceResolutionArray
    | PatternSequenceResolutionSlice
    | PatternSequenceResolutionFixedArray
)


def encode_pattern_sequence_resolution(
    writer: BinaryWriter, value: PatternSequenceResolution
) -> None:
    """Encode one PatternSequenceResolution."""
    if value.kind == "array":
        writer.write_unsigned(0)
        writer.write_unsigned(len(value.fields))
        for item_value_fields_0 in value.fields:
            encode_pattern_field_resolution(writer, item_value_fields_0)
        if value.rest is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            encode_pattern_rest_resolution(writer, value.rest)
    elif value.kind == "slice":
        writer.write_unsigned(1)
        writer.write_unsigned(len(value.fields))
        for item_value_fields_0 in value.fields:
            encode_pattern_field_resolution(writer, item_value_fields_0)
        if value.rest is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            encode_pattern_rest_resolution(writer, value.rest)
    elif value.kind == "fixedArray":
        writer.write_unsigned(2)
        writer.write_unsigned(len(value.fields))
        for item_value_fields_0 in value.fields:
            encode_pattern_field_resolution(writer, item_value_fields_0)
        destack._generated.dir.type.type.encode_global_type_id(writer, value.length)
    else:
        raise SerdeError("unknown enum variant")


def decode_pattern_sequence_resolution(
    reader: BinaryReader,
) -> PatternSequenceResolution:
    """Decode one PatternSequenceResolution."""
    variant = reader.read_number()

    if variant == 0:
        fields = [
            decode_pattern_field_resolution(reader) for _ in range(reader.read_number())
        ]
        rest = reader.read_option(lambda: decode_pattern_rest_resolution(reader))

        return PatternSequenceResolutionArray(
            fields=fields,
            rest=rest,
        )
    elif variant == 1:
        fields = [
            decode_pattern_field_resolution(reader) for _ in range(reader.read_number())
        ]
        rest = reader.read_option(lambda: decode_pattern_rest_resolution(reader))

        return PatternSequenceResolutionSlice(
            fields=fields,
            rest=rest,
        )
    elif variant == 2:
        fields = [
            decode_pattern_field_resolution(reader) for _ in range(reader.read_number())
        ]
        length = destack._generated.dir.type.type.decode_global_type_id(reader)

        return PatternSequenceResolutionFixedArray(
            fields=fields,
            length=length,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_pattern_sequence_resolution(value: PatternSequenceResolution) -> Json:
    """Return one JSON value for one PatternSequenceResolution."""
    if value.kind == "array":
        return {
            "kind": "array",
            "fields": [
                to_json_pattern_field_resolution(item_0) for item_0 in value.fields
            ],
            **(
                {}
                if value.rest is None
                else {"rest": to_json_pattern_rest_resolution(value.rest)}
            ),
        }
    elif value.kind == "slice":
        return {
            "kind": "slice",
            "fields": [
                to_json_pattern_field_resolution(item_0) for item_0 in value.fields
            ],
            **(
                {}
                if value.rest is None
                else {"rest": to_json_pattern_rest_resolution(value.rest)}
            ),
        }
    elif value.kind == "fixedArray":
        return {
            "kind": "fixedArray",
            "fields": [
                to_json_pattern_field_resolution(item_0) for item_0 in value.fields
            ],
            "length": destack._generated.dir.type.type.to_json_global_type_id(
                value.length
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_pattern_sequence_resolution(value: Json) -> PatternSequenceResolution:
    """Return one PatternSequenceResolution from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "array":
        return PatternSequenceResolutionArray(
            fields=[
                from_json_pattern_field_resolution(item_0)
                for item_0 in json_array(json_field(object_, "fields"))
            ],
            rest=json_optional(
                object_, "rest", lambda value: from_json_pattern_rest_resolution(value)
            ),
        )
    elif kind == "slice":
        return PatternSequenceResolutionSlice(
            fields=[
                from_json_pattern_field_resolution(item_0)
                for item_0 in json_array(json_field(object_, "fields"))
            ],
            rest=json_optional(
                object_, "rest", lambda value: from_json_pattern_rest_resolution(value)
            ),
        )
    elif kind == "fixedArray":
        return PatternSequenceResolutionFixedArray(
            fields=[
                from_json_pattern_field_resolution(item_0)
                for item_0 in json_array(json_field(object_, "fields"))
            ],
            length=destack._generated.dir.type.type.from_json_global_type_id(
                json_field(object_, "length")
            ),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class PatternRestResolution:
    """Rest field selected by one ordered pattern."""

    # the source node that introduces the rest field
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the nested pattern matched for the rest field
    pattern: destack._generated.dir.tree.node.GlobalNodeIdAny | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_rest_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternRestResolution:
        """Decode one PatternRestResolution."""
        return decode_pattern_rest_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_rest_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> PatternRestResolution:
        """Return one PatternRestResolution from one JSON value."""
        return from_json_pattern_rest_resolution(value)


def encode_pattern_rest_resolution(
    writer: BinaryWriter, value: PatternRestResolution
) -> None:
    """Encode one PatternRestResolution."""
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.source)
    if value.pattern is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.node.encode_global_node_id_any(
            writer, value.pattern
        )


def decode_pattern_rest_resolution(reader: BinaryReader) -> PatternRestResolution:
    """Decode one PatternRestResolution."""
    source = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    pattern = reader.read_option(
        lambda: destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    )

    return PatternRestResolution(
        source=source,
        pattern=pattern,
    )


def to_json_pattern_rest_resolution(value: PatternRestResolution) -> Json:
    """Return one JSON value for one PatternRestResolution."""
    return {
        "source": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.source
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


def from_json_pattern_rest_resolution(value: Json) -> PatternRestResolution:
    """Return one PatternRestResolution from one JSON value."""
    object_ = json_object(value)

    return PatternRestResolution(
        source=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "source")
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
class PatternShapeResolution:
    """Structural fields selected by one pattern."""

    # the structural field mapping in source order
    fields: Sequence[PatternFieldResolution]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_shape_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternShapeResolution:
        """Decode one PatternShapeResolution."""
        return decode_pattern_shape_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_shape_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> PatternShapeResolution:
        """Return one PatternShapeResolution from one JSON value."""
        return from_json_pattern_shape_resolution(value)


def encode_pattern_shape_resolution(
    writer: BinaryWriter, value: PatternShapeResolution
) -> None:
    """Encode one PatternShapeResolution."""
    writer.write_unsigned(len(value.fields))
    for item_value_fields_0 in value.fields:
        encode_pattern_field_resolution(writer, item_value_fields_0)


def decode_pattern_shape_resolution(reader: BinaryReader) -> PatternShapeResolution:
    """Decode one PatternShapeResolution."""
    fields = [
        decode_pattern_field_resolution(reader) for _ in range(reader.read_number())
    ]

    return PatternShapeResolution(
        fields=fields,
    )


def to_json_pattern_shape_resolution(value: PatternShapeResolution) -> Json:
    """Return one JSON value for one PatternShapeResolution."""
    return {
        "fields": [to_json_pattern_field_resolution(item_0) for item_0 in value.fields],
    }


def from_json_pattern_shape_resolution(value: Json) -> PatternShapeResolution:
    """Return one PatternShapeResolution from one JSON value."""
    object_ = json_object(value)

    return PatternShapeResolution(
        fields=[
            from_json_pattern_field_resolution(item_0)
            for item_0 in json_array(json_field(object_, "fields"))
        ],
    )


@dataclass(frozen=True, slots=True)
class PatternNominalResolution:
    """Symbol-backed nominal pattern selected during checking."""

    # the selected nominal symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the generic arguments of the nominal symbol, empty when not statically applied
    arguments: Sequence[destack._generated.dir.type.type.GlobalTypeId]
    # the nominal field mapping in source order
    fields: Sequence[PatternFieldResolution]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_nominal_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternNominalResolution:
        """Decode one PatternNominalResolution."""
        return decode_pattern_nominal_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_nominal_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> PatternNominalResolution:
        """Return one PatternNominalResolution from one JSON value."""
        return from_json_pattern_nominal_resolution(value)


def encode_pattern_nominal_resolution(
    writer: BinaryWriter, value: PatternNominalResolution
) -> None:
    """Encode one PatternNominalResolution."""
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(writer, value.symbol)
    writer.write_unsigned(len(value.arguments))
    for item_value_arguments_0 in value.arguments:
        destack._generated.dir.type.type.encode_global_type_id(
            writer, item_value_arguments_0
        )
    writer.write_unsigned(len(value.fields))
    for item_value_fields_0 in value.fields:
        encode_pattern_field_resolution(writer, item_value_fields_0)


def decode_pattern_nominal_resolution(reader: BinaryReader) -> PatternNominalResolution:
    """Decode one PatternNominalResolution."""
    symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    arguments = [
        destack._generated.dir.type.type.decode_global_type_id(reader)
        for _ in range(reader.read_number())
    ]
    fields = [
        decode_pattern_field_resolution(reader) for _ in range(reader.read_number())
    ]

    return PatternNominalResolution(
        symbol=symbol,
        arguments=arguments,
        fields=fields,
    )


def to_json_pattern_nominal_resolution(value: PatternNominalResolution) -> Json:
    """Return one JSON value for one PatternNominalResolution."""
    return {
        "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.symbol
        ),
        "arguments": [
            destack._generated.dir.type.type.to_json_global_type_id(item_0)
            for item_0 in value.arguments
        ],
        "fields": [to_json_pattern_field_resolution(item_0) for item_0 in value.fields],
    }


def from_json_pattern_nominal_resolution(value: Json) -> PatternNominalResolution:
    """Return one PatternNominalResolution from one JSON value."""
    object_ = json_object(value)

    return PatternNominalResolution(
        symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "symbol")
        ),
        arguments=[
            destack._generated.dir.type.type.from_json_global_type_id(item_0)
            for item_0 in json_array(json_field(object_, "arguments"))
        ],
        fields=[
            from_json_pattern_field_resolution(item_0)
            for item_0 in json_array(json_field(object_, "fields"))
        ],
    )


@dataclass(frozen=True, slots=True)
class PatternNewtypeResolution:
    """Symbol-backed newtype pattern selected during checking."""

    # the selected newtype symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the generic arguments of the newtype symbol, empty when not statically applied
    arguments: Sequence[destack._generated.dir.type.type.GlobalTypeId]
    # the wrapped value pattern
    value: destack._generated.dir.tree.node.GlobalNodeIdAny | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_newtype_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternNewtypeResolution:
        """Decode one PatternNewtypeResolution."""
        return decode_pattern_newtype_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_newtype_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> PatternNewtypeResolution:
        """Return one PatternNewtypeResolution from one JSON value."""
        return from_json_pattern_newtype_resolution(value)


def encode_pattern_newtype_resolution(
    writer: BinaryWriter, value: PatternNewtypeResolution
) -> None:
    """Encode one PatternNewtypeResolution."""
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(writer, value.symbol)
    writer.write_unsigned(len(value.arguments))
    for item_value_arguments_0 in value.arguments:
        destack._generated.dir.type.type.encode_global_type_id(
            writer, item_value_arguments_0
        )
    if value.value is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.value)


def decode_pattern_newtype_resolution(reader: BinaryReader) -> PatternNewtypeResolution:
    """Decode one PatternNewtypeResolution."""
    symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    arguments = [
        destack._generated.dir.type.type.decode_global_type_id(reader)
        for _ in range(reader.read_number())
    ]
    value_ = reader.read_option(
        lambda: destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    )

    return PatternNewtypeResolution(
        symbol=symbol,
        arguments=arguments,
        value=value_,
    )


def to_json_pattern_newtype_resolution(value: PatternNewtypeResolution) -> Json:
    """Return one JSON value for one PatternNewtypeResolution."""
    return {
        "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.symbol
        ),
        "arguments": [
            destack._generated.dir.type.type.to_json_global_type_id(item_0)
            for item_0 in value.arguments
        ],
        **(
            {}
            if value.value is None
            else {
                "value": destack._generated.dir.tree.node.to_json_global_node_id_any(
                    value.value
                )
            }
        ),
    }


def from_json_pattern_newtype_resolution(value: Json) -> PatternNewtypeResolution:
    """Return one PatternNewtypeResolution from one JSON value."""
    object_ = json_object(value)

    return PatternNewtypeResolution(
        symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "symbol")
        ),
        arguments=[
            destack._generated.dir.type.type.from_json_global_type_id(item_0)
            for item_0 in json_array(json_field(object_, "arguments"))
        ],
        value=json_optional(
            object_,
            "value",
            lambda value: destack._generated.dir.tree.node.from_json_global_node_id_any(
                value
            ),
        ),
    )


@dataclass(frozen=True, slots=True)
class PatternVariantResolution:
    """Symbol-backed variant pattern selected during checking."""

    # the selected variant family symbol
    owner: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the selected variant symbol
    variant: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the generic arguments of the variant symbol, empty when not statically applied
    arguments: Sequence[destack._generated.dir.type.type.GlobalTypeId]
    # the discriminant value
    discriminant: destack._generated.dir.tree.literal.ScalarLiteral
    # the variant field mapping in source order
    fields: Sequence[PatternFieldResolution]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_variant_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternVariantResolution:
        """Decode one PatternVariantResolution."""
        return decode_pattern_variant_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_variant_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> PatternVariantResolution:
        """Return one PatternVariantResolution from one JSON value."""
        return from_json_pattern_variant_resolution(value)


def encode_pattern_variant_resolution(
    writer: BinaryWriter, value: PatternVariantResolution
) -> None:
    """Encode one PatternVariantResolution."""
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(writer, value.owner)
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(writer, value.variant)
    writer.write_unsigned(len(value.arguments))
    for item_value_arguments_0 in value.arguments:
        destack._generated.dir.type.type.encode_global_type_id(
            writer, item_value_arguments_0
        )
    destack._generated.dir.tree.literal.encode_scalar_literal(
        writer, value.discriminant
    )
    writer.write_unsigned(len(value.fields))
    for item_value_fields_0 in value.fields:
        encode_pattern_field_resolution(writer, item_value_fields_0)


def decode_pattern_variant_resolution(reader: BinaryReader) -> PatternVariantResolution:
    """Decode one PatternVariantResolution."""
    owner = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    variant = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    arguments = [
        destack._generated.dir.type.type.decode_global_type_id(reader)
        for _ in range(reader.read_number())
    ]
    discriminant = destack._generated.dir.tree.literal.decode_scalar_literal(reader)
    fields = [
        decode_pattern_field_resolution(reader) for _ in range(reader.read_number())
    ]

    return PatternVariantResolution(
        owner=owner,
        variant=variant,
        arguments=arguments,
        discriminant=discriminant,
        fields=fields,
    )


def to_json_pattern_variant_resolution(value: PatternVariantResolution) -> Json:
    """Return one JSON value for one PatternVariantResolution."""
    return {
        "owner": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.owner
        ),
        "variant": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.variant
        ),
        "arguments": [
            destack._generated.dir.type.type.to_json_global_type_id(item_0)
            for item_0 in value.arguments
        ],
        "discriminant": destack._generated.dir.tree.literal.to_json_scalar_literal(
            value.discriminant
        ),
        "fields": [to_json_pattern_field_resolution(item_0) for item_0 in value.fields],
    }


def from_json_pattern_variant_resolution(value: Json) -> PatternVariantResolution:
    """Return one PatternVariantResolution from one JSON value."""
    object_ = json_object(value)

    return PatternVariantResolution(
        owner=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "owner")
        ),
        variant=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "variant")
        ),
        arguments=[
            destack._generated.dir.type.type.from_json_global_type_id(item_0)
            for item_0 in json_array(json_field(object_, "arguments"))
        ],
        discriminant=destack._generated.dir.tree.literal.from_json_scalar_literal(
            json_field(object_, "discriminant")
        ),
        fields=[
            from_json_pattern_field_resolution(item_0)
            for item_0 in json_array(json_field(object_, "fields"))
        ],
    )


@dataclass(frozen=True, slots=True)
class PatternUnionResolution:
    """Alternative patterns selected during checking."""

    # the alternative pattern nodes
    alternatives: Sequence[destack._generated.dir.tree.node.GlobalNodeIdAny]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_union_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternUnionResolution:
        """Decode one PatternUnionResolution."""
        return decode_pattern_union_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_union_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> PatternUnionResolution:
        """Return one PatternUnionResolution from one JSON value."""
        return from_json_pattern_union_resolution(value)


def encode_pattern_union_resolution(
    writer: BinaryWriter, value: PatternUnionResolution
) -> None:
    """Encode one PatternUnionResolution."""
    writer.write_unsigned(len(value.alternatives))
    for item_value_alternatives_0 in value.alternatives:
        destack._generated.dir.tree.node.encode_global_node_id_any(
            writer, item_value_alternatives_0
        )


def decode_pattern_union_resolution(reader: BinaryReader) -> PatternUnionResolution:
    """Decode one PatternUnionResolution."""
    alternatives = [
        destack._generated.dir.tree.node.decode_global_node_id_any(reader)
        for _ in range(reader.read_number())
    ]

    return PatternUnionResolution(
        alternatives=alternatives,
    )


def to_json_pattern_union_resolution(value: PatternUnionResolution) -> Json:
    """Return one JSON value for one PatternUnionResolution."""
    return {
        "alternatives": [
            destack._generated.dir.tree.node.to_json_global_node_id_any(item_0)
            for item_0 in value.alternatives
        ],
    }


def from_json_pattern_union_resolution(value: Json) -> PatternUnionResolution:
    """Return one PatternUnionResolution from one JSON value."""
    object_ = json_object(value)

    return PatternUnionResolution(
        alternatives=[
            destack._generated.dir.tree.node.from_json_global_node_id_any(item_0)
            for item_0 in json_array(json_field(object_, "alternatives"))
        ],
    )


@dataclass(frozen=True, slots=True)
class PatternBorrowResolution:
    """Borrow operation selected by one pattern."""

    # the requested borrow access, if source explicit
    access: destack._generated.dir.type.type.Access | None
    # the pattern matched through the borrow
    pattern: destack._generated.dir.tree.node.GlobalNodeIdAny

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_borrow_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternBorrowResolution:
        """Decode one PatternBorrowResolution."""
        return decode_pattern_borrow_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_borrow_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> PatternBorrowResolution:
        """Return one PatternBorrowResolution from one JSON value."""
        return from_json_pattern_borrow_resolution(value)


def encode_pattern_borrow_resolution(
    writer: BinaryWriter, value: PatternBorrowResolution
) -> None:
    """Encode one PatternBorrowResolution."""
    if value.access is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.type.type.encode_access(writer, value.access)
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.pattern)


def decode_pattern_borrow_resolution(reader: BinaryReader) -> PatternBorrowResolution:
    """Decode one PatternBorrowResolution."""
    access = reader.read_option(
        lambda: destack._generated.dir.type.type.decode_access(reader)
    )
    pattern = destack._generated.dir.tree.node.decode_global_node_id_any(reader)

    return PatternBorrowResolution(
        access=access,
        pattern=pattern,
    )


def to_json_pattern_borrow_resolution(value: PatternBorrowResolution) -> Json:
    """Return one JSON value for one PatternBorrowResolution."""
    return {
        **(
            {}
            if value.access is None
            else {
                "access": destack._generated.dir.type.type.to_json_access(value.access)
            }
        ),
        "pattern": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.pattern
        ),
    }


def from_json_pattern_borrow_resolution(value: Json) -> PatternBorrowResolution:
    """Return one PatternBorrowResolution from one JSON value."""
    object_ = json_object(value)

    return PatternBorrowResolution(
        access=json_optional(
            object_,
            "access",
            lambda value: destack._generated.dir.type.type.from_json_access(value),
        ),
        pattern=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "pattern")
        ),
    )


@dataclass(frozen=True, slots=True)
class PatternMoveResolution:
    """Move operation selected by one pattern."""

    # the requested move access, if source explicit
    access: destack._generated.dir.type.type.Access | None
    # the pattern matched after moving
    pattern: destack._generated.dir.tree.node.GlobalNodeIdAny

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_move_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternMoveResolution:
        """Decode one PatternMoveResolution."""
        return decode_pattern_move_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_move_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> PatternMoveResolution:
        """Return one PatternMoveResolution from one JSON value."""
        return from_json_pattern_move_resolution(value)


def encode_pattern_move_resolution(
    writer: BinaryWriter, value: PatternMoveResolution
) -> None:
    """Encode one PatternMoveResolution."""
    if value.access is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.type.type.encode_access(writer, value.access)
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.pattern)


def decode_pattern_move_resolution(reader: BinaryReader) -> PatternMoveResolution:
    """Decode one PatternMoveResolution."""
    access = reader.read_option(
        lambda: destack._generated.dir.type.type.decode_access(reader)
    )
    pattern = destack._generated.dir.tree.node.decode_global_node_id_any(reader)

    return PatternMoveResolution(
        access=access,
        pattern=pattern,
    )


def to_json_pattern_move_resolution(value: PatternMoveResolution) -> Json:
    """Return one JSON value for one PatternMoveResolution."""
    return {
        **(
            {}
            if value.access is None
            else {
                "access": destack._generated.dir.type.type.to_json_access(value.access)
            }
        ),
        "pattern": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.pattern
        ),
    }


def from_json_pattern_move_resolution(value: Json) -> PatternMoveResolution:
    """Return one PatternMoveResolution from one JSON value."""
    object_ = json_object(value)

    return PatternMoveResolution(
        access=json_optional(
            object_,
            "access",
            lambda value: destack._generated.dir.type.type.from_json_access(value),
        ),
        pattern=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "pattern")
        ),
    )


@dataclass(frozen=True, slots=True)
class PatternDereferenceResolution:
    """Dereference operation selected by one pattern."""

    # the pattern matched through the dereference
    pattern: destack._generated.dir.tree.node.GlobalNodeIdAny

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_pattern_dereference_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PatternDereferenceResolution:
        """Decode one PatternDereferenceResolution."""
        return decode_pattern_dereference_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_pattern_dereference_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> PatternDereferenceResolution:
        """Return one PatternDereferenceResolution from one JSON value."""
        return from_json_pattern_dereference_resolution(value)


def encode_pattern_dereference_resolution(
    writer: BinaryWriter, value: PatternDereferenceResolution
) -> None:
    """Encode one PatternDereferenceResolution."""
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.pattern)


def decode_pattern_dereference_resolution(
    reader: BinaryReader,
) -> PatternDereferenceResolution:
    """Decode one PatternDereferenceResolution."""
    pattern = destack._generated.dir.tree.node.decode_global_node_id_any(reader)

    return PatternDereferenceResolution(
        pattern=pattern,
    )


def to_json_pattern_dereference_resolution(value: PatternDereferenceResolution) -> Json:
    """Return one JSON value for one PatternDereferenceResolution."""
    return {
        "pattern": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.pattern
        ),
    }


def from_json_pattern_dereference_resolution(
    value: Json,
) -> PatternDereferenceResolution:
    """Return one PatternDereferenceResolution from one JSON value."""
    object_ = json_object(value)

    return PatternDereferenceResolution(
        pattern=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "pattern")
        ),
    )


@dataclass(frozen=True, slots=True)
class AssignPatternResolutionPlace:
    """Direct writable place target, like `value` or `object.field`."""

    place: AssignPatternPlaceResolution
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
    | AssignPatternResolutionObject
)


def encode_assign_pattern_resolution(
    writer: BinaryWriter, value: AssignPatternResolution
) -> None:
    """Encode one AssignPatternResolution."""
    if value.kind == "place":
        writer.write_unsigned(0)
        encode_assign_pattern_place_resolution(writer, value.place)
    elif value.kind == "default":
        writer.write_unsigned(1)
        encode_assign_pattern_default_resolution(writer, value.default)
    elif value.kind == "sequence":
        writer.write_unsigned(2)
        encode_assign_pattern_sequence_resolution(writer, value.sequence)
    elif value.kind == "object":
        writer.write_unsigned(3)
        encode_assign_pattern_object_resolution(writer, value.object)
    else:
        raise SerdeError("unknown enum variant")


def decode_assign_pattern_resolution(reader: BinaryReader) -> AssignPatternResolution:
    """Decode one AssignPatternResolution."""
    variant = reader.read_number()

    if variant == 0:
        place = decode_assign_pattern_place_resolution(reader)

        return AssignPatternResolutionPlace(place=place)
    elif variant == 1:
        default = decode_assign_pattern_default_resolution(reader)

        return AssignPatternResolutionDefault(default=default)
    elif variant == 2:
        sequence = decode_assign_pattern_sequence_resolution(reader)

        return AssignPatternResolutionSequence(sequence=sequence)
    elif variant == 3:
        object = decode_assign_pattern_object_resolution(reader)

        return AssignPatternResolutionObject(object=object)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_assign_pattern_resolution(value: AssignPatternResolution) -> Json:
    """Return one JSON value for one AssignPatternResolution."""
    if value.kind == "place":
        return {
            "kind": "place",
            "place": to_json_assign_pattern_place_resolution(value.place),
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
            place=from_json_assign_pattern_place_resolution(
                json_field(object_, "place")
            )
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
    elif kind == "object":
        return AssignPatternResolutionObject(
            object=from_json_assign_pattern_object_resolution(
                json_field(object_, "object")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class AssignPatternPlaceResolution:
    """Direct assignment place selected during checking."""

    # the expression node that designates the writable place
    target: destack._generated.dir.tree.node.GlobalNodeIdAny

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_assign_pattern_place_resolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> AssignPatternPlaceResolution:
        """Decode one AssignPatternPlaceResolution."""
        return decode_assign_pattern_place_resolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_assign_pattern_place_resolution(self)

    @classmethod
    def from_json(cls, value: Json) -> AssignPatternPlaceResolution:
        """Return one AssignPatternPlaceResolution from one JSON value."""
        return from_json_assign_pattern_place_resolution(value)


def encode_assign_pattern_place_resolution(
    writer: BinaryWriter, value: AssignPatternPlaceResolution
) -> None:
    """Encode one AssignPatternPlaceResolution."""
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.target)


def decode_assign_pattern_place_resolution(
    reader: BinaryReader,
) -> AssignPatternPlaceResolution:
    """Decode one AssignPatternPlaceResolution."""
    target = destack._generated.dir.tree.node.decode_global_node_id_any(reader)

    return AssignPatternPlaceResolution(
        target=target,
    )


def to_json_assign_pattern_place_resolution(
    value: AssignPatternPlaceResolution,
) -> Json:
    """Return one JSON value for one AssignPatternPlaceResolution."""
    return {
        "target": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.target
        ),
    }


def from_json_assign_pattern_place_resolution(
    value: Json,
) -> AssignPatternPlaceResolution:
    """Return one AssignPatternPlaceResolution from one JSON value."""
    object_ = json_object(value)

    return AssignPatternPlaceResolution(
        target=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "target")
        ),
    )


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

    # the fixed fields in source order
    fields: Sequence[AssignPatternFieldResolution]
    # the rest target, when present
    rest: AssignPatternRestResolution | None

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
    writer.write_unsigned(len(value.fields))
    for item_value_fields_0 in value.fields:
        encode_assign_pattern_field_resolution(writer, item_value_fields_0)
    if value.rest is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_assign_pattern_rest_resolution(writer, value.rest)


def decode_assign_pattern_sequence_resolution(
    reader: BinaryReader,
) -> AssignPatternSequenceResolution:
    """Decode one AssignPatternSequenceResolution."""
    fields = [
        decode_assign_pattern_field_resolution(reader)
        for _ in range(reader.read_number())
    ]
    rest = reader.read_option(lambda: decode_assign_pattern_rest_resolution(reader))

    return AssignPatternSequenceResolution(
        fields=fields,
        rest=rest,
    )


def to_json_assign_pattern_sequence_resolution(
    value: AssignPatternSequenceResolution,
) -> Json:
    """Return one JSON value for one AssignPatternSequenceResolution."""
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


def from_json_assign_pattern_sequence_resolution(
    value: Json,
) -> AssignPatternSequenceResolution:
    """Return one AssignPatternSequenceResolution from one JSON value."""
    object_ = json_object(value)

    return AssignPatternSequenceResolution(
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
class AssignPatternFieldResolution:
    """One destructured assignment field."""

    # the source node that introduces the field
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the selected field target
    target: PatternFieldTarget
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
    encode_pattern_field_target(writer, value.target)
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
    target = decode_pattern_field_target(reader)
    pattern = reader.read_option(
        lambda: destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    )

    return AssignPatternFieldResolution(
        source=source,
        target=target,
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
        "target": to_json_pattern_field_target(value.target),
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
        target=from_json_pattern_field_target(json_field(object_, "target")),
        pattern=json_optional(
            object_,
            "pattern",
            lambda value: destack._generated.dir.tree.node.from_json_global_node_id_any(
                value
            ),
        ),
    )


@dataclass(frozen=True, slots=True)
class AssignPatternRestResolution:
    """Rest field selected by one assignment destructuring pattern."""

    # the source node that introduces the rest field
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
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
    pattern = reader.read_option(
        lambda: destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    )

    return AssignPatternRestResolution(
        source=source,
        pattern=pattern,
    )


def to_json_assign_pattern_rest_resolution(value: AssignPatternRestResolution) -> Json:
    """Return one JSON value for one AssignPatternRestResolution."""
    return {
        "source": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.source
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
        pattern=json_optional(
            object_,
            "pattern",
            lambda value: destack._generated.dir.tree.node.from_json_global_node_id_any(
                value
            ),
        ),
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


__all__ = [
    "NameResolution",
    "encode_name_resolution",
    "decode_name_resolution",
    "to_json_name_resolution",
    "from_json_name_resolution",
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
    "ReadWriteResolution",
    "encode_read_write_resolution",
    "decode_read_write_resolution",
    "to_json_read_write_resolution",
    "from_json_read_write_resolution",
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
    "PatternResolution",
    "encode_pattern_resolution",
    "decode_pattern_resolution",
    "to_json_pattern_resolution",
    "from_json_pattern_resolution",
    "PatternResolutionWildcard",
    "PatternResolutionBinding",
    "PatternResolutionLiteral",
    "PatternResolutionRange",
    "PatternResolutionTuple",
    "PatternResolutionSequence",
    "PatternResolutionShape",
    "PatternResolutionNominal",
    "PatternResolutionNewtype",
    "PatternResolutionVariant",
    "PatternResolutionUnion",
    "PatternResolutionBorrow",
    "PatternResolutionMove",
    "PatternResolutionDereference",
    "PatternBindingResolution",
    "encode_pattern_binding_resolution",
    "decode_pattern_binding_resolution",
    "to_json_pattern_binding_resolution",
    "from_json_pattern_binding_resolution",
    "PatternLiteralResolution",
    "encode_pattern_literal_resolution",
    "decode_pattern_literal_resolution",
    "to_json_pattern_literal_resolution",
    "from_json_pattern_literal_resolution",
    "PatternRangeResolution",
    "encode_pattern_range_resolution",
    "decode_pattern_range_resolution",
    "to_json_pattern_range_resolution",
    "from_json_pattern_range_resolution",
    "PatternTupleResolution",
    "encode_pattern_tuple_resolution",
    "decode_pattern_tuple_resolution",
    "to_json_pattern_tuple_resolution",
    "from_json_pattern_tuple_resolution",
    "PatternFieldResolution",
    "encode_pattern_field_resolution",
    "decode_pattern_field_resolution",
    "to_json_pattern_field_resolution",
    "from_json_pattern_field_resolution",
    "PatternFieldTarget",
    "encode_pattern_field_target",
    "decode_pattern_field_target",
    "to_json_pattern_field_target",
    "from_json_pattern_field_target",
    "PatternFieldTargetKey",
    "PatternFieldTargetIndex",
    "PatternSequenceResolution",
    "encode_pattern_sequence_resolution",
    "decode_pattern_sequence_resolution",
    "to_json_pattern_sequence_resolution",
    "from_json_pattern_sequence_resolution",
    "PatternSequenceResolutionArray",
    "PatternSequenceResolutionSlice",
    "PatternSequenceResolutionFixedArray",
    "PatternRestResolution",
    "encode_pattern_rest_resolution",
    "decode_pattern_rest_resolution",
    "to_json_pattern_rest_resolution",
    "from_json_pattern_rest_resolution",
    "PatternShapeResolution",
    "encode_pattern_shape_resolution",
    "decode_pattern_shape_resolution",
    "to_json_pattern_shape_resolution",
    "from_json_pattern_shape_resolution",
    "PatternNominalResolution",
    "encode_pattern_nominal_resolution",
    "decode_pattern_nominal_resolution",
    "to_json_pattern_nominal_resolution",
    "from_json_pattern_nominal_resolution",
    "PatternNewtypeResolution",
    "encode_pattern_newtype_resolution",
    "decode_pattern_newtype_resolution",
    "to_json_pattern_newtype_resolution",
    "from_json_pattern_newtype_resolution",
    "PatternVariantResolution",
    "encode_pattern_variant_resolution",
    "decode_pattern_variant_resolution",
    "to_json_pattern_variant_resolution",
    "from_json_pattern_variant_resolution",
    "PatternUnionResolution",
    "encode_pattern_union_resolution",
    "decode_pattern_union_resolution",
    "to_json_pattern_union_resolution",
    "from_json_pattern_union_resolution",
    "PatternBorrowResolution",
    "encode_pattern_borrow_resolution",
    "decode_pattern_borrow_resolution",
    "to_json_pattern_borrow_resolution",
    "from_json_pattern_borrow_resolution",
    "PatternMoveResolution",
    "encode_pattern_move_resolution",
    "decode_pattern_move_resolution",
    "to_json_pattern_move_resolution",
    "from_json_pattern_move_resolution",
    "PatternDereferenceResolution",
    "encode_pattern_dereference_resolution",
    "decode_pattern_dereference_resolution",
    "to_json_pattern_dereference_resolution",
    "from_json_pattern_dereference_resolution",
    "AssignPatternResolution",
    "encode_assign_pattern_resolution",
    "decode_assign_pattern_resolution",
    "to_json_assign_pattern_resolution",
    "from_json_assign_pattern_resolution",
    "AssignPatternResolutionPlace",
    "AssignPatternResolutionDefault",
    "AssignPatternResolutionSequence",
    "AssignPatternResolutionObject",
    "AssignPatternPlaceResolution",
    "encode_assign_pattern_place_resolution",
    "decode_assign_pattern_place_resolution",
    "to_json_assign_pattern_place_resolution",
    "from_json_assign_pattern_place_resolution",
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
    "AssignPatternRestResolution",
    "encode_assign_pattern_rest_resolution",
    "decode_assign_pattern_rest_resolution",
    "to_json_assign_pattern_rest_resolution",
    "from_json_assign_pattern_rest_resolution",
    "AssignPatternObjectResolution",
    "encode_assign_pattern_object_resolution",
    "decode_assign_pattern_object_resolution",
    "to_json_assign_pattern_object_resolution",
    "from_json_assign_pattern_object_resolution",
]
