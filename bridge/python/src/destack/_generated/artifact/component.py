# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
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

import destack._generated.source.file.model.component
import destack._generated.source.file.model.module
import destack._generated.source.file.model.profile


@dataclass(frozen=True, slots=True)
class ComponentGraph:
    """Strongly connected component partition of one profile's module graph."""

    # the profile this partition belongs to
    profile: destack._generated.source.file.model.profile.ProfileId
    # imported modules per module, deduplicated in import order
    imports: Mapping[
        destack._generated.source.file.model.module.ModuleId,
        Sequence[destack._generated.source.file.model.module.ModuleId],
    ]
    # owning component per module
    component_of: Mapping[
        destack._generated.source.file.model.module.ModuleId,
        destack._generated.source.file.model.component.ComponentId,
    ]
    # member modules per component, sorted, with the entry first
    members: Mapping[
        destack._generated.source.file.model.component.ComponentId,
        Sequence[destack._generated.source.file.model.module.ModuleId],
    ]
    # external components each component depends on
    dependencies: Mapping[
        destack._generated.source.file.model.component.ComponentId,
        Sequence[destack._generated.source.file.model.component.ComponentId],
    ]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_component_graph(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ComponentGraph:
        """Decode one ComponentGraph."""
        return decode_component_graph(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_component_graph(self)

    @classmethod
    def from_json(cls, value: Json) -> ComponentGraph:
        """Return one ComponentGraph from one JSON value."""
        return from_json_component_graph(value)


def encode_component_graph(writer: BinaryWriter, value: ComponentGraph) -> None:
    """Encode one ComponentGraph."""
    destack._generated.source.file.model.profile.encode_profile_id(
        writer, value.profile
    )
    entries_value_imports_0 = []
    for key_value_imports_0, item_value_imports_0 in value.imports.items():

        def write_key_value_imports_0(writer: BinaryWriter) -> None:
            destack._generated.source.file.model.module.encode_module_id(
                writer, key_value_imports_0
            )

        key_bytes = nested_bytes(write_key_value_imports_0)
        entries_value_imports_0.append(
            (key_value_imports_0, item_value_imports_0, key_bytes)
        )
    entries_value_imports_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_imports_0))
    for entry_value_imports_0 in entries_value_imports_0:
        destack._generated.source.file.model.module.encode_module_id(
            writer, entry_value_imports_0[0]
        )
        writer.write_unsigned(len(entry_value_imports_0[1]))
        for item_entry_value_imports_0_1_1 in entry_value_imports_0[1]:
            destack._generated.source.file.model.module.encode_module_id(
                writer, item_entry_value_imports_0_1_1
            )
    entries_value_component_of_0 = []
    for (
        key_value_component_of_0,
        item_value_component_of_0,
    ) in value.component_of.items():

        def write_key_value_component_of_0(writer: BinaryWriter) -> None:
            destack._generated.source.file.model.module.encode_module_id(
                writer, key_value_component_of_0
            )

        key_bytes = nested_bytes(write_key_value_component_of_0)
        entries_value_component_of_0.append(
            (key_value_component_of_0, item_value_component_of_0, key_bytes)
        )
    entries_value_component_of_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_component_of_0))
    for entry_value_component_of_0 in entries_value_component_of_0:
        destack._generated.source.file.model.module.encode_module_id(
            writer, entry_value_component_of_0[0]
        )
        destack._generated.source.file.model.component.encode_component_id(
            writer, entry_value_component_of_0[1]
        )
    entries_value_members_0 = []
    for key_value_members_0, item_value_members_0 in value.members.items():

        def write_key_value_members_0(writer: BinaryWriter) -> None:
            destack._generated.source.file.model.component.encode_component_id(
                writer, key_value_members_0
            )

        key_bytes = nested_bytes(write_key_value_members_0)
        entries_value_members_0.append(
            (key_value_members_0, item_value_members_0, key_bytes)
        )
    entries_value_members_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_members_0))
    for entry_value_members_0 in entries_value_members_0:
        destack._generated.source.file.model.component.encode_component_id(
            writer, entry_value_members_0[0]
        )
        writer.write_unsigned(len(entry_value_members_0[1]))
        for item_entry_value_members_0_1_1 in entry_value_members_0[1]:
            destack._generated.source.file.model.module.encode_module_id(
                writer, item_entry_value_members_0_1_1
            )
    entries_value_dependencies_0 = []
    for (
        key_value_dependencies_0,
        item_value_dependencies_0,
    ) in value.dependencies.items():

        def write_key_value_dependencies_0(writer: BinaryWriter) -> None:
            destack._generated.source.file.model.component.encode_component_id(
                writer, key_value_dependencies_0
            )

        key_bytes = nested_bytes(write_key_value_dependencies_0)
        entries_value_dependencies_0.append(
            (key_value_dependencies_0, item_value_dependencies_0, key_bytes)
        )
    entries_value_dependencies_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_dependencies_0))
    for entry_value_dependencies_0 in entries_value_dependencies_0:
        destack._generated.source.file.model.component.encode_component_id(
            writer, entry_value_dependencies_0[0]
        )
        writer.write_unsigned(len(entry_value_dependencies_0[1]))
        for item_entry_value_dependencies_0_1_1 in entry_value_dependencies_0[1]:
            destack._generated.source.file.model.component.encode_component_id(
                writer, item_entry_value_dependencies_0_1_1
            )


