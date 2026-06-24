# generated bridge target, do not edit

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
class AnnotationSegment:
    """Annotation invocations added by one DIR phase."""

    # the module id of the annotation segment
    module_id: destack._generated.source.file.model.module.ModuleId
    # the first annotation invocation id owned by this table segment
    first_invocation_id: int
    # annotation invocations owned by this segment
    invocations: Sequence[AnnotationInvocation]
    # annotation invocations attached to each owner
    invocations_by_owner: Mapping[
        destack._generated.dir.tree.node.GlobalNodeIdAny, Sequence[LocalAnnotationId]
    ]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_annotation_segment(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> AnnotationSegment:
        """Decode one AnnotationSegment."""
        return decode_annotation_segment(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_annotation_segment(self)

    @classmethod
    def from_json(cls, value: Json) -> AnnotationSegment:
        """Return one AnnotationSegment from one JSON value."""
        return from_json_annotation_segment(value)


def encode_annotation_segment(writer: BinaryWriter, value: AnnotationSegment) -> None:
    """Encode one AnnotationSegment."""
    destack._generated.source.file.model.module.encode_module_id(
        writer, value.module_id
    )
    writer.write_unsigned(value.first_invocation_id)
    writer.write_unsigned(len(value.invocations))
    for item_value_invocations_0 in value.invocations:
        encode_annotation_invocation(writer, item_value_invocations_0)
    entries_value_invocations_by_owner_0 = []
    for (
        key_value_invocations_by_owner_0,
        item_value_invocations_by_owner_0,
    ) in value.invocations_by_owner.items():

        def write_key_value_invocations_by_owner_0(writer: BinaryWriter) -> None:
            destack._generated.dir.tree.node.encode_global_node_id_any(
                writer, key_value_invocations_by_owner_0
            )

        key_bytes = nested_bytes(write_key_value_invocations_by_owner_0)
        entries_value_invocations_by_owner_0.append(
            (
                key_value_invocations_by_owner_0,
                item_value_invocations_by_owner_0,
                key_bytes,
            )
        )
    entries_value_invocations_by_owner_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_invocations_by_owner_0))
    for entry_value_invocations_by_owner_0 in entries_value_invocations_by_owner_0:
        destack._generated.dir.tree.node.encode_global_node_id_any(
            writer, entry_value_invocations_by_owner_0[0]
        )
        writer.write_unsigned(len(entry_value_invocations_by_owner_0[1]))
        for (
            item_entry_value_invocations_by_owner_0_1_1
        ) in entry_value_invocations_by_owner_0[1]:
            encode_local_annotation_id(
                writer, item_entry_value_invocations_by_owner_0_1_1
            )


def decode_annotation_segment(reader: BinaryReader) -> AnnotationSegment:
    """Decode one AnnotationSegment."""
    module_id = destack._generated.source.file.model.module.decode_module_id(reader)
    first_invocation_id = reader.read_number()
    invocations = [
        decode_annotation_invocation(reader) for _ in range(reader.read_number())
    ]
    invocations_by_owner = {
        destack._generated.dir.tree.node.decode_global_node_id_any(reader): [
            decode_local_annotation_id(reader) for _ in range(reader.read_number())
        ]
        for _ in range(reader.read_number())
    }

    return AnnotationSegment(
        module_id=module_id,
        first_invocation_id=first_invocation_id,
        invocations=invocations,
        invocations_by_owner=invocations_by_owner,
    )


def to_json_annotation_segment(value: AnnotationSegment) -> Json:
    """Return one JSON value for one AnnotationSegment."""
    return {
        "moduleId": destack._generated.source.file.model.module.to_json_module_id(
            value.module_id
        ),
        "firstInvocationId": value.first_invocation_id,
        "invocations": [
            to_json_annotation_invocation(item_0) for item_0 in value.invocations
        ],
        "invocationsByOwner": [
            [
                destack._generated.dir.tree.node.to_json_global_node_id_any(key_0),
                [to_json_local_annotation_id(item_1) for item_1 in item_0],
            ]
            for key_0, item_0 in value.invocations_by_owner.items()
        ],
    }


