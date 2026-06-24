# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

@dataclass(frozen=True, slots=True)
class RuntimeIdentitySelector:
    """Runtime identity selector for worker and runtime scopes."""

    # name selector for one runtime or one worker
    name: str | None
    # label selector for one runtime or one worker
    labels: RuntimeLabelSelector | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> RuntimeIdentitySelector: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> RuntimeIdentitySelector: ...

def encode_runtime_identity_selector(
    writer: BinaryWriter, value: RuntimeIdentitySelector
) -> None: ...
def decode_runtime_identity_selector(
    reader: BinaryReader,
) -> RuntimeIdentitySelector: ...
def to_json_runtime_identity_selector(value: RuntimeIdentitySelector) -> Json: ...
def from_json_runtime_identity_selector(value: Json) -> RuntimeIdentitySelector: ...

@dataclass(frozen=True, slots=True)
class RuntimeLabelSelector:
    """Kubernetes-style label selector for runtime identity."""

    # exact-match labels that must all be present
    match_labels: Mapping[str, str]
    # additional set-based label requirements
    match_expressions: Sequence[RuntimeLabelRequirement]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> RuntimeLabelSelector: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> RuntimeLabelSelector: ...

def encode_runtime_label_selector(
    writer: BinaryWriter, value: RuntimeLabelSelector
) -> None: ...
def decode_runtime_label_selector(reader: BinaryReader) -> RuntimeLabelSelector: ...
def to_json_runtime_label_selector(value: RuntimeLabelSelector) -> Json: ...
def from_json_runtime_label_selector(value: Json) -> RuntimeLabelSelector: ...

@dataclass(frozen=True, slots=True)
class RuntimeLabelRequirement:
    """One label requirement clause for runtime identity selectors."""

    # label key to evaluate
    key: str
    # label requirement operator
    operator: RuntimeLabelOperator
    # label values for set-based operators
    values: Sequence[str]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> RuntimeLabelRequirement: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> RuntimeLabelRequirement: ...

def encode_runtime_label_requirement(
    writer: BinaryWriter, value: RuntimeLabelRequirement
) -> None: ...
def decode_runtime_label_requirement(
    reader: BinaryReader,
) -> RuntimeLabelRequirement: ...
def to_json_runtime_label_requirement(value: RuntimeLabelRequirement) -> Json: ...
def from_json_runtime_label_requirement(value: Json) -> RuntimeLabelRequirement: ...

"""Label selection operator for runtime identity selectors."""
RuntimeLabelOperator: typing.TypeAlias = (
    typing.Literal["in"]
    | typing.Literal["notIn"]
    | typing.Literal["exists"]
    | typing.Literal["doesNotExist"]
)

def encode_runtime_label_operator(
    writer: BinaryWriter, value: RuntimeLabelOperator
) -> None: ...
def decode_runtime_label_operator(reader: BinaryReader) -> RuntimeLabelOperator: ...
def to_json_runtime_label_operator(value: RuntimeLabelOperator) -> Json: ...
def from_json_runtime_label_operator(value: Json) -> RuntimeLabelOperator: ...

__all__ = [
    "RuntimeIdentitySelector",
    "encode_runtime_identity_selector",
    "decode_runtime_identity_selector",
    "to_json_runtime_identity_selector",
    "from_json_runtime_identity_selector",
    "RuntimeLabelSelector",
    "encode_runtime_label_selector",
    "decode_runtime_label_selector",
    "to_json_runtime_label_selector",
    "from_json_runtime_label_selector",
    "RuntimeLabelRequirement",
    "encode_runtime_label_requirement",
    "decode_runtime_label_requirement",
    "to_json_runtime_label_requirement",
    "from_json_runtime_label_requirement",
    "RuntimeLabelOperator",
    "encode_runtime_label_operator",
    "decode_runtime_label_operator",
    "to_json_runtime_label_operator",
    "from_json_runtime_label_operator",
]
