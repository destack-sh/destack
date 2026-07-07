# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
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
    nested_bytes,
)

import destack._generated.dir.symbol.language
import destack._generated.dir.symbol.symbol
import destack._generated.dir.tree.node
import destack._generated.dir.tree.static
import destack._generated.source.file.model.module


@dataclass(frozen=True, slots=True)
class DecoratorSegment:
    """Decorator applications added by one DIR phase."""

    # the module id of the decorator segment
    module_id: destack._generated.source.file.model.module.ModuleId
    # the first decorator application id owned by this table segment
    first_application_id: int
    # decorator applications owned by this segment
    applications: Sequence[DecoratorApplication]
    # decorator applications attached to each owner
    applications_by_owner: Mapping[
        destack._generated.dir.tree.node.GlobalNodeIdAny, Sequence[LocalDecoratorId]
    ]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_decorator_segment(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DecoratorSegment:
        """Decode one DecoratorSegment."""
        return decode_decorator_segment(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_decorator_segment(self)

    @classmethod
    def from_json(cls, value: Json) -> DecoratorSegment:
        """Return one DecoratorSegment from one JSON value."""
        return from_json_decorator_segment(value)


def encode_decorator_segment(writer: BinaryWriter, value: DecoratorSegment) -> None:
    """Encode one DecoratorSegment."""
    destack._generated.source.file.model.module.encode_module_id(
        writer, value.module_id
    )
    writer.write_unsigned(value.first_application_id)
    writer.write_unsigned(len(value.applications))
    for item_value_applications_0 in value.applications:
        encode_decorator_application(writer, item_value_applications_0)
    entries_value_applications_by_owner_0 = []
    for (
        key_value_applications_by_owner_0,
        item_value_applications_by_owner_0,
    ) in value.applications_by_owner.items():

        def write_key_value_applications_by_owner_0(writer: BinaryWriter) -> None:
            destack._generated.dir.tree.node.encode_global_node_id_any(
                writer, key_value_applications_by_owner_0
            )

        key_bytes = nested_bytes(write_key_value_applications_by_owner_0)
        entries_value_applications_by_owner_0.append(
            (
                key_value_applications_by_owner_0,
                item_value_applications_by_owner_0,
                key_bytes,
            )
        )
    entries_value_applications_by_owner_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_applications_by_owner_0))
    for entry_value_applications_by_owner_0 in entries_value_applications_by_owner_0:
        destack._generated.dir.tree.node.encode_global_node_id_any(
            writer, entry_value_applications_by_owner_0[0]
        )
        writer.write_unsigned(len(entry_value_applications_by_owner_0[1]))
        for (
            item_entry_value_applications_by_owner_0_1_1
        ) in entry_value_applications_by_owner_0[1]:
            encode_local_decorator_id(
                writer, item_entry_value_applications_by_owner_0_1_1
            )


def decode_decorator_segment(reader: BinaryReader) -> DecoratorSegment:
    """Decode one DecoratorSegment."""
    module_id = destack._generated.source.file.model.module.decode_module_id(reader)
    first_application_id = reader.read_number()
    applications = [
        decode_decorator_application(reader) for _ in range(reader.read_number())
    ]
    applications_by_owner = {
        destack._generated.dir.tree.node.decode_global_node_id_any(reader): [
            decode_local_decorator_id(reader) for _ in range(reader.read_number())
        ]
        for _ in range(reader.read_number())
    }

    return DecoratorSegment(
        module_id=module_id,
        first_application_id=first_application_id,
        applications=applications,
        applications_by_owner=applications_by_owner,
    )


def to_json_decorator_segment(value: DecoratorSegment) -> Json:
    """Return one JSON value for one DecoratorSegment."""
    return {
        "moduleId": destack._generated.source.file.model.module.to_json_module_id(
            value.module_id
        ),
        "firstApplicationId": value.first_application_id,
        "applications": [
            to_json_decorator_application(item_0) for item_0 in value.applications
        ],
        "applicationsByOwner": [
            [
                destack._generated.dir.tree.node.to_json_global_node_id_any(key_0),
                [to_json_local_decorator_id(item_1) for item_1 in item_0],
            ]
            for key_0, item_0 in value.applications_by_owner.items()
        ],
    }


def from_json_decorator_segment(value: Json) -> DecoratorSegment:
    """Return one DecoratorSegment from one JSON value."""
    object_ = json_object(value)

    return DecoratorSegment(
        module_id=destack._generated.source.file.model.module.from_json_module_id(
            json_field(object_, "moduleId")
        ),
        first_application_id=json_int(json_field(object_, "firstApplicationId")),
        applications=[
            from_json_decorator_application(item_0)
            for item_0 in json_array(json_field(object_, "applications"))
        ],
        applications_by_owner={
            destack._generated.dir.tree.node.from_json_global_node_id_any(key_0): [
                from_json_local_decorator_id(item_1) for item_1 in json_array(item_0)
            ]
            for key_0, item_0 in json_array(json_field(object_, "applicationsByOwner"))
        },
    )


