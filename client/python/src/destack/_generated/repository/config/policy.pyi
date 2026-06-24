# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.repository.config.condition
import destack._generated.repository.config.runtime.selectors

@dataclass(frozen=True, slots=True)
class Policy:
    """Static policy configuration."""

    # package-declared policy requirements
    requires: Sequence[PolicyRequirement]
    # ordered authorization rules
    rules: Sequence[PolicyRule]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Policy: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Policy: ...

def encode_policy(writer: BinaryWriter, value: Policy) -> None: ...
def decode_policy(reader: BinaryReader) -> Policy: ...
def to_json_policy(value: Policy) -> Json: ...
def from_json_policy(value: Json) -> Policy: ...

@dataclass(frozen=True, slots=True)
class PolicyRequirement:
    """Declared policy action required by one package."""

    # policy domain where the action is exercised
    domain: PolicyDomain
    # action name required by package code
    action: str
    # resource scope touched by the action
    resource: str

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PolicyRequirement: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PolicyRequirement: ...

def encode_policy_requirement(
    writer: BinaryWriter, value: PolicyRequirement
) -> None: ...
def decode_policy_requirement(reader: BinaryReader) -> PolicyRequirement: ...
def to_json_policy_requirement(value: PolicyRequirement) -> Json: ...
def from_json_policy_requirement(value: Json) -> PolicyRequirement: ...

"""Policy domain where one action is exercised."""
PolicyDomain: typing.TypeAlias = typing.Literal["comptime"] | typing.Literal["runtime"]

def encode_policy_domain(writer: BinaryWriter, value: PolicyDomain) -> None: ...
def decode_policy_domain(reader: BinaryReader) -> PolicyDomain: ...
def to_json_policy_domain(value: PolicyDomain) -> Json: ...
def from_json_policy_domain(value: Json) -> PolicyDomain: ...

@dataclass(frozen=True, slots=True)
class PolicyRule:
    """Policy rule for package, target, or runtime evaluation."""

    # policy domain where the action is exercised
    domain: PolicyDomain
    # subject matched by this rule
    subject: PolicySubject
    # action name matched by this rule
    action: str
    # resource scope matched by this rule
    resource: str
    # access outcome selected by this rule
    access: PolicyAccess

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PolicyRule: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PolicyRule: ...

def encode_policy_rule(writer: BinaryWriter, value: PolicyRule) -> None: ...
def decode_policy_rule(reader: BinaryReader) -> PolicyRule: ...
def to_json_policy_rule(value: PolicyRule) -> Json: ...
def from_json_policy_rule(value: Json) -> PolicyRule: ...

@dataclass(frozen=True, slots=True)
class PolicySubject:
    """Policy subject matched by one rule."""

    # package selector
    package: PackageSelector | None
    # runtime identity selector
    runtime_identity: (
        destack._generated.repository.config.runtime.selectors.RuntimeIdentitySelector
        | None
    )
    # worker identity selector
    worker: (
        destack._generated.repository.config.runtime.selectors.RuntimeIdentitySelector
        | None
    )
    # active mode selector
    mode: destack._generated.repository.config.condition.ConditionSelector | None
    # active role selector
    role: destack._generated.repository.config.condition.ConditionSelector | None
    # active feature selector
    feature: destack._generated.repository.config.condition.ConditionSelector | None
    # active tag selector
    tag: destack._generated.repository.config.condition.ConditionSelector | None
    # active target selector
    target: destack._generated.repository.config.condition.ConditionSelector | None
    # active product selector
    product: destack._generated.repository.config.condition.ConditionSelector | None
    # active target platform selector
    platform: destack._generated.repository.config.condition.ConditionSelector | None
    # active host environment selector
    host: destack._generated.repository.config.condition.ConditionSelector | None
    # active runtime selector
    runtime: destack._generated.repository.config.condition.ConditionSelector | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PolicySubject: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PolicySubject: ...

def encode_policy_subject(writer: BinaryWriter, value: PolicySubject) -> None: ...
def decode_policy_subject(reader: BinaryReader) -> PolicySubject: ...
def to_json_policy_subject(value: PolicySubject) -> Json: ...
def from_json_policy_subject(value: Json) -> PolicySubject: ...

@dataclass(frozen=True, slots=True)
class PackageSelector:
    """Package selector used by static policy rules."""

    # package name glob patterns
    patterns: Sequence[str]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PackageSelector: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PackageSelector: ...

def encode_package_selector(writer: BinaryWriter, value: PackageSelector) -> None: ...
def decode_package_selector(reader: BinaryReader) -> PackageSelector: ...
def to_json_package_selector(value: PackageSelector) -> Json: ...
def from_json_package_selector(value: Json) -> PackageSelector: ...

"""Policy access result selected by one rule."""
PolicyAccess: typing.TypeAlias = typing.Literal["allow"] | typing.Literal["deny"]

def encode_policy_access(writer: BinaryWriter, value: PolicyAccess) -> None: ...
def decode_policy_access(reader: BinaryReader) -> PolicyAccess: ...
def to_json_policy_access(value: PolicyAccess) -> Json: ...
def from_json_policy_access(value: Json) -> PolicyAccess: ...

__all__ = [
    "Policy",
    "encode_policy",
    "decode_policy",
    "to_json_policy",
    "from_json_policy",
    "PolicyRequirement",
    "encode_policy_requirement",
    "decode_policy_requirement",
    "to_json_policy_requirement",
    "from_json_policy_requirement",
    "PolicyDomain",
    "encode_policy_domain",
    "decode_policy_domain",
    "to_json_policy_domain",
    "from_json_policy_domain",
    "PolicyRule",
    "encode_policy_rule",
    "decode_policy_rule",
    "to_json_policy_rule",
    "from_json_policy_rule",
    "PolicySubject",
    "encode_policy_subject",
    "decode_policy_subject",
    "to_json_policy_subject",
    "from_json_policy_subject",
    "PackageSelector",
    "encode_package_selector",
    "decode_package_selector",
    "to_json_package_selector",
    "from_json_package_selector",
    "PolicyAccess",
    "encode_policy_access",
    "decode_policy_access",
    "to_json_policy_access",
    "from_json_policy_access",
]
