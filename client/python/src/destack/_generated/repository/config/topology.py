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
    json_optional,
    json_string,
    nested_bytes,
)


@dataclass(frozen=True, slots=True)
class Topology:
    """Package topology definition."""

    # declared topology entities
    entities: Mapping[str, TopologyEntity]
    # declared topology edges
    edges: Mapping[str, TopologyEdge]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_topology(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Topology:
        """Decode one Topology."""
        return decode_topology(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_topology(self)

    @classmethod
    def from_json(cls, value: Json) -> Topology:
        """Return one Topology from one JSON value."""
        return from_json_topology(value)


def encode_topology(writer: BinaryWriter, value: Topology) -> None:
    """Encode one Topology."""
    entries_value_entities_0 = []
    for key_value_entities_0, item_value_entities_0 in value.entities.items():

        def write_key_value_entities_0(writer: BinaryWriter) -> None:
            writer.write_string(key_value_entities_0)

        key_bytes = nested_bytes(write_key_value_entities_0)
        entries_value_entities_0.append(
            (key_value_entities_0, item_value_entities_0, key_bytes)
        )
    entries_value_entities_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_entities_0))
    for entry_value_entities_0 in entries_value_entities_0:
        writer.write_string(entry_value_entities_0[0])
        encode_topology_entity(writer, entry_value_entities_0[1])
    entries_value_edges_0 = []
    for key_value_edges_0, item_value_edges_0 in value.edges.items():

        def write_key_value_edges_0(writer: BinaryWriter) -> None:
            writer.write_string(key_value_edges_0)

        key_bytes = nested_bytes(write_key_value_edges_0)
        entries_value_edges_0.append((key_value_edges_0, item_value_edges_0, key_bytes))
    entries_value_edges_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_edges_0))
    for entry_value_edges_0 in entries_value_edges_0:
        writer.write_string(entry_value_edges_0[0])
        encode_topology_edge(writer, entry_value_edges_0[1])


def decode_topology(reader: BinaryReader) -> Topology:
    """Decode one Topology."""
    entities = {
        reader.read_string(): decode_topology_entity(reader)
        for _ in range(reader.read_number())
    }
    edges = {
        reader.read_string(): decode_topology_edge(reader)
        for _ in range(reader.read_number())
    }

    return Topology(
        entities=entities,
        edges=edges,
    )


def to_json_topology(value: Topology) -> Json:
    """Return one JSON value for one Topology."""
    return {
        "entities": {
            key_0: to_json_topology_entity(item_0)
            for key_0, item_0 in value.entities.items()
        },
        "edges": {
            key_0: to_json_topology_edge(item_0)
            for key_0, item_0 in value.edges.items()
        },
    }


def from_json_topology(value: Json) -> Topology:
    """Return one Topology from one JSON value."""
    object_ = json_object(value)

    return Topology(
        entities={
            key_0: from_json_topology_entity(item_0)
            for key_0, item_0 in json_object(json_field(object_, "entities")).items()
        },
        edges={
            key_0: from_json_topology_edge(item_0)
            for key_0, item_0 in json_object(json_field(object_, "edges")).items()
        },
    )


@dataclass(frozen=True, slots=True)
class TopologyEntity:
    """One declared topology entity."""

    # stable entity kind identifier
    kind: str
    # stable entity name
    name: str | None
    # entity labels
    labels: Mapping[str, str]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_topology_entity(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TopologyEntity:
        """Decode one TopologyEntity."""
        return decode_topology_entity(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_topology_entity(self)

    @classmethod
    def from_json(cls, value: Json) -> TopologyEntity:
        """Return one TopologyEntity from one JSON value."""
        return from_json_topology_entity(value)


def encode_topology_entity(writer: BinaryWriter, value: TopologyEntity) -> None:
    """Encode one TopologyEntity."""
    writer.write_string(value.kind)
    if value.name is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.name)
    entries_value_labels_0 = []
    for key_value_labels_0, item_value_labels_0 in value.labels.items():

        def write_key_value_labels_0(writer: BinaryWriter) -> None:
            writer.write_string(key_value_labels_0)

        key_bytes = nested_bytes(write_key_value_labels_0)
        entries_value_labels_0.append(
            (key_value_labels_0, item_value_labels_0, key_bytes)
        )
    entries_value_labels_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_labels_0))
    for entry_value_labels_0 in entries_value_labels_0:
        writer.write_string(entry_value_labels_0[0])
        writer.write_string(entry_value_labels_0[1])


