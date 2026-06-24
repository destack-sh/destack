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
    json_object,
    json_string,
)

import destack._generated.query.core.target


@dataclass(frozen=True, slots=True)
class GotoDefinitionRequest:
    """Request goto definition at a cursor position."""

    # the queried position
    position: destack._generated.query.core.target.QueryPosition

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_goto_definition_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> GotoDefinitionRequest:
        """Decode one GotoDefinitionRequest."""
        return decode_goto_definition_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_goto_definition_request(self)

    @classmethod
    def from_json(cls, value: Json) -> GotoDefinitionRequest:
        """Return one GotoDefinitionRequest from one JSON value."""
        return from_json_goto_definition_request(value)


def encode_goto_definition_request(
    writer: BinaryWriter, value: GotoDefinitionRequest
) -> None:
    """Encode one GotoDefinitionRequest."""
    destack._generated.query.core.target.encode_query_position(writer, value.position)


def decode_goto_definition_request(reader: BinaryReader) -> GotoDefinitionRequest:
    """Decode one GotoDefinitionRequest."""
    position = destack._generated.query.core.target.decode_query_position(reader)

    return GotoDefinitionRequest(
        position=position,
    )


def to_json_goto_definition_request(value: GotoDefinitionRequest) -> Json:
    """Return one JSON value for one GotoDefinitionRequest."""
    return {
        "position": destack._generated.query.core.target.to_json_query_position(
            value.position
        ),
    }


def from_json_goto_definition_request(value: Json) -> GotoDefinitionRequest:
    """Return one GotoDefinitionRequest from one JSON value."""
    object_ = json_object(value)

    return GotoDefinitionRequest(
        position=destack._generated.query.core.target.from_json_query_position(
            json_field(object_, "position")
        ),
    )


@dataclass(frozen=True, slots=True)
class GotoDeclarationRequest:
    """Request goto declaration at a cursor position."""

    # the queried position
    position: destack._generated.query.core.target.QueryPosition

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_goto_declaration_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> GotoDeclarationRequest:
        """Decode one GotoDeclarationRequest."""
        return decode_goto_declaration_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_goto_declaration_request(self)

    @classmethod
    def from_json(cls, value: Json) -> GotoDeclarationRequest:
        """Return one GotoDeclarationRequest from one JSON value."""
        return from_json_goto_declaration_request(value)


def encode_goto_declaration_request(
    writer: BinaryWriter, value: GotoDeclarationRequest
) -> None:
    """Encode one GotoDeclarationRequest."""
    destack._generated.query.core.target.encode_query_position(writer, value.position)


def decode_goto_declaration_request(reader: BinaryReader) -> GotoDeclarationRequest:
    """Decode one GotoDeclarationRequest."""
    position = destack._generated.query.core.target.decode_query_position(reader)

    return GotoDeclarationRequest(
        position=position,
    )


def to_json_goto_declaration_request(value: GotoDeclarationRequest) -> Json:
    """Return one JSON value for one GotoDeclarationRequest."""
    return {
        "position": destack._generated.query.core.target.to_json_query_position(
            value.position
        ),
    }


def from_json_goto_declaration_request(value: Json) -> GotoDeclarationRequest:
    """Return one GotoDeclarationRequest from one JSON value."""
    object_ = json_object(value)

    return GotoDeclarationRequest(
        position=destack._generated.query.core.target.from_json_query_position(
            json_field(object_, "position")
        ),
    )


@dataclass(frozen=True, slots=True)
class GotoTypeDefinitionRequest:
    """Request goto type definition at a cursor position."""

    # the queried position
    position: destack._generated.query.core.target.QueryPosition

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_goto_type_definition_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> GotoTypeDefinitionRequest:
        """Decode one GotoTypeDefinitionRequest."""
        return decode_goto_type_definition_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_goto_type_definition_request(self)

    @classmethod
    def from_json(cls, value: Json) -> GotoTypeDefinitionRequest:
        """Return one GotoTypeDefinitionRequest from one JSON value."""
        return from_json_goto_type_definition_request(value)


def encode_goto_type_definition_request(
    writer: BinaryWriter, value: GotoTypeDefinitionRequest
) -> None:
    """Encode one GotoTypeDefinitionRequest."""
    destack._generated.query.core.target.encode_query_position(writer, value.position)


def decode_goto_type_definition_request(
    reader: BinaryReader,
) -> GotoTypeDefinitionRequest:
    """Decode one GotoTypeDefinitionRequest."""
    position = destack._generated.query.core.target.decode_query_position(reader)

    return GotoTypeDefinitionRequest(
        position=position,
    )


def to_json_goto_type_definition_request(value: GotoTypeDefinitionRequest) -> Json:
    """Return one JSON value for one GotoTypeDefinitionRequest."""
    return {
        "position": destack._generated.query.core.target.to_json_query_position(
            value.position
        ),
    }


def from_json_goto_type_definition_request(value: Json) -> GotoTypeDefinitionRequest:
    """Return one GotoTypeDefinitionRequest from one JSON value."""
    object_ = json_object(value)

    return GotoTypeDefinitionRequest(
        position=destack._generated.query.core.target.from_json_query_position(
            json_field(object_, "position")
        ),
    )


