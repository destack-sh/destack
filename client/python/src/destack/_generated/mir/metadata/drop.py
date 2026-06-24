# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping
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
    json_string,
    nested_bytes,
)

from destack._impl.mir.metadata.drop import (
    DropMetadataImpl,
)
from destack._impl.mir.metadata.drop import (
    DropGlueImpl,
)
from destack._impl.mir.metadata.drop import (
    DropHookImpl,
)

import destack._generated.mir.metadata.dispatch
import destack._generated.mir.tree.node


@dataclass(frozen=True, slots=True)
class DropMetadata(DropMetadataImpl):
    """Drop metadata for one MIR module."""

    # full drop glue keyed by type id
    glue_by_type: Mapping[destack._generated.mir.tree.node.LocalNodeId, DropGlue]
    # user-authored drop hooks keyed by type id
    hooks_by_type: Mapping[destack._generated.mir.tree.node.LocalNodeId, DropHook]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_drop_metadata(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DropMetadata:
        """Decode one DropMetadata."""
        return decode_drop_metadata(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_drop_metadata(self)

    @classmethod
    def from_json(cls, value: Json) -> DropMetadata:
        """Return one DropMetadata from one JSON value."""
        return from_json_drop_metadata(value)


def encode_drop_metadata(writer: BinaryWriter, value: DropMetadata) -> None:
    """Encode one DropMetadata."""
    entries_value_glue_by_type_0 = []
    for (
        key_value_glue_by_type_0,
        item_value_glue_by_type_0,
    ) in value.glue_by_type.items():

        def write_key_value_glue_by_type_0(writer: BinaryWriter) -> None:
            destack._generated.mir.tree.node.encode_local_node_id(
                writer, key_value_glue_by_type_0
            )

        key_bytes = nested_bytes(write_key_value_glue_by_type_0)
        entries_value_glue_by_type_0.append(
            (key_value_glue_by_type_0, item_value_glue_by_type_0, key_bytes)
        )
    entries_value_glue_by_type_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_glue_by_type_0))
    for entry_value_glue_by_type_0 in entries_value_glue_by_type_0:
        destack._generated.mir.tree.node.encode_local_node_id(
            writer, entry_value_glue_by_type_0[0]
        )
        encode_drop_glue(writer, entry_value_glue_by_type_0[1])
    entries_value_hooks_by_type_0 = []
    for (
        key_value_hooks_by_type_0,
        item_value_hooks_by_type_0,
    ) in value.hooks_by_type.items():

        def write_key_value_hooks_by_type_0(writer: BinaryWriter) -> None:
            destack._generated.mir.tree.node.encode_local_node_id(
                writer, key_value_hooks_by_type_0
            )

        key_bytes = nested_bytes(write_key_value_hooks_by_type_0)
        entries_value_hooks_by_type_0.append(
            (key_value_hooks_by_type_0, item_value_hooks_by_type_0, key_bytes)
        )
    entries_value_hooks_by_type_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_hooks_by_type_0))
    for entry_value_hooks_by_type_0 in entries_value_hooks_by_type_0:
        destack._generated.mir.tree.node.encode_local_node_id(
            writer, entry_value_hooks_by_type_0[0]
        )
        encode_drop_hook(writer, entry_value_hooks_by_type_0[1])


def decode_drop_metadata(reader: BinaryReader) -> DropMetadata:
    """Decode one DropMetadata."""
    glue_by_type = {
        destack._generated.mir.tree.node.decode_local_node_id(reader): decode_drop_glue(
            reader
        )
        for _ in range(reader.read_number())
    }
    hooks_by_type = {
        destack._generated.mir.tree.node.decode_local_node_id(reader): decode_drop_hook(
            reader
        )
        for _ in range(reader.read_number())
    }

    return DropMetadata(
        glue_by_type=glue_by_type,
        hooks_by_type=hooks_by_type,
    )


def to_json_drop_metadata(value: DropMetadata) -> Json:
    """Return one JSON value for one DropMetadata."""
    return {
        "glueByType": [
            [
                destack._generated.mir.tree.node.to_json_local_node_id(key_0),
                to_json_drop_glue(item_0),
            ]
            for key_0, item_0 in value.glue_by_type.items()
        ],
        "hooksByType": [
            [
                destack._generated.mir.tree.node.to_json_local_node_id(key_0),
                to_json_drop_hook(item_0),
            ]
            for key_0, item_0 in value.hooks_by_type.items()
        ],
    }


def from_json_drop_metadata(value: Json) -> DropMetadata:
    """Return one DropMetadata from one JSON value."""
    object_ = json_object(value)

    return DropMetadata(
        glue_by_type={
            destack._generated.mir.tree.node.from_json_local_node_id(
                key_0
            ): from_json_drop_glue(item_0)
            for key_0, item_0 in json_array(json_field(object_, "glueByType"))
        },
        hooks_by_type={
            destack._generated.mir.tree.node.from_json_local_node_id(
                key_0
            ): from_json_drop_hook(item_0)
            for key_0, item_0 in json_array(json_field(object_, "hooksByType"))
        },
    )