@dataclass(frozen=True, slots=True)
class DecoratorApplication:
    """Checked decorator application attached to one owner node."""

    # the decorator node
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the node decorated by this application
    owner: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the decorator target expression
    target: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the resolved decorator target
    resolution: DecoratorResolution
    # the application arguments
    arguments: Sequence[DecoratorArgument]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_decorator_application(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DecoratorApplication:
        """Decode one DecoratorApplication."""
        return decode_decorator_application(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_decorator_application(self)

    @classmethod
    def from_json(cls, value: Json) -> DecoratorApplication:
        """Return one DecoratorApplication from one JSON value."""
        return from_json_decorator_application(value)


def encode_decorator_application(
    writer: BinaryWriter, value: DecoratorApplication
) -> None:
    """Encode one DecoratorApplication."""
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.source)
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.owner)
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.target)
    encode_decorator_resolution(writer, value.resolution)
    writer.write_unsigned(len(value.arguments))
    for item_value_arguments_0 in value.arguments:
        encode_decorator_argument(writer, item_value_arguments_0)


def decode_decorator_application(reader: BinaryReader) -> DecoratorApplication:
    """Decode one DecoratorApplication."""
    source = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    owner = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    target = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    resolution = decode_decorator_resolution(reader)
    arguments = [decode_decorator_argument(reader) for _ in range(reader.read_number())]

    return DecoratorApplication(
        source=source,
        owner=owner,
        target=target,
        resolution=resolution,
        arguments=arguments,
    )


def to_json_decorator_application(value: DecoratorApplication) -> Json:
    """Return one JSON value for one DecoratorApplication."""
    return {
        "source": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.source
        ),
        "owner": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.owner
        ),
        "target": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.target
        ),
        "resolution": to_json_decorator_resolution(value.resolution),
        "arguments": [to_json_decorator_argument(item_0) for item_0 in value.arguments],
    }


def from_json_decorator_application(value: Json) -> DecoratorApplication:
    """Return one DecoratorApplication from one JSON value."""
    object_ = json_object(value)

    return DecoratorApplication(
        source=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "source")
        ),
        owner=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "owner")
        ),
        target=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "target")
        ),
        resolution=from_json_decorator_resolution(json_field(object_, "resolution")),
        arguments=[
            from_json_decorator_argument(item_0)
            for item_0 in json_array(json_field(object_, "arguments"))
        ],
    )


@dataclass(frozen=True, slots=True)
class DecoratorResolutionLanguageItem:
    """Compiler language item decorator."""

    language_item: destack._generated.dir.symbol.language.LanguageItem
    kind: typing.Literal["languageItem"] = "languageItem"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_decorator_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_decorator_resolution(self)


@dataclass(frozen=True, slots=True)
class DecoratorResolutionSymbol:
    """User-defined decorator symbol."""

    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    kind: typing.Literal["symbol"] = "symbol"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_decorator_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_decorator_resolution(self)


