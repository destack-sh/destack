# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_array,
    json_field,
    json_object,
    nested_bytes,
)

import destack._generated.dir.tree.node
import destack._generated.dir.type.resolution
import destack._generated.source.file.model.module


@dataclass(frozen=True, slots=True)
class ResolutionSegment:
    """Resolutions added by one DIR phase."""

    # the module id of the resolution segment
    module_id: destack._generated.source.file.model.module.ModuleId
    # checked lexical or path resolutions keyed by DIR node
    names: Mapping[
        destack._generated.dir.tree.node.GlobalNodeIdAny,
        destack._generated.dir.type.resolution.NameResolution,
    ]
    # checked generic instantiations keyed by DIR node
    instantiations: Mapping[
        destack._generated.dir.tree.node.GlobalNodeIdAny,
        destack._generated.dir.type.resolution.InstantiationResolution,
    ]
    # checked label resolutions keyed by DIR node
    labels: Mapping[
        destack._generated.dir.tree.node.GlobalNodeIdAny,
        destack._generated.dir.type.resolution.LabelResolution,
    ]
    # checked receiver resolutions keyed by DIR node
    receivers: Mapping[
        destack._generated.dir.tree.node.GlobalNodeIdAny,
        destack._generated.dir.type.resolution.ReceiverResolution,
    ]
    # checked member resolutions keyed by DIR node
    members: Mapping[
        destack._generated.dir.tree.node.GlobalNodeIdAny,
        destack._generated.dir.type.resolution.MemberResolution,
    ]
    # checked call resolutions keyed by DIR node
    calls: Mapping[
        destack._generated.dir.tree.node.GlobalNodeIdAny,
        destack._generated.dir.type.resolution.CallResolution,
    ]
    # checked place resolutions keyed by DIR node
    places: Mapping[
        destack._generated.dir.tree.node.GlobalNodeIdAny,
        destack._generated.dir.type.resolution.PlaceResolution,
    ]
    # checked guard resolutions keyed by DIR node
    guards: Mapping[
        destack._generated.dir.tree.node.GlobalNodeIdAny,
        destack._generated.dir.type.resolution.GuardResolution,
    ]
    # checked construct resolutions keyed by DIR node
    constructs: Mapping[
        destack._generated.dir.tree.node.GlobalNodeIdAny,
        destack._generated.dir.type.resolution.ConstructResolution,
    ]
    # checked pattern resolutions keyed by DIR node
    patterns: Mapping[
        destack._generated.dir.tree.node.GlobalNodeIdAny,
        destack._generated.dir.type.resolution.PatternResolution,
    ]
    # checked assignment pattern resolutions keyed by DIR node
    assign_patterns: Mapping[
        destack._generated.dir.tree.node.GlobalNodeIdAny,
        destack._generated.dir.type.resolution.AssignPatternResolution,
    ]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_resolution_segment(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ResolutionSegment:
        """Decode one ResolutionSegment."""
        return decode_resolution_segment(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_resolution_segment(self)

    @classmethod
    def from_json(cls, value: Json) -> ResolutionSegment:
        """Return one ResolutionSegment from one JSON value."""
        return from_json_resolution_segment(value)


def encode_resolution_segment(writer: BinaryWriter, value: ResolutionSegment) -> None:
    """Encode one ResolutionSegment."""
    destack._generated.source.file.model.module.encode_module_id(
        writer, value.module_id
    )
    entries_value_names_0 = []
    for key_value_names_0, item_value_names_0 in value.names.items():

        def write_key_value_names_0(writer: BinaryWriter) -> None:
            destack._generated.dir.tree.node.encode_global_node_id_any(
                writer, key_value_names_0
            )

        key_bytes = nested_bytes(write_key_value_names_0)
        entries_value_names_0.append((key_value_names_0, item_value_names_0, key_bytes))
    entries_value_names_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_names_0))
    for entry_value_names_0 in entries_value_names_0:
        destack._generated.dir.tree.node.encode_global_node_id_any(
            writer, entry_value_names_0[0]
        )
        destack._generated.dir.type.resolution.encode_name_resolution(
            writer, entry_value_names_0[1]
        )
    entries_value_instantiations_0 = []
    for (
        key_value_instantiations_0,
        item_value_instantiations_0,
    ) in value.instantiations.items():

        def write_key_value_instantiations_0(writer: BinaryWriter) -> None:
            destack._generated.dir.tree.node.encode_global_node_id_any(
                writer, key_value_instantiations_0
            )

        key_bytes = nested_bytes(write_key_value_instantiations_0)
        entries_value_instantiations_0.append(
            (key_value_instantiations_0, item_value_instantiations_0, key_bytes)
        )
    entries_value_instantiations_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_instantiations_0))
    for entry_value_instantiations_0 in entries_value_instantiations_0:
        destack._generated.dir.tree.node.encode_global_node_id_any(
            writer, entry_value_instantiations_0[0]
        )
        destack._generated.dir.type.resolution.encode_instantiation_resolution(
            writer, entry_value_instantiations_0[1]
        )
    entries_value_labels_0 = []
    for key_value_labels_0, item_value_labels_0 in value.labels.items():

        def write_key_value_labels_0(writer: BinaryWriter) -> None:
            destack._generated.dir.tree.node.encode_global_node_id_any(
                writer, key_value_labels_0
            )

        key_bytes = nested_bytes(write_key_value_labels_0)
        entries_value_labels_0.append(
            (key_value_labels_0, item_value_labels_0, key_bytes)
        )
    entries_value_labels_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_labels_0))
    for entry_value_labels_0 in entries_value_labels_0:
        destack._generated.dir.tree.node.encode_global_node_id_any(
            writer, entry_value_labels_0[0]
        )
        destack._generated.dir.type.resolution.encode_label_resolution(
            writer, entry_value_labels_0[1]
        )
    entries_value_receivers_0 = []
    for key_value_receivers_0, item_value_receivers_0 in value.receivers.items():

        def write_key_value_receivers_0(writer: BinaryWriter) -> None:
            destack._generated.dir.tree.node.encode_global_node_id_any(
                writer, key_value_receivers_0
            )

        key_bytes = nested_bytes(write_key_value_receivers_0)
        entries_value_receivers_0.append(
            (key_value_receivers_0, item_value_receivers_0, key_bytes)
        )
    entries_value_receivers_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_receivers_0))
    for entry_value_receivers_0 in entries_value_receivers_0:
        destack._generated.dir.tree.node.encode_global_node_id_any(
            writer, entry_value_receivers_0[0]
        )
        destack._generated.dir.type.resolution.encode_receiver_resolution(
            writer, entry_value_receivers_0[1]
        )
    entries_value_members_0 = []
    for key_value_members_0, item_value_members_0 in value.members.items():

        def write_key_value_members_0(writer: BinaryWriter) -> None:
            destack._generated.dir.tree.node.encode_global_node_id_any(
                writer, key_value_members_0
            )

        key_bytes = nested_bytes(write_key_value_members_0)
        entries_value_members_0.append(
            (key_value_members_0, item_value_members_0, key_bytes)
        )
    entries_value_members_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_members_0))
    for entry_value_members_0 in entries_value_members_0:
        destack._generated.dir.tree.node.encode_global_node_id_any(
            writer, entry_value_members_0[0]
        )
        destack._generated.dir.type.resolution.encode_member_resolution(
            writer, entry_value_members_0[1]
        )
    entries_value_calls_0 = []
    for key_value_calls_0, item_value_calls_0 in value.calls.items():

        def write_key_value_calls_0(writer: BinaryWriter) -> None:
            destack._generated.dir.tree.node.encode_global_node_id_any(
                writer, key_value_calls_0
            )

        key_bytes = nested_bytes(write_key_value_calls_0)
        entries_value_calls_0.append((key_value_calls_0, item_value_calls_0, key_bytes))
    entries_value_calls_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_calls_0))
    for entry_value_calls_0 in entries_value_calls_0:
        destack._generated.dir.tree.node.encode_global_node_id_any(
            writer, entry_value_calls_0[0]
        )
        destack._generated.dir.type.resolution.encode_call_resolution(
            writer, entry_value_calls_0[1]
        )
    entries_value_places_0 = []
    for key_value_places_0, item_value_places_0 in value.places.items():

        def write_key_value_places_0(writer: BinaryWriter) -> None:
            destack._generated.dir.tree.node.encode_global_node_id_any(
                writer, key_value_places_0
            )

        key_bytes = nested_bytes(write_key_value_places_0)
        entries_value_places_0.append(
            (key_value_places_0, item_value_places_0, key_bytes)
        )
    entries_value_places_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_places_0))
    for entry_value_places_0 in entries_value_places_0:
        destack._generated.dir.tree.node.encode_global_node_id_any(
            writer, entry_value_places_0[0]
        )
        destack._generated.dir.type.resolution.encode_place_resolution(
            writer, entry_value_places_0[1]
        )
    entries_value_guards_0 = []
    for key_value_guards_0, item_value_guards_0 in value.guards.items():

        def write_key_value_guards_0(writer: BinaryWriter) -> None:
            destack._generated.dir.tree.node.encode_global_node_id_any(
                writer, key_value_guards_0
            )

        key_bytes = nested_bytes(write_key_value_guards_0)
        entries_value_guards_0.append(
            (key_value_guards_0, item_value_guards_0, key_bytes)
        )
    entries_value_guards_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_guards_0))
    for entry_value_guards_0 in entries_value_guards_0:
        destack._generated.dir.tree.node.encode_global_node_id_any(
            writer, entry_value_guards_0[0]
        )
        destack._generated.dir.type.resolution.encode_guard_resolution(
            writer, entry_value_guards_0[1]
        )
    entries_value_constructs_0 = []
    for key_value_constructs_0, item_value_constructs_0 in value.constructs.items():

        def write_key_value_constructs_0(writer: BinaryWriter) -> None:
            destack._generated.dir.tree.node.encode_global_node_id_any(
                writer, key_value_constructs_0
            )

        key_bytes = nested_bytes(write_key_value_constructs_0)
        entries_value_constructs_0.append(
            (key_value_constructs_0, item_value_constructs_0, key_bytes)
        )
    entries_value_constructs_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_constructs_0))
    for entry_value_constructs_0 in entries_value_constructs_0:
        destack._generated.dir.tree.node.encode_global_node_id_any(
            writer, entry_value_constructs_0[0]
        )
        destack._generated.dir.type.resolution.encode_construct_resolution(
            writer, entry_value_constructs_0[1]
        )
    entries_value_patterns_0 = []
    for key_value_patterns_0, item_value_patterns_0 in value.patterns.items():

        def write_key_value_patterns_0(writer: BinaryWriter) -> None:
            destack._generated.dir.tree.node.encode_global_node_id_any(
                writer, key_value_patterns_0
            )

        key_bytes = nested_bytes(write_key_value_patterns_0)
        entries_value_patterns_0.append(
            (key_value_patterns_0, item_value_patterns_0, key_bytes)
        )
    entries_value_patterns_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_patterns_0))
    for entry_value_patterns_0 in entries_value_patterns_0:
        destack._generated.dir.tree.node.encode_global_node_id_any(
            writer, entry_value_patterns_0[0]
        )
        destack._generated.dir.type.resolution.encode_pattern_resolution(
            writer, entry_value_patterns_0[1]
        )
    entries_value_assign_patterns_0 = []
    for (
        key_value_assign_patterns_0,
        item_value_assign_patterns_0,
    ) in value.assign_patterns.items():

        def write_key_value_assign_patterns_0(writer: BinaryWriter) -> None:
            destack._generated.dir.tree.node.encode_global_node_id_any(
                writer, key_value_assign_patterns_0
            )

        key_bytes = nested_bytes(write_key_value_assign_patterns_0)
        entries_value_assign_patterns_0.append(
            (key_value_assign_patterns_0, item_value_assign_patterns_0, key_bytes)
        )
    entries_value_assign_patterns_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_assign_patterns_0))
    for entry_value_assign_patterns_0 in entries_value_assign_patterns_0:
        destack._generated.dir.tree.node.encode_global_node_id_any(
            writer, entry_value_assign_patterns_0[0]
        )
        destack._generated.dir.type.resolution.encode_assign_pattern_resolution(
            writer, entry_value_assign_patterns_0[1]
        )