@dataclass(frozen=True, slots=True)
class DropGlueNone(DropGlueImpl):
    """No drop glue is required."""

    kind: typing.Literal["none"] = "none"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_drop_glue(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_drop_glue(self)


@dataclass(frozen=True, slots=True)
class DropGlueGenerated(DropGlueImpl):
    """Call compiler-generated drop glue."""

    # the drop function
    function: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["generated"] = "generated"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_drop_glue(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_drop_glue(self)


@dataclass(frozen=True, slots=True)
class DropGlueDynamic(DropGlueImpl):
    """Dispatch through a runtime drop slot."""

    # the drop dispatch slot
    slot: destack._generated.mir.metadata.dispatch.DispatchSlot
    kind: typing.Literal["dynamic"] = "dynamic"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_drop_glue(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_drop_glue(self)


"""Full drop glue selected for one MIR type."""
DropGlue: typing.TypeAlias = DropGlueNone | DropGlueGenerated | DropGlueDynamic


def encode_drop_glue(writer: BinaryWriter, value: DropGlue) -> None:
    """Encode one DropGlue."""
    if value.kind == "none":
        writer.write_unsigned(0)
    elif value.kind == "generated":
        writer.write_unsigned(1)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.function)
    elif value.kind == "dynamic":
        writer.write_unsigned(2)
        destack._generated.mir.metadata.dispatch.encode_dispatch_slot(
            writer, value.slot
        )
    else:
        raise SerdeError("unknown enum variant")


def decode_drop_glue(reader: BinaryReader) -> DropGlue:
    """Decode one DropGlue."""
    variant = reader.read_number()

    if variant == 0:
        return DropGlueNone()
    elif variant == 1:
        function = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return DropGlueGenerated(
            function=function,
        )
    elif variant == 2:
        slot = destack._generated.mir.metadata.dispatch.decode_dispatch_slot(reader)

        return DropGlueDynamic(
            slot=slot,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_drop_glue(value: DropGlue) -> Json:
    """Return one JSON value for one DropGlue."""
    if value.kind == "none":
        return {
            "kind": "none",
        }
    elif value.kind == "generated":
        return {
            "kind": "generated",
            "function": destack._generated.mir.tree.node.to_json_local_node_id(
                value.function
            ),
        }
    elif value.kind == "dynamic":
        return {
            "kind": "dynamic",
            "slot": destack._generated.mir.metadata.dispatch.to_json_dispatch_slot(
                value.slot
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_drop_glue(value: Json) -> DropGlue:
    """Return one DropGlue from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "none":
        return DropGlueNone()
    elif kind == "generated":
        return DropGlueGenerated(
            function=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "function")
            ),
        )
    elif kind == "dynamic":
        return DropGlueDynamic(
            slot=destack._generated.mir.metadata.dispatch.from_json_dispatch_slot(
                json_field(object_, "slot")
            ),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class DropHook(DropHookImpl):
    """User-authored drop hook for one MIR type."""

    # the hook function
    function: destack._generated.mir.tree.node.LocalNodeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_drop_hook(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DropHook:
        """Decode one DropHook."""
        return decode_drop_hook(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_drop_hook(self)

    @classmethod
    def from_json(cls, value: Json) -> DropHook:
        """Return one DropHook from one JSON value."""
        return from_json_drop_hook(value)


def encode_drop_hook(writer: BinaryWriter, value: DropHook) -> None:
    """Encode one DropHook."""
    destack._generated.mir.tree.node.encode_local_node_id(writer, value.function)


def decode_drop_hook(reader: BinaryReader) -> DropHook:
    """Decode one DropHook."""
    function = destack._generated.mir.tree.node.decode_local_node_id(reader)

    return DropHook(
        function=function,
    )


def to_json_drop_hook(value: DropHook) -> Json:
    """Return one JSON value for one DropHook."""
    return {
        "function": destack._generated.mir.tree.node.to_json_local_node_id(
            value.function
        ),
    }


def from_json_drop_hook(value: Json) -> DropHook:
    """Return one DropHook from one JSON value."""
    object_ = json_object(value)

    return DropHook(
        function=destack._generated.mir.tree.node.from_json_local_node_id(
            json_field(object_, "function")
        ),
    )


__all__ = [
    "DropMetadata",
    "encode_drop_metadata",
    "decode_drop_metadata",
    "to_json_drop_metadata",
    "from_json_drop_metadata",
    "DropGlue",
    "encode_drop_glue",
    "decode_drop_glue",
    "to_json_drop_glue",
    "from_json_drop_glue",
    "DropGlueNone",
    "DropGlueGenerated",
    "DropGlueDynamic",
    "DropHook",
    "encode_drop_hook",
    "decode_drop_hook",
    "to_json_drop_hook",
    "from_json_drop_hook",
]
