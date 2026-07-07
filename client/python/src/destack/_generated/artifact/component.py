# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_array,
    json_field,
    json_int,
    json_object,
)

import destack._generated.source.file.model.component
import destack._generated.source.file.model.module
import destack._generated.source.file.model.profile


@dataclass(frozen=True, slots=True)
class ModuleGraph:
    """Dense module dependency graph for one profile."""

    # the profile this graph belongs to
    profile: destack._generated.source.file.model.profile.ProfileId
    # modules sorted by stable id
    modules: Sequence[destack._generated.source.file.model.module.ModuleId]
    # per-module edge targets as dense module indexes
    edges: Sequence[Sequence[int]]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_module_graph(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ModuleGraph:
        """Decode one ModuleGraph."""
        return decode_module_graph(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_module_graph(self)

    @classmethod
    def from_json(cls, value: Json) -> ModuleGraph:
        """Return one ModuleGraph from one JSON value."""
        return from_json_module_graph(value)


def encode_module_graph(writer: BinaryWriter, value: ModuleGraph) -> None:
    """Encode one ModuleGraph."""
    destack._generated.source.file.model.profile.encode_profile_id(
        writer, value.profile
    )
    writer.write_unsigned(len(value.modules))
    for item_value_modules_0 in value.modules:
        destack._generated.source.file.model.module.encode_module_id(
            writer, item_value_modules_0
        )
    writer.write_unsigned(len(value.edges))
    for item_value_edges_0 in value.edges:
        writer.write_unsigned(len(item_value_edges_0))
        for item_item_value_edges_0_1 in item_value_edges_0:
            writer.write_unsigned(item_item_value_edges_0_1)


def decode_module_graph(reader: BinaryReader) -> ModuleGraph:
    """Decode one ModuleGraph."""
    profile = destack._generated.source.file.model.profile.decode_profile_id(reader)
    modules = [
        destack._generated.source.file.model.module.decode_module_id(reader)
        for _ in range(reader.read_number())
    ]
    edges = [
        [reader.read_number() for _ in range(reader.read_number())]
        for _ in range(reader.read_number())
    ]

    return ModuleGraph(
        profile=profile,
        modules=modules,
        edges=edges,
    )


def to_json_module_graph(value: ModuleGraph) -> Json:
    """Return one JSON value for one ModuleGraph."""
    return {
        "profile": destack._generated.source.file.model.profile.to_json_profile_id(
            value.profile
        ),
        "modules": [
            destack._generated.source.file.model.module.to_json_module_id(item_0)
            for item_0 in value.modules
        ],
        "edges": [[item_1 for item_1 in item_0] for item_0 in value.edges],
    }


def from_json_module_graph(value: Json) -> ModuleGraph:
    """Return one ModuleGraph from one JSON value."""
    object_ = json_object(value)

    return ModuleGraph(
        profile=destack._generated.source.file.model.profile.from_json_profile_id(
            json_field(object_, "profile")
        ),
        modules=[
            destack._generated.source.file.model.module.from_json_module_id(item_0)
            for item_0 in json_array(json_field(object_, "modules"))
        ],
        edges=[
            [json_int(item_1) for item_1 in json_array(item_0)]
            for item_0 in json_array(json_field(object_, "edges"))
        ],
    )


@dataclass(frozen=True, slots=True)
class ComponentGraph:
    """Strongly connected component partition of one profile's module graph."""

    # the dense module graph being partitioned
    module_graph: ModuleGraph
    # per-module owning component
    component_of: Sequence[destack._generated.source.file.model.component.ComponentId]
    # components sorted by stable id
    components: Sequence[destack._generated.source.file.model.component.ComponentId]
    # per-component member start offsets into `member_modules`
    member_offsets: Sequence[int]
    # component members as module ids
    member_modules: Sequence[destack._generated.source.file.model.module.ModuleId]
    # component members as dense module indexes
    member_indexes: Sequence[int]
    # per-module dense component index
    module_components: Sequence[int]
    # per-component topological rank in the condensation graph
    component_ranks: Sequence[int]
    # external components each component depends on
    dependencies: Sequence[
        Sequence[destack._generated.source.file.model.component.ComponentId]
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
    encode_module_graph(writer, value.module_graph)
    writer.write_unsigned(len(value.component_of))
    for item_value_component_of_0 in value.component_of:
        destack._generated.source.file.model.component.encode_component_id(
            writer, item_value_component_of_0
        )
    writer.write_unsigned(len(value.components))
    for item_value_components_0 in value.components:
        destack._generated.source.file.model.component.encode_component_id(
            writer, item_value_components_0
        )
    writer.write_unsigned(len(value.member_offsets))
    for item_value_member_offsets_0 in value.member_offsets:
        writer.write_unsigned(item_value_member_offsets_0)
    writer.write_unsigned(len(value.member_modules))
    for item_value_member_modules_0 in value.member_modules:
        destack._generated.source.file.model.module.encode_module_id(
            writer, item_value_member_modules_0
        )
    writer.write_unsigned(len(value.member_indexes))
    for item_value_member_indexes_0 in value.member_indexes:
        writer.write_unsigned(item_value_member_indexes_0)
    writer.write_unsigned(len(value.module_components))
    for item_value_module_components_0 in value.module_components:
        writer.write_unsigned(item_value_module_components_0)
    writer.write_unsigned(len(value.component_ranks))
    for item_value_component_ranks_0 in value.component_ranks:
        writer.write_unsigned(item_value_component_ranks_0)
    writer.write_unsigned(len(value.dependencies))
    for item_value_dependencies_0 in value.dependencies:
        writer.write_unsigned(len(item_value_dependencies_0))
        for item_item_value_dependencies_0_1 in item_value_dependencies_0:
            destack._generated.source.file.model.component.encode_component_id(
                writer, item_item_value_dependencies_0_1
            )


def decode_component_graph(reader: BinaryReader) -> ComponentGraph:
    """Decode one ComponentGraph."""
    module_graph = decode_module_graph(reader)
    component_of = [
        destack._generated.source.file.model.component.decode_component_id(reader)
        for _ in range(reader.read_number())
    ]
    components = [
        destack._generated.source.file.model.component.decode_component_id(reader)
        for _ in range(reader.read_number())
    ]
    member_offsets = [reader.read_number() for _ in range(reader.read_number())]
    member_modules = [
        destack._generated.source.file.model.module.decode_module_id(reader)
        for _ in range(reader.read_number())
    ]
    member_indexes = [reader.read_number() for _ in range(reader.read_number())]
    module_components = [reader.read_number() for _ in range(reader.read_number())]
    component_ranks = [reader.read_number() for _ in range(reader.read_number())]
    dependencies = [
        [
            destack._generated.source.file.model.component.decode_component_id(reader)
            for _ in range(reader.read_number())
        ]
        for _ in range(reader.read_number())
    ]

    return ComponentGraph(
        module_graph=module_graph,
        component_of=component_of,
        components=components,
        member_offsets=member_offsets,
        member_modules=member_modules,
        member_indexes=member_indexes,
        module_components=module_components,
        component_ranks=component_ranks,
        dependencies=dependencies,
    )


def to_json_component_graph(value: ComponentGraph) -> Json:
    """Return one JSON value for one ComponentGraph."""
    return {
        "moduleGraph": to_json_module_graph(value.module_graph),
        "componentOf": [
            destack._generated.source.file.model.component.to_json_component_id(item_0)
            for item_0 in value.component_of
        ],
        "components": [
            destack._generated.source.file.model.component.to_json_component_id(item_0)
            for item_0 in value.components
        ],
        "memberOffsets": [item_0 for item_0 in value.member_offsets],
        "memberModules": [
            destack._generated.source.file.model.module.to_json_module_id(item_0)
            for item_0 in value.member_modules
        ],
        "memberIndexes": [item_0 for item_0 in value.member_indexes],
        "moduleComponents": [item_0 for item_0 in value.module_components],
        "componentRanks": [item_0 for item_0 in value.component_ranks],
        "dependencies": [
            [
                destack._generated.source.file.model.component.to_json_component_id(
                    item_1
                )
                for item_1 in item_0
            ]
            for item_0 in value.dependencies
        ],
    }


def from_json_component_graph(value: Json) -> ComponentGraph:
    """Return one ComponentGraph from one JSON value."""
    object_ = json_object(value)

    return ComponentGraph(
        module_graph=from_json_module_graph(json_field(object_, "moduleGraph")),
        component_of=[
            destack._generated.source.file.model.component.from_json_component_id(
                item_0
            )
            for item_0 in json_array(json_field(object_, "componentOf"))
        ],
        components=[
            destack._generated.source.file.model.component.from_json_component_id(
                item_0
            )
            for item_0 in json_array(json_field(object_, "components"))
        ],
        member_offsets=[
            json_int(item_0)
            for item_0 in json_array(json_field(object_, "memberOffsets"))
        ],
        member_modules=[
            destack._generated.source.file.model.module.from_json_module_id(item_0)
            for item_0 in json_array(json_field(object_, "memberModules"))
        ],
        member_indexes=[
            json_int(item_0)
            for item_0 in json_array(json_field(object_, "memberIndexes"))
        ],
        module_components=[
            json_int(item_0)
            for item_0 in json_array(json_field(object_, "moduleComponents"))
        ],
        component_ranks=[
            json_int(item_0)
            for item_0 in json_array(json_field(object_, "componentRanks"))
        ],
        dependencies=[
            [
                destack._generated.source.file.model.component.from_json_component_id(
                    item_1
                )
                for item_1 in json_array(item_0)
            ]
            for item_0 in json_array(json_field(object_, "dependencies"))
        ],
    )


__all__ = [
    "ModuleGraph",
    "encode_module_graph",
    "decode_module_graph",
    "to_json_module_graph",
    "from_json_module_graph",
    "ComponentGraph",
    "encode_component_graph",
    "decode_component_graph",
    "to_json_component_graph",
    "from_json_component_graph",
]
