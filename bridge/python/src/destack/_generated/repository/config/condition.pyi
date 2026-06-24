# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.repository.config.dependency

@dataclass(frozen=True, slots=True)
class ConditionRefName:
    """Named condition alias or `axis:name` reference."""

    name: str
    kind: typing.Literal["name"] = "name"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ConditionRefPredicate:
    """Inline condition gate."""

    predicate: ConditionPredicate
    kind: typing.Literal["predicate"] = "predicate"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Source-level reference to one active condition predicate."""
ConditionRef: typing.TypeAlias = ConditionRefName | ConditionRefPredicate

def encode_condition_ref(writer: BinaryWriter, value: ConditionRef) -> None: ...
def decode_condition_ref(reader: BinaryReader) -> ConditionRef: ...
def to_json_condition_ref(value: ConditionRef) -> Json: ...
def from_json_condition_ref(value: Json) -> ConditionRef: ...

@dataclass(frozen=True, slots=True)
class ConditionPredicate:
    """Declared condition predicate before named references are resolved."""

    # active mode selector
    mode: ConditionSelector | None
    # active role selector
    role: ConditionSelector | None
    # active feature selector
    feature: ConditionSelector | None
    # active tag selector
    tag: ConditionSelector | None
    # active build target selector
    target: ConditionSelector | None
    # active product selector
    product: ConditionSelector | None
    # active package release stage selector
    stage: ConditionSelector | None
    # active target platform selector
    platform: ConditionSelector | None
    # active host environment selector
    host: ConditionSelector | None
    # active runtime selector
    runtime: ConditionSelector | None
    # predicates that must all match
    all: Sequence[ConditionRef] | None
    # predicates where at least one must match
    any: Sequence[ConditionRef] | None
    # predicate that must not match
    not_: ConditionRef | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ConditionPredicate: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ConditionPredicate: ...

def encode_condition_predicate(
    writer: BinaryWriter, value: ConditionPredicate
) -> None: ...
def decode_condition_predicate(reader: BinaryReader) -> ConditionPredicate: ...
def to_json_condition_predicate(value: ConditionPredicate) -> Json: ...
def from_json_condition_predicate(value: Json) -> ConditionPredicate: ...

@dataclass(frozen=True, slots=True)
class ConditionSelector:
    """Selector over one active condition axis."""

    # condition names or glob patterns
    patterns: Sequence[str]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ConditionSelector: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ConditionSelector: ...

def encode_condition_selector(
    writer: BinaryWriter, value: ConditionSelector
) -> None: ...
def decode_condition_selector(reader: BinaryReader) -> ConditionSelector: ...
def to_json_condition_selector(value: ConditionSelector) -> Json: ...
def from_json_condition_selector(value: Json) -> ConditionSelector: ...

@dataclass(frozen=True, slots=True)
class ConditionCatalog:
    """Named condition declarations from `destack.json`."""

    # named source graph modes
    modes: Mapping[str, Condition]
    # named source graph roles
    roles: Mapping[str, Condition]
    # named optional source graph features
    features: Mapping[str, Condition]
    # named source graph tags
    tags: Mapping[str, Condition]
    # named condition aliases
    aliases: Mapping[str, ConditionRef]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ConditionCatalog: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ConditionCatalog: ...

def encode_condition_catalog(writer: BinaryWriter, value: ConditionCatalog) -> None: ...
def decode_condition_catalog(reader: BinaryReader) -> ConditionCatalog: ...
def to_json_condition_catalog(value: ConditionCatalog) -> Json: ...
def from_json_condition_catalog(value: Json) -> ConditionCatalog: ...

@dataclass(frozen=True, slots=True)
class Condition:
    """Named source graph condition."""

    # human-readable condition description
    description: str | None
    # condition labels
    labels: Mapping[str, str]
    # condition names included before this condition
    extends: Sequence[str]
    # dependencies enabled by this condition
    dependencies: Mapping[
        str, destack._generated.repository.config.dependency.Dependency
    ]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Condition: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Condition: ...

def encode_condition(writer: BinaryWriter, value: Condition) -> None: ...
def decode_condition(reader: BinaryReader) -> Condition: ...
def to_json_condition(value: Condition) -> Json: ...
def from_json_condition(value: Json) -> Condition: ...

__all__ = [
    "ConditionRef",
    "encode_condition_ref",
    "decode_condition_ref",
    "to_json_condition_ref",
    "from_json_condition_ref",
    "ConditionRefName",
    "ConditionRefPredicate",
    "ConditionPredicate",
    "encode_condition_predicate",
    "decode_condition_predicate",
    "to_json_condition_predicate",
    "from_json_condition_predicate",
    "ConditionSelector",
    "encode_condition_selector",
    "decode_condition_selector",
    "to_json_condition_selector",
    "from_json_condition_selector",
    "ConditionCatalog",
    "encode_condition_catalog",
    "decode_condition_catalog",
    "to_json_condition_catalog",
    "from_json_condition_catalog",
    "Condition",
    "encode_condition",
    "decode_condition",
    "to_json_condition",
    "from_json_condition",
]