def decode_topology_entity(reader: BinaryReader) -> TopologyEntity:
    """Decode one TopologyEntity."""
    kind = reader.read_string()
    name = reader.read_option(lambda: reader.read_string())
    labels = {
        reader.read_string(): reader.read_string() for _ in range(reader.read_number())
    }

    return TopologyEntity(
        kind=kind,
        name=name,
        labels=labels,
    )


def to_json_topology_entity(value: TopologyEntity) -> Json:
    """Return one JSON value for one TopologyEntity."""
    return {
        "kind": value.kind,
        **({} if value.name is None else {"name": value.name}),
        "labels": {key_0: item_0 for key_0, item_0 in value.labels.items()},
    }


def from_json_topology_entity(value: Json) -> TopologyEntity:
    """Return one TopologyEntity from one JSON value."""
    object_ = json_object(value)

    return TopologyEntity(
        kind=json_string(json_field(object_, "kind")),
        name=json_optional(object_, "name", lambda value: json_string(value)),
        labels={
            key_0: json_string(item_0)
            for key_0, item_0 in json_object(json_field(object_, "labels")).items()
        },
    )


@dataclass(frozen=True, slots=True)
class TopologyEdge:
    """One declared topology edge."""

    # stable edge kind identifier
    kind: str
    # stable edge name
    name: str | None
    # source node name
    from_: str
    # target node name
    to: str
    # edge labels
    labels: Mapping[str, str]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_topology_edge(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TopologyEdge:
        """Decode one TopologyEdge."""
        return decode_topology_edge(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_topology_edge(self)

    @classmethod
    def from_json(cls, value: Json) -> TopologyEdge:
        """Return one TopologyEdge from one JSON value."""
        return from_json_topology_edge(value)


def encode_topology_edge(writer: BinaryWriter, value: TopologyEdge) -> None:
    """Encode one TopologyEdge."""
    writer.write_string(value.kind)
    if value.name is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.name)
    writer.write_string(value.from_)
    writer.write_string(value.to)
    entries_value_labels_0 = []
    for key_value_labels_0, item_value_labels_0 in value.labels.items():

        def write_key_value_labels_0(writer: BinaryWriter) -> None:
            writer.write_string(key_value_labels_0)

        key_bytes = nested_bytes(write_key_value_labels_0)
        entries_value_labels_0.append(
            (key_value_labels_0, item_value_labels_0, key_bytes)
        )
    entries_value_labels_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_labels_0))
    for entry_value_labels_0 in entries_value_labels_0:
        writer.write_string(entry_value_labels_0[0])
        writer.write_string(entry_value_labels_0[1])


def decode_topology_edge(reader: BinaryReader) -> TopologyEdge:
    """Decode one TopologyEdge."""
    kind = reader.read_string()
    name = reader.read_option(lambda: reader.read_string())
    from_ = reader.read_string()
    to = reader.read_string()
    labels = {
        reader.read_string(): reader.read_string() for _ in range(reader.read_number())
    }

    return TopologyEdge(
        kind=kind,
        name=name,
        from_=from_,
        to=to,
        labels=labels,
    )


def to_json_topology_edge(value: TopologyEdge) -> Json:
    """Return one JSON value for one TopologyEdge."""
    return {
        "kind": value.kind,
        **({} if value.name is None else {"name": value.name}),
        "from": value.from_,
        "to": value.to,
        "labels": {key_0: item_0 for key_0, item_0 in value.labels.items()},
    }


def from_json_topology_edge(value: Json) -> TopologyEdge:
    """Return one TopologyEdge from one JSON value."""
    object_ = json_object(value)

    return TopologyEdge(
        kind=json_string(json_field(object_, "kind")),
        name=json_optional(object_, "name", lambda value: json_string(value)),
        from_=json_string(json_field(object_, "from")),
        to=json_string(json_field(object_, "to")),
        labels={
            key_0: json_string(item_0)
            for key_0, item_0 in json_object(json_field(object_, "labels")).items()
        },
    )


__all__ = [
    "Topology",
    "encode_topology",
    "decode_topology",
    "to_json_topology",
    "from_json_topology",
    "TopologyEntity",
    "encode_topology_entity",
    "decode_topology_entity",
    "to_json_topology_entity",
    "from_json_topology_entity",
    "TopologyEdge",
    "encode_topology_edge",
    "decode_topology_edge",
    "to_json_topology_edge",
    "from_json_topology_edge",
]