@dataclass(frozen=True, slots=True)
class GotoDefinitionResponse:
    """Response payload for goto definition queries."""

    # definition targets
    targets: Sequence[NavigationTarget]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_goto_definition_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> GotoDefinitionResponse:
        """Decode one GotoDefinitionResponse."""
        return decode_goto_definition_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_goto_definition_response(self)

    @classmethod
    def from_json(cls, value: Json) -> GotoDefinitionResponse:
        """Return one GotoDefinitionResponse from one JSON value."""
        return from_json_goto_definition_response(value)


def encode_goto_definition_response(
    writer: BinaryWriter, value: GotoDefinitionResponse
) -> None:
    """Encode one GotoDefinitionResponse."""
    writer.write_unsigned(len(value.targets))
    for item_value_targets_0 in value.targets:
        encode_navigation_target(writer, item_value_targets_0)


def decode_goto_definition_response(reader: BinaryReader) -> GotoDefinitionResponse:
    """Decode one GotoDefinitionResponse."""
    targets = [decode_navigation_target(reader) for _ in range(reader.read_number())]

    return GotoDefinitionResponse(
        targets=targets,
    )


def to_json_goto_definition_response(value: GotoDefinitionResponse) -> Json:
    """Return one JSON value for one GotoDefinitionResponse."""
    return {
        "targets": [to_json_navigation_target(item_0) for item_0 in value.targets],
    }


def from_json_goto_definition_response(value: Json) -> GotoDefinitionResponse:
    """Return one GotoDefinitionResponse from one JSON value."""
    object_ = json_object(value)

    return GotoDefinitionResponse(
        targets=[
            from_json_navigation_target(item_0)
            for item_0 in json_array(json_field(object_, "targets"))
        ],
    )