def decode_resolution_segment(reader: BinaryReader) -> ResolutionSegment:
    """Decode one ResolutionSegment."""
    module_id = destack._generated.source.file.model.module.decode_module_id(reader)
    names = {
        destack._generated.dir.tree.node.decode_global_node_id_any(
            reader
        ): destack._generated.dir.type.resolution.decode_name_resolution(reader)
        for _ in range(reader.read_number())
    }
    instantiations = {
        destack._generated.dir.tree.node.decode_global_node_id_any(
            reader
        ): destack._generated.dir.type.resolution.decode_instantiation_resolution(
            reader
        )
        for _ in range(reader.read_number())
    }
    labels = {
        destack._generated.dir.tree.node.decode_global_node_id_any(
            reader
        ): destack._generated.dir.type.resolution.decode_label_resolution(reader)
        for _ in range(reader.read_number())
    }
    receivers = {
        destack._generated.dir.tree.node.decode_global_node_id_any(
            reader
        ): destack._generated.dir.type.resolution.decode_receiver_resolution(reader)
        for _ in range(reader.read_number())
    }
    members = {
        destack._generated.dir.tree.node.decode_global_node_id_any(
            reader
        ): destack._generated.dir.type.resolution.decode_member_resolution(reader)
        for _ in range(reader.read_number())
    }
    calls = {
        destack._generated.dir.tree.node.decode_global_node_id_any(
            reader
        ): destack._generated.dir.type.resolution.decode_call_resolution(reader)
        for _ in range(reader.read_number())
    }
    places = {
        destack._generated.dir.tree.node.decode_global_node_id_any(
            reader
        ): destack._generated.dir.type.resolution.decode_place_resolution(reader)
        for _ in range(reader.read_number())
    }
    guards = {
        destack._generated.dir.tree.node.decode_global_node_id_any(
            reader
        ): destack._generated.dir.type.resolution.decode_guard_resolution(reader)
        for _ in range(reader.read_number())
    }
    constructs = {
        destack._generated.dir.tree.node.decode_global_node_id_any(
            reader
        ): destack._generated.dir.type.resolution.decode_construct_resolution(reader)
        for _ in range(reader.read_number())
    }
    patterns = {
        destack._generated.dir.tree.node.decode_global_node_id_any(
            reader
        ): destack._generated.dir.type.resolution.decode_pattern_resolution(reader)
        for _ in range(reader.read_number())
    }
    assign_patterns = {
        destack._generated.dir.tree.node.decode_global_node_id_any(
            reader
        ): destack._generated.dir.type.resolution.decode_assign_pattern_resolution(
            reader
        )
        for _ in range(reader.read_number())
    }

    return ResolutionSegment(
        module_id=module_id,
        names=names,
        instantiations=instantiations,
        labels=labels,
        receivers=receivers,
        members=members,
        calls=calls,
        places=places,
        guards=guards,
        constructs=constructs,
        patterns=patterns,
        assign_patterns=assign_patterns,
    )