def from_json_annotation_segment(value: Json) -> AnnotationSegment:
    """Return one AnnotationSegment from one JSON value."""
    object_ = json_object(value)

    return AnnotationSegment(
        module_id=destack._generated.source.file.model.module.from_json_module_id(
            json_field(object_, "moduleId")
        ),
        first_invocation_id=json_int(json_field(object_, "firstInvocationId")),
        invocations=[
            from_json_annotation_invocation(item_0)
            for item_0 in json_array(json_field(object_, "invocations"))
        ],
        invocations_by_owner={
            destack._generated.dir.tree.node.from_json_global_node_id_any(key_0): [
                from_json_local_annotation_id(item_1) for item_1 in json_array(item_0)
            ]
            for key_0, item_0 in json_array(json_field(object_, "invocationsByOwner"))
        },
    )


@dataclass(frozen=True, slots=True)
class AnnotationInvocation:
    """Checked annotation invocation attached to one owner node."""

    # the decorator node
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the node annotated by this invocation
    owner: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the decorator target expression
    target: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the resolved annotation target
    resolution: AnnotationTarget
    # the invocation arguments
    arguments: Sequence[AnnotationArgument]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_annotation_invocation(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> AnnotationInvocation:
        """Decode one AnnotationInvocation."""
        return decode_annotation_invocation(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_annotation_invocation(self)

    @classmethod
    def from_json(cls, value: Json) -> AnnotationInvocation:
        """Return one AnnotationInvocation from one JSON value."""
        return from_json_annotation_invocation(value)


def encode_annotation_invocation(
    writer: BinaryWriter, value: AnnotationInvocation
) -> None:
    """Encode one AnnotationInvocation."""
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.source)
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.owner)
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.target)
    encode_annotation_target(writer, value.resolution)
    writer.write_unsigned(len(value.arguments))
    for item_value_arguments_0 in value.arguments:
        encode_annotation_argument(writer, item_value_arguments_0)


def decode_annotation_invocation(reader: BinaryReader) -> AnnotationInvocation:
    """Decode one AnnotationInvocation."""
    source = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    owner = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    target = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    resolution = decode_annotation_target(reader)
    arguments = [
        decode_annotation_argument(reader) for _ in range(reader.read_number())
    ]

    return AnnotationInvocation(
        source=source,
        owner=owner,
        target=target,
        resolution=resolution,
        arguments=arguments,
    )


def to_json_annotation_invocation(value: AnnotationInvocation) -> Json:
    """Return one JSON value for one AnnotationInvocation."""
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
        "resolution": to_json_annotation_target(value.resolution),
        "arguments": [
            to_json_annotation_argument(item_0) for item_0 in value.arguments
        ],
    }


def from_json_annotation_invocation(value: Json) -> AnnotationInvocation:
    """Return one AnnotationInvocation from one JSON value."""
    object_ = json_object(value)

    return AnnotationInvocation(
        source=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "source")
        ),
        owner=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "owner")
        ),
        target=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "target")
        ),
        resolution=from_json_annotation_target(json_field(object_, "resolution")),
        arguments=[
            from_json_annotation_argument(item_0)
            for item_0 in json_array(json_field(object_, "arguments"))
        ],
    )