def decode_component_graph(reader: BinaryReader) -> ComponentGraph:
    """Decode one ComponentGraph."""
    profile = destack._generated.source.file.model.profile.decode_profile_id(reader)
    imports = {
        destack._generated.source.file.model.module.decode_module_id(reader): [
            destack._generated.source.file.model.module.decode_module_id(reader)
            for _ in range(reader.read_number())
        ]
        for _ in range(reader.read_number())
    }
    component_of = {
        destack._generated.source.file.model.module.decode_module_id(
            reader
        ): destack._generated.source.file.model.component.decode_component_id(reader)
        for _ in range(reader.read_number())
    }
    members = {
        destack._generated.source.file.model.component.decode_component_id(reader): [
            destack._generated.source.file.model.module.decode_module_id(reader)
            for _ in range(reader.read_number())
        ]
        for _ in range(reader.read_number())
    }
    dependencies = {
        destack._generated.source.file.model.component.decode_component_id(reader): [
            destack._generated.source.file.model.component.decode_component_id(reader)
            for _ in range(reader.read_number())
        ]
        for _ in range(reader.read_number())
    }

    return ComponentGraph(
        profile=profile,
        imports=imports,
        component_of=component_of,
        members=members,
        dependencies=dependencies,
    )


def to_json_component_graph(value: ComponentGraph) -> Json:
    """Return one JSON value for one ComponentGraph."""
    return {
        "profile": destack._generated.source.file.model.profile.to_json_profile_id(
            value.profile
        ),
        "imports": [
            [
                destack._generated.source.file.model.module.to_json_module_id(key_0),
                [
                    destack._generated.source.file.model.module.to_json_module_id(
                        item_1
                    )
                    for item_1 in item_0
                ],
            ]
            for key_0, item_0 in value.imports.items()
        ],
        "componentOf": [
            [
                destack._generated.source.file.model.module.to_json_module_id(key_0),
                destack._generated.source.file.model.component.to_json_component_id(
                    item_0
                ),
            ]
            for key_0, item_0 in value.component_of.items()
        ],
        "members": [
            [
                destack._generated.source.file.model.component.to_json_component_id(
                    key_0
                ),
                [
                    destack._generated.source.file.model.module.to_json_module_id(
                        item_1
                    )
                    for item_1 in item_0
                ],
            ]
            for key_0, item_0 in value.members.items()
        ],
        "dependencies": [
            [
                destack._generated.source.file.model.component.to_json_component_id(
                    key_0
                ),
                [
                    destack._generated.source.file.model.component.to_json_component_id(
                        item_1
                    )
                    for item_1 in item_0
                ],
            ]
            for key_0, item_0 in value.dependencies.items()
        ],
    }


def from_json_component_graph(value: Json) -> ComponentGraph:
    """Return one ComponentGraph from one JSON value."""
    object_ = json_object(value)

    return ComponentGraph(
        profile=destack._generated.source.file.model.profile.from_json_profile_id(
            json_field(object_, "profile")
        ),
        imports={
            destack._generated.source.file.model.module.from_json_module_id(key_0): [
                destack._generated.source.file.model.module.from_json_module_id(item_1)
                for item_1 in json_array(item_0)
            ]
            for key_0, item_0 in json_array(json_field(object_, "imports"))
        },
        component_of={
            destack._generated.source.file.model.module.from_json_module_id(
                key_0
            ): destack._generated.source.file.model.component.from_json_component_id(
                item_0
            )
            for key_0, item_0 in json_array(json_field(object_, "componentOf"))
        },
        members={
            destack._generated.source.file.model.component.from_json_component_id(
                key_0
            ): [
                destack._generated.source.file.model.module.from_json_module_id(item_1)
                for item_1 in json_array(item_0)
            ]
            for key_0, item_0 in json_array(json_field(object_, "members"))
        },
        dependencies={
            destack._generated.source.file.model.component.from_json_component_id(
                key_0
            ): [
                destack._generated.source.file.model.component.from_json_component_id(
                    item_1
                )
                for item_1 in json_array(item_0)
            ]
            for key_0, item_0 in json_array(json_field(object_, "dependencies"))
        },
    )


__all__ = [
    "ComponentGraph",
    "encode_component_graph",
    "decode_component_graph",
    "to_json_component_graph",
    "from_json_component_graph",
]