def to_json_resolution_segment(value: ResolutionSegment) -> Json:
    """Return one JSON value for one ResolutionSegment."""
    return {
        "moduleId": destack._generated.source.file.model.module.to_json_module_id(
            value.module_id
        ),
        "names": [
            [
                destack._generated.dir.tree.node.to_json_global_node_id_any(key_0),
                destack._generated.dir.type.resolution.to_json_name_resolution(item_0),
            ]
            for key_0, item_0 in value.names.items()
        ],
        "instantiations": [
            [
                destack._generated.dir.tree.node.to_json_global_node_id_any(key_0),
                destack._generated.dir.type.resolution.to_json_instantiation_resolution(
                    item_0
                ),
            ]
            for key_0, item_0 in value.instantiations.items()
        ],
        "labels": [
            [
                destack._generated.dir.tree.node.to_json_global_node_id_any(key_0),
                destack._generated.dir.type.resolution.to_json_label_resolution(item_0),
            ]
            for key_0, item_0 in value.labels.items()
        ],
        "receivers": [
            [
                destack._generated.dir.tree.node.to_json_global_node_id_any(key_0),
                destack._generated.dir.type.resolution.to_json_receiver_resolution(
                    item_0
                ),
            ]
            for key_0, item_0 in value.receivers.items()
        ],
        "members": [
            [
                destack._generated.dir.tree.node.to_json_global_node_id_any(key_0),
                destack._generated.dir.type.resolution.to_json_member_resolution(
                    item_0
                ),
            ]
            for key_0, item_0 in value.members.items()
        ],
        "calls": [
            [
                destack._generated.dir.tree.node.to_json_global_node_id_any(key_0),
                destack._generated.dir.type.resolution.to_json_call_resolution(item_0),
            ]
            for key_0, item_0 in value.calls.items()
        ],
        "places": [
            [
                destack._generated.dir.tree.node.to_json_global_node_id_any(key_0),
                destack._generated.dir.type.resolution.to_json_place_resolution(item_0),
            ]
            for key_0, item_0 in value.places.items()
        ],
        "guards": [
            [
                destack._generated.dir.tree.node.to_json_global_node_id_any(key_0),
                destack._generated.dir.type.resolution.to_json_guard_resolution(item_0),
            ]
            for key_0, item_0 in value.guards.items()
        ],
        "constructs": [
            [
                destack._generated.dir.tree.node.to_json_global_node_id_any(key_0),
                destack._generated.dir.type.resolution.to_json_construct_resolution(
                    item_0
                ),
            ]
            for key_0, item_0 in value.constructs.items()
        ],
        "patterns": [
            [
                destack._generated.dir.tree.node.to_json_global_node_id_any(key_0),
                destack._generated.dir.type.resolution.to_json_pattern_resolution(
                    item_0
                ),
            ]
            for key_0, item_0 in value.patterns.items()
        ],
        "assignPatterns": [
            [
                destack._generated.dir.tree.node.to_json_global_node_id_any(key_0),
                destack._generated.dir.type.resolution.to_json_assign_pattern_resolution(
                    item_0
                ),
            ]
            for key_0, item_0 in value.assign_patterns.items()
        ],
    }


