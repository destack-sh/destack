# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.query.core.target

if TYPE_CHECKING:
    from destack._generated.protocol.query.core.target import (
        QueryPosition,
        QueryTarget,
    )


@dataclass(frozen=True, slots=True)
class GotoDefinitionRequest:
    """Request goto definition at a cursor position."""

    """The queried position."""
    position: QueryPosition


def encode_goto_definition_request(
    writer: Writer, value: GotoDefinitionRequest
) -> None:
    destack._generated.protocol.query.core.target.encode_query_position(
        writer, value.position
    )


def decode_goto_definition_request(reader: Reader) -> GotoDefinitionRequest:
    field_0 = destack._generated.protocol.query.core.target.decode_query_position(
        reader
    )

    return GotoDefinitionRequest(
        position=field_0,
    )


@dataclass(frozen=True, slots=True)
class GotoDeclarationRequest:
    """Request goto declaration at a cursor position."""

    """The queried position."""
    position: QueryPosition


def encode_goto_declaration_request(
    writer: Writer, value: GotoDeclarationRequest
) -> None:
    destack._generated.protocol.query.core.target.encode_query_position(
        writer, value.position
    )


def decode_goto_declaration_request(reader: Reader) -> GotoDeclarationRequest:
    field_0 = destack._generated.protocol.query.core.target.decode_query_position(
        reader
    )

    return GotoDeclarationRequest(
        position=field_0,
    )


@dataclass(frozen=True, slots=True)
class GotoTypeDefinitionRequest:
    """Request goto type definition at a cursor position."""

    """The queried position."""
    position: QueryPosition


def encode_goto_type_definition_request(
    writer: Writer, value: GotoTypeDefinitionRequest
) -> None:
    destack._generated.protocol.query.core.target.encode_query_position(
        writer, value.position
    )


def decode_goto_type_definition_request(reader: Reader) -> GotoTypeDefinitionRequest:
    field_0 = destack._generated.protocol.query.core.target.decode_query_position(
        reader
    )

    return GotoTypeDefinitionRequest(
        position=field_0,
    )


@dataclass(frozen=True, slots=True)
class GotoDefinitionResponse:
    """Response payload for goto definition queries."""

    """Definition targets."""
    targets: Sequence[NavigationTarget]


def encode_goto_definition_response(
    writer: Writer, value: GotoDefinitionResponse
) -> None:
    writer.write_unsigned(len(value.targets))
    for item_0 in value.targets:
        encode_navigation_target(writer, item_0)


def decode_goto_definition_response(reader: Reader) -> GotoDefinitionResponse:
    field_0 = [decode_navigation_target(reader) for _ in range(reader.read_number())]

    return GotoDefinitionResponse(
        targets=field_0,
    )


@dataclass(frozen=True, slots=True)
class NavigationTarget:
    """One navigation target."""

    """The target location and resolved identity."""
    target: QueryTarget
    """The relationship to the query origin."""
    relation: NavigationRelation


def encode_navigation_target(writer: Writer, value: NavigationTarget) -> None:
    destack._generated.protocol.query.core.target.encode_query_target(
        writer, value.target
    )
    encode_navigation_relation(writer, value.relation)


def decode_navigation_target(reader: Reader) -> NavigationTarget:
    field_0 = destack._generated.protocol.query.core.target.decode_query_target(reader)
    field_1 = decode_navigation_relation(reader)

    return NavigationTarget(
        target=field_0,
        relation=field_1,
    )


"""Relationship between a navigation origin and target."""
NavigationRelation: TypeAlias = (
    Literal["declaration"]
    | Literal["definition"]
    | Literal["typeDefinition"]
    | Literal["implementation"]
)


def encode_navigation_relation(writer: Writer, value: NavigationRelation) -> None:
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


def decode_navigation_relation(reader: Reader) -> NavigationRelation:
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


@dataclass(frozen=True, slots=True)
class GotoDeclarationResponse:
    """Response payload for goto declaration queries."""

    """Declaration targets."""
    targets: Sequence[NavigationTarget]


def encode_goto_declaration_response(
    writer: Writer, value: GotoDeclarationResponse
) -> None:
    writer.write_unsigned(len(value.targets))
    for item_0 in value.targets:
        encode_navigation_target(writer, item_0)


def decode_goto_declaration_response(reader: Reader) -> GotoDeclarationResponse:
    field_0 = [decode_navigation_target(reader) for _ in range(reader.read_number())]

    return GotoDeclarationResponse(
        targets=field_0,
    )


@dataclass(frozen=True, slots=True)
class GotoTypeDefinitionResponse:
    """Response payload for goto type definition queries."""

    """Type definition targets."""
    targets: Sequence[NavigationTarget]


def encode_goto_type_definition_response(
    writer: Writer, value: GotoTypeDefinitionResponse
) -> None:
    writer.write_unsigned(len(value.targets))
    for item_0 in value.targets:
        encode_navigation_target(writer, item_0)


def decode_goto_type_definition_response(reader: Reader) -> GotoTypeDefinitionResponse:
    field_0 = [decode_navigation_target(reader) for _ in range(reader.read_number())]

    return GotoTypeDefinitionResponse(
        targets=field_0,
    )


__all__ = [
    "GotoDefinitionRequest",
    "encode_goto_definition_request",
    "decode_goto_definition_request",
    "GotoDeclarationRequest",
    "encode_goto_declaration_request",
    "decode_goto_declaration_request",
    "GotoTypeDefinitionRequest",
    "encode_goto_type_definition_request",
    "decode_goto_type_definition_request",
    "GotoDefinitionResponse",
    "encode_goto_definition_response",
    "decode_goto_definition_response",
    "NavigationTarget",
    "encode_navigation_target",
    "decode_navigation_target",
    "NavigationRelation",
    "encode_navigation_relation",
    "decode_navigation_relation",
    "GotoDeclarationResponse",
    "encode_goto_declaration_response",
    "decode_goto_declaration_response",
    "GotoTypeDefinitionResponse",
    "encode_goto_type_definition_response",
    "decode_goto_type_definition_response",
]