@dataclass(frozen=True, slots=True)
class DecoratorResolutionUnresolved:
    """Unresolved or non-symbol decorator target."""

    kind: typing.Literal["unresolved"] = "unresolved"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_decorator_resolution(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_decorator_resolution(self)


"""Resolved decorator target."""
DecoratorResolution: typing.TypeAlias = (
    DecoratorResolutionLanguageItem
    | DecoratorResolutionSymbol
    | DecoratorResolutionUnresolved
)


def encode_decorator_resolution(
    writer: BinaryWriter, value: DecoratorResolution
) -> None:
    """Encode one DecoratorResolution."""
    if value.kind == "languageItem":
        writer.write_unsigned(0)
        destack._generated.dir.symbol.language.encode_language_item(
            writer, value.language_item
        )
    elif value.kind == "symbol":
        writer.write_unsigned(1)
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(
            writer, value.symbol
        )
    elif value.kind == "unresolved":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_decorator_resolution(reader: BinaryReader) -> DecoratorResolution:
    """Decode one DecoratorResolution."""
    variant = reader.read_number()

    if variant == 0:
        language_item = destack._generated.dir.symbol.language.decode_language_item(
            reader
        )

        return DecoratorResolutionLanguageItem(language_item=language_item)
    elif variant == 1:
        symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)

        return DecoratorResolutionSymbol(symbol=symbol)
    elif variant == 2:
        return DecoratorResolutionUnresolved()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_decorator_resolution(value: DecoratorResolution) -> Json:
    """Return one JSON value for one DecoratorResolution."""
    if value.kind == "languageItem":
        return {
            "kind": "languageItem",
            "language_item": destack._generated.dir.symbol.language.to_json_language_item(
                value.language_item
            ),
        }
    elif value.kind == "symbol":
        return {
            "kind": "symbol",
            "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
                value.symbol
            ),
        }
    elif value.kind == "unresolved":
        return {
            "kind": "unresolved",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_decorator_resolution(value: Json) -> DecoratorResolution:
    """Return one DecoratorResolution from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "languageItem":
        return DecoratorResolutionLanguageItem(
            language_item=destack._generated.dir.symbol.language.from_json_language_item(
                json_field(object_, "language_item")
            )
        )
    elif kind == "symbol":
        return DecoratorResolutionSymbol(
            symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
                json_field(object_, "symbol")
            )
        )
    elif kind == "unresolved":
        return DecoratorResolutionUnresolved()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class DecoratorArgument:
    """Checked decorator argument."""

    # the argument node
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the committed static value when one exists
    value: destack._generated.dir.tree.static.GlobalStaticId | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_decorator_argument(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DecoratorArgument:
        """Decode one DecoratorArgument."""
        return decode_decorator_argument(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_decorator_argument(self)

    @classmethod
    def from_json(cls, value: Json) -> DecoratorArgument:
        """Return one DecoratorArgument from one JSON value."""
        return from_json_decorator_argument(value)


def encode_decorator_argument(writer: BinaryWriter, value: DecoratorArgument) -> None:
    """Encode one DecoratorArgument."""
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.source)
    if value.value is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.static.encode_global_static_id(writer, value.value)


def decode_decorator_argument(reader: BinaryReader) -> DecoratorArgument:
    """Decode one DecoratorArgument."""
    source = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    value_ = reader.read_option(
        lambda: destack._generated.dir.tree.static.decode_global_static_id(reader)
    )

    return DecoratorArgument(
        source=source,
        value=value_,
    )


def to_json_decorator_argument(value: DecoratorArgument) -> Json:
    """Return one JSON value for one DecoratorArgument."""
    return {
        "source": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.source
        ),
        **(
            {}
            if value.value is None
            else {
                "value": destack._generated.dir.tree.static.to_json_global_static_id(
                    value.value
                )
            }
        ),
    }


def from_json_decorator_argument(value: Json) -> DecoratorArgument:
    """Return one DecoratorArgument from one JSON value."""
    object_ = json_object(value)

    return DecoratorArgument(
        source=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "source")
        ),
        value=json_optional(
            object_,
            "value",
            lambda value: destack._generated.dir.tree.static.from_json_global_static_id(
                value
            ),
        ),
    )


"""Unique identifier for a decorator application."""
LocalDecoratorId: typing.TypeAlias = int


def encode_local_decorator_id(writer: BinaryWriter, value: LocalDecoratorId) -> None:
    """Encode one LocalDecoratorId."""
    writer.write_unsigned(value)


def decode_local_decorator_id(reader: BinaryReader) -> LocalDecoratorId:
    """Decode one LocalDecoratorId."""
    return reader.read_number()


def to_json_local_decorator_id(value: LocalDecoratorId) -> Json:
    """Return one JSON value for one LocalDecoratorId."""
    return value


def from_json_local_decorator_id(value: Json) -> LocalDecoratorId:
    """Return one LocalDecoratorId from one JSON value."""
    return json_int(value)


__all__ = [
    "DecoratorSegment",
    "encode_decorator_segment",
    "decode_decorator_segment",
    "to_json_decorator_segment",
    "from_json_decorator_segment",
    "DecoratorApplication",
    "encode_decorator_application",
    "decode_decorator_application",
    "to_json_decorator_application",
    "from_json_decorator_application",
    "DecoratorResolution",
    "encode_decorator_resolution",
    "decode_decorator_resolution",
    "to_json_decorator_resolution",
    "from_json_decorator_resolution",
    "DecoratorResolutionLanguageItem",
    "DecoratorResolutionSymbol",
    "DecoratorResolutionUnresolved",
    "DecoratorArgument",
    "encode_decorator_argument",
    "decode_decorator_argument",
    "to_json_decorator_argument",
    "from_json_decorator_argument",
    "LocalDecoratorId",
    "encode_local_decorator_id",
    "decode_local_decorator_id",
    "to_json_local_decorator_id",
    "from_json_local_decorator_id",
]