@dataclass(frozen=True, slots=True)
class NavigationTarget:
    """One navigation target."""

    # the target location and resolved identity
    target: destack._generated.query.core.target.QueryTarget
    # the relationship to the query origin
    relation: NavigationRelation

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_navigation_target(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> NavigationTarget:
        """Decode one NavigationTarget."""
        return decode_navigation_target(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_navigation_target(self)

    @classmethod
    def from_json(cls, value: Json) -> NavigationTarget:
        """Return one NavigationTarget from one JSON value."""
        return from_json_navigation_target(value)


def encode_navigation_target(writer: BinaryWriter, value: NavigationTarget) -> None:
    """Encode one NavigationTarget."""
    destack._generated.query.core.target.encode_query_target(writer, value.target)
    encode_navigation_relation(writer, value.relation)


def decode_navigation_target(reader: BinaryReader) -> NavigationTarget:
    """Decode one NavigationTarget."""
    target = destack._generated.query.core.target.decode_query_target(reader)
    relation = decode_navigation_relation(reader)

    return NavigationTarget(
        target=target,
        relation=relation,
    )


def to_json_navigation_target(value: NavigationTarget) -> Json:
    """Return one JSON value for one NavigationTarget."""
    return {
        "target": destack._generated.query.core.target.to_json_query_target(
            value.target
        ),
        "relation": to_json_navigation_relation(value.relation),
    }


def from_json_navigation_target(value: Json) -> NavigationTarget:
    """Return one NavigationTarget from one JSON value."""
    object_ = json_object(value)

    return NavigationTarget(
        target=destack._generated.query.core.target.from_json_query_target(
            json_field(object_, "target")
        ),
        relation=from_json_navigation_relation(json_field(object_, "relation")),
    )


"""Relationship between a navigation origin and target."""
NavigationRelation: typing.TypeAlias = (
    typing.Literal["declaration"]
    | typing.Literal["definition"]
    | typing.Literal["typeDefinition"]
    | typing.Literal["implementation"]
)


def encode_navigation_relation(writer: BinaryWriter, value: NavigationRelation) -> None:
    """Encode one NavigationRelation."""
    if value == "declaration":
        writer.write_unsigned(0)
    elif value == "definition":
        writer.write_unsigned(1)
    elif value == "typeDefinition":
        writer.write_unsigned(2)
    elif value == "implementation":
        writer.write_unsigned(3)
    else:
        raise SerdeError("unknown enum variant")


def decode_navigation_relation(reader: BinaryReader) -> NavigationRelation:
    """Decode one NavigationRelation."""
    variant = reader.read_number()

    if variant == 0:
        return "declaration"
    elif variant == 1:
        return "definition"
    elif variant == 2:
        return "typeDefinition"
    elif variant == 3:
        return "implementation"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_navigation_relation(value: NavigationRelation) -> Json:
    """Return one JSON value for one NavigationRelation."""
    return value


def from_json_navigation_relation(value: Json) -> NavigationRelation:
    """Return one NavigationRelation from one JSON value."""
    variant = json_string(value)

    if variant == "declaration":
        return "declaration"
    elif variant == "definition":
        return "definition"
    elif variant == "typeDefinition":
        return "typeDefinition"
    elif variant == "implementation":
        return "implementation"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class GotoDeclarationResponse:
    """Response payload for goto declaration queries."""

    # declaration targets
    targets: Sequence[NavigationTarget]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_goto_declaration_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> GotoDeclarationResponse:
        """Decode one GotoDeclarationResponse."""
        return decode_goto_declaration_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_goto_declaration_response(self)

    @classmethod
    def from_json(cls, value: Json) -> GotoDeclarationResponse:
        """Return one GotoDeclarationResponse from one JSON value."""
        return from_json_goto_declaration_response(value)


def encode_goto_declaration_response(
    writer: BinaryWriter, value: GotoDeclarationResponse
) -> None:
    """Encode one GotoDeclarationResponse."""
    writer.write_unsigned(len(value.targets))
    for item_value_targets_0 in value.targets:
        encode_navigation_target(writer, item_value_targets_0)


def decode_goto_declaration_response(reader: BinaryReader) -> GotoDeclarationResponse:
    """Decode one GotoDeclarationResponse."""
    targets = [decode_navigation_target(reader) for _ in range(reader.read_number())]

    return GotoDeclarationResponse(
        targets=targets,
    )


def to_json_goto_declaration_response(value: GotoDeclarationResponse) -> Json:
    """Return one JSON value for one GotoDeclarationResponse."""
    return {
        "targets": [to_json_navigation_target(item_0) for item_0 in value.targets],
    }


def from_json_goto_declaration_response(value: Json) -> GotoDeclarationResponse:
    """Return one GotoDeclarationResponse from one JSON value."""
    object_ = json_object(value)

    return GotoDeclarationResponse(
        targets=[
            from_json_navigation_target(item_0)
            for item_0 in json_array(json_field(object_, "targets"))
        ],
    )


@dataclass(frozen=True, slots=True)
class GotoTypeDefinitionResponse:
    """Response payload for goto type definition queries."""

    # type definition targets
    targets: Sequence[NavigationTarget]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_goto_type_definition_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> GotoTypeDefinitionResponse:
        """Decode one GotoTypeDefinitionResponse."""
        return decode_goto_type_definition_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_goto_type_definition_response(self)

    @classmethod
    def from_json(cls, value: Json) -> GotoTypeDefinitionResponse:
        """Return one GotoTypeDefinitionResponse from one JSON value."""
        return from_json_goto_type_definition_response(value)


def encode_goto_type_definition_response(
    writer: BinaryWriter, value: GotoTypeDefinitionResponse
) -> None:
    """Encode one GotoTypeDefinitionResponse."""
    writer.write_unsigned(len(value.targets))
    for item_value_targets_0 in value.targets:
        encode_navigation_target(writer, item_value_targets_0)


def decode_goto_type_definition_response(
    reader: BinaryReader,
) -> GotoTypeDefinitionResponse:
    """Decode one GotoTypeDefinitionResponse."""
    targets = [decode_navigation_target(reader) for _ in range(reader.read_number())]

    return GotoTypeDefinitionResponse(
        targets=targets,
    )


def to_json_goto_type_definition_response(value: GotoTypeDefinitionResponse) -> Json:
    """Return one JSON value for one GotoTypeDefinitionResponse."""
    return {
        "targets": [to_json_navigation_target(item_0) for item_0 in value.targets],
    }


def from_json_goto_type_definition_response(value: Json) -> GotoTypeDefinitionResponse:
    """Return one GotoTypeDefinitionResponse from one JSON value."""
    object_ = json_object(value)

    return GotoTypeDefinitionResponse(
        targets=[
            from_json_navigation_target(item_0)
            for item_0 in json_array(json_field(object_, "targets"))
        ],
    )


__all__ = [
    "GotoDefinitionRequest",
    "encode_goto_definition_request",
    "decode_goto_definition_request",
    "to_json_goto_definition_request",
    "from_json_goto_definition_request",
    "GotoDeclarationRequest",
    "encode_goto_declaration_request",
    "decode_goto_declaration_request",
    "to_json_goto_declaration_request",
    "from_json_goto_declaration_request",
    "GotoTypeDefinitionRequest",
    "encode_goto_type_definition_request",
    "decode_goto_type_definition_request",
    "to_json_goto_type_definition_request",
    "from_json_goto_type_definition_request",
    "GotoDefinitionResponse",
    "encode_goto_definition_response",
    "decode_goto_definition_response",
    "to_json_goto_definition_response",
    "from_json_goto_definition_response",
    "NavigationTarget",
    "encode_navigation_target",
    "decode_navigation_target",
    "to_json_navigation_target",
    "from_json_navigation_target",
    "NavigationRelation",
    "encode_navigation_relation",
    "decode_navigation_relation",
    "to_json_navigation_relation",
    "from_json_navigation_relation",
    "GotoDeclarationResponse",
    "encode_goto_declaration_response",
    "decode_goto_declaration_response",
    "to_json_goto_declaration_response",
    "from_json_goto_declaration_response",
    "GotoTypeDefinitionResponse",
    "encode_goto_type_definition_response",
    "decode_goto_type_definition_response",
    "to_json_goto_type_definition_response",
    "from_json_goto_type_definition_response",
]