@dataclass(frozen=True, slots=True)
class AnnotationTargetLanguageItem:
    """Compiler language item annotation."""

    language_item: destack._generated.dir.symbol.language.LanguageItem
    kind: typing.Literal["languageItem"] = "languageItem"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_annotation_target(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_annotation_target(self)


@dataclass(frozen=True, slots=True)
class AnnotationTargetSymbol:
    """User-defined annotation symbol."""

    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    kind: typing.Literal["symbol"] = "symbol"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_annotation_target(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_annotation_target(self)


@dataclass(frozen=True, slots=True)
class AnnotationTargetUnknown:
    """Unresolved or non-symbol annotation target."""

    kind: typing.Literal["unknown"] = "unknown"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_annotation_target(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_annotation_target(self)


"""Resolved annotation target."""
AnnotationTarget: typing.TypeAlias = (
    AnnotationTargetLanguageItem | AnnotationTargetSymbol | AnnotationTargetUnknown
)


def encode_annotation_target(writer: BinaryWriter, value: AnnotationTarget) -> None:
    """Encode one AnnotationTarget."""
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
    elif value.kind == "unknown":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_annotation_target(reader: BinaryReader) -> AnnotationTarget:
    """Decode one AnnotationTarget."""
    variant = reader.read_number()

    if variant == 0:
        language_item = destack._generated.dir.symbol.language.decode_language_item(
            reader
        )

        return AnnotationTargetLanguageItem(language_item=language_item)
    elif variant == 1:
        symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)

        return AnnotationTargetSymbol(symbol=symbol)
    elif variant == 2:
        return AnnotationTargetUnknown()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_annotation_target(value: AnnotationTarget) -> Json:
    """Return one JSON value for one AnnotationTarget."""
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
    elif value.kind == "unknown":
        return {
            "kind": "unknown",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_annotation_target(value: Json) -> AnnotationTarget:
    """Return one AnnotationTarget from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "languageItem":
        return AnnotationTargetLanguageItem(
            language_item=destack._generated.dir.symbol.language.from_json_language_item(
                json_field(object_, "language_item")
            )
        )
    elif kind == "symbol":
        return AnnotationTargetSymbol(
            symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
                json_field(object_, "symbol")
            )
        )
    elif kind == "unknown":
        return AnnotationTargetUnknown()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class AnnotationArgument:
    """Checked annotation argument."""

    # the argument node
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the committed static value when one exists
    value: destack._generated.dir.tree.static.GlobalStaticId | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_annotation_argument(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> AnnotationArgument:
        """Decode one AnnotationArgument."""
        return decode_annotation_argument(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_annotation_argument(self)

    @classmethod
    def from_json(cls, value: Json) -> AnnotationArgument:
        """Return one AnnotationArgument from one JSON value."""
        return from_json_annotation_argument(value)


def encode_annotation_argument(writer: BinaryWriter, value: AnnotationArgument) -> None:
    """Encode one AnnotationArgument."""
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.source)
    if value.value is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.static.encode_global_static_id(writer, value.value)


def decode_annotation_argument(reader: BinaryReader) -> AnnotationArgument:
    """Decode one AnnotationArgument."""
    source = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    value_ = reader.read_option(
        lambda: destack._generated.dir.tree.static.decode_global_static_id(reader)
    )

    return AnnotationArgument(
        source=source,
        value=value_,
    )


def to_json_annotation_argument(value: AnnotationArgument) -> Json:
    """Return one JSON value for one AnnotationArgument."""
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


def from_json_annotation_argument(value: Json) -> AnnotationArgument:
    """Return one AnnotationArgument from one JSON value."""
    object_ = json_object(value)

    return AnnotationArgument(
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


"""Unique identifier for an annotation invocation."""
LocalAnnotationId: typing.TypeAlias = int


def encode_local_annotation_id(writer: BinaryWriter, value: LocalAnnotationId) -> None:
    """Encode one LocalAnnotationId."""
    writer.write_unsigned(value)


def decode_local_annotation_id(reader: BinaryReader) -> LocalAnnotationId:
    """Decode one LocalAnnotationId."""
    return reader.read_number()


def to_json_local_annotation_id(value: LocalAnnotationId) -> Json:
    """Return one JSON value for one LocalAnnotationId."""
    return value


def from_json_local_annotation_id(value: Json) -> LocalAnnotationId:
    """Return one LocalAnnotationId from one JSON value."""
    return json_int(value)


__all__ = [
    "AnnotationSegment",
    "encode_annotation_segment",
    "decode_annotation_segment",
    "to_json_annotation_segment",
    "from_json_annotation_segment",
    "AnnotationInvocation",
    "encode_annotation_invocation",
    "decode_annotation_invocation",
    "to_json_annotation_invocation",
    "from_json_annotation_invocation",
    "AnnotationTarget",
    "encode_annotation_target",
    "decode_annotation_target",
    "to_json_annotation_target",
    "from_json_annotation_target",
    "AnnotationTargetLanguageItem",
    "AnnotationTargetSymbol",
    "AnnotationTargetUnknown",
    "AnnotationArgument",
    "encode_annotation_argument",
    "decode_annotation_argument",
    "to_json_annotation_argument",
    "from_json_annotation_argument",
    "LocalAnnotationId",
    "encode_local_annotation_id",
    "decode_local_annotation_id",
    "to_json_local_annotation_id",
    "from_json_local_annotation_id",
]