def from_json_resolution_segment(value: Json) -> ResolutionSegment:
    """Return one ResolutionSegment from one JSON value."""
    object_ = json_object(value)

    return ResolutionSegment(
        module_id=destack._generated.source.file.model.module.from_json_module_id(
            json_field(object_, "moduleId")
        ),
        names={
            destack._generated.dir.tree.node.from_json_global_node_id_any(
                key_0
            ): destack._generated.dir.type.resolution.from_json_name_resolution(item_0)
            for key_0, item_0 in json_array(json_field(object_, "names"))
        },
        instantiations={
            destack._generated.dir.tree.node.from_json_global_node_id_any(
                key_0
            ): destack._generated.dir.type.resolution.from_json_instantiation_resolution(
                item_0
            )
            for key_0, item_0 in json_array(json_field(object_, "instantiations"))
        },
        labels={
            destack._generated.dir.tree.node.from_json_global_node_id_any(
                key_0
            ): destack._generated.dir.type.resolution.from_json_label_resolution(item_0)
            for key_0, item_0 in json_array(json_field(object_, "labels"))
        },
        receivers={
            destack._generated.dir.tree.node.from_json_global_node_id_any(
                key_0
            ): destack._generated.dir.type.resolution.from_json_receiver_resolution(
                item_0
            )
            for key_0, item_0 in json_array(json_field(object_, "receivers"))
        },
        members={
            destack._generated.dir.tree.node.from_json_global_node_id_any(
                key_0
            ): destack._generated.dir.type.resolution.from_json_member_resolution(
                item_0
            )
            for key_0, item_0 in json_array(json_field(object_, "members"))
        },
        calls={
            destack._generated.dir.tree.node.from_json_global_node_id_any(
                key_0
            ): destack._generated.dir.type.resolution.from_json_call_resolution(item_0)
            for key_0, item_0 in json_array(json_field(object_, "calls"))
        },
        places={
            destack._generated.dir.tree.node.from_json_global_node_id_any(
                key_0
            ): destack._generated.dir.type.resolution.from_json_place_resolution(item_0)
            for key_0, item_0 in json_array(json_field(object_, "places"))
        },
        guards={
            destack._generated.dir.tree.node.from_json_global_node_id_any(
                key_0
            ): destack._generated.dir.type.resolution.from_json_guard_resolution(item_0)
            for key_0, item_0 in json_array(json_field(object_, "guards"))
        },
        constructs={
            destack._generated.dir.tree.node.from_json_global_node_id_any(
                key_0
            ): destack._generated.dir.type.resolution.from_json_construct_resolution(
                item_0
            )
            for key_0, item_0 in json_array(json_field(object_, "constructs"))
        },
        patterns={
            destack._generated.dir.tree.node.from_json_global_node_id_any(
                key_0
            ): destack._generated.dir.type.resolution.from_json_pattern_resolution(
                item_0
            )
            for key_0, item_0 in json_array(json_field(object_, "patterns"))
        },
        assign_patterns={
            destack._generated.dir.tree.node.from_json_global_node_id_any(
                key_0
            ): destack._generated.dir.type.resolution.from_json_assign_pattern_resolution(
                item_0
            )
            for key_0, item_0 in json_array(json_field(object_, "assignPatterns"))
        },
    )


__all__ = [
    "ResolutionSegment",
    "encode_resolution_segment",
    "decode_resolution_segment",
    "to_json_resolution_segment",
    "from_json_resolution_segment",
]
