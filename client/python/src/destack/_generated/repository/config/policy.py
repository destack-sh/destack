# generated client target, do not edit

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
    json_optional,
    json_string,
)

import destack._generated.repository.config.condition
import destack._generated.repository.config.runtime.selectors


@dataclass(frozen=True, slots=True)
class Policy:
    """Static policy configuration."""

    # package-declared policy requirements
    requires: Sequence[PolicyRequirement]
    # ordered authorization rules
    rules: Sequence[PolicyRule]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_policy(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Policy:
        """Decode one Policy."""
        return decode_policy(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_policy(self)

    @classmethod
    def from_json(cls, value: Json) -> Policy:
        """Return one Policy from one JSON value."""
        return from_json_policy(value)


def encode_policy(writer: BinaryWriter, value: Policy) -> None:
    """Encode one Policy."""
    writer.write_unsigned(len(value.requires))
    for item_value_requires_0 in value.requires:
        encode_policy_requirement(writer, item_value_requires_0)
    writer.write_unsigned(len(value.rules))
    for item_value_rules_0 in value.rules:
        encode_policy_rule(writer, item_value_rules_0)


def decode_policy(reader: BinaryReader) -> Policy:
    """Decode one Policy."""
    requires = [decode_policy_requirement(reader) for _ in range(reader.read_number())]
    rules = [decode_policy_rule(reader) for _ in range(reader.read_number())]

    return Policy(
        requires=requires,
        rules=rules,
    )


def to_json_policy(value: Policy) -> Json:
    """Return one JSON value for one Policy."""
    return {
        "requires": [to_json_policy_requirement(item_0) for item_0 in value.requires],
        "rules": [to_json_policy_rule(item_0) for item_0 in value.rules],
    }


def from_json_policy(value: Json) -> Policy:
    """Return one Policy from one JSON value."""
    object_ = json_object(value)

    return Policy(
        requires=[
            from_json_policy_requirement(item_0)
            for item_0 in json_array(json_field(object_, "requires"))
        ],
        rules=[
            from_json_policy_rule(item_0)
            for item_0 in json_array(json_field(object_, "rules"))
        ],
    )


@dataclass(frozen=True, slots=True)
class PolicyRequirement:
    """Declared policy action required by one package."""

    # policy domain where the action is exercised
    domain: PolicyDomain
    # action name required by package code
    action: str
    # resource scope touched by the action
    resource: str

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_policy_requirement(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PolicyRequirement:
        """Decode one PolicyRequirement."""
        return decode_policy_requirement(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_policy_requirement(self)

    @classmethod
    def from_json(cls, value: Json) -> PolicyRequirement:
        """Return one PolicyRequirement from one JSON value."""
        return from_json_policy_requirement(value)


def encode_policy_requirement(writer: BinaryWriter, value: PolicyRequirement) -> None:
    """Encode one PolicyRequirement."""
    encode_policy_domain(writer, value.domain)
    writer.write_string(value.action)
    writer.write_string(value.resource)


def decode_policy_requirement(reader: BinaryReader) -> PolicyRequirement:
    """Decode one PolicyRequirement."""
    domain = decode_policy_domain(reader)
    action = reader.read_string()
    resource = reader.read_string()

    return PolicyRequirement(
        domain=domain,
        action=action,
        resource=resource,
    )


def to_json_policy_requirement(value: PolicyRequirement) -> Json:
    """Return one JSON value for one PolicyRequirement."""
    return {
        "domain": to_json_policy_domain(value.domain),
        "action": value.action,
        "resource": value.resource,
    }


def from_json_policy_requirement(value: Json) -> PolicyRequirement:
    """Return one PolicyRequirement from one JSON value."""
    object_ = json_object(value)

    return PolicyRequirement(
        domain=from_json_policy_domain(json_field(object_, "domain")),
        action=json_string(json_field(object_, "action")),
        resource=json_string(json_field(object_, "resource")),
    )


"""Policy domain where one action is exercised."""
PolicyDomain: typing.TypeAlias = typing.Literal["comptime"] | typing.Literal["runtime"]


def encode_policy_domain(writer: BinaryWriter, value: PolicyDomain) -> None:
    """Encode one PolicyDomain."""
    if value == "comptime":
        writer.write_unsigned(0)
    elif value == "runtime":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_policy_domain(reader: BinaryReader) -> PolicyDomain:
    """Decode one PolicyDomain."""
    variant = reader.read_number()

    if variant == 0:
        return "comptime"
    elif variant == 1:
        return "runtime"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_policy_domain(value: PolicyDomain) -> Json:
    """Return one JSON value for one PolicyDomain."""
    return value


def from_json_policy_domain(value: Json) -> PolicyDomain:
    """Return one PolicyDomain from one JSON value."""
    variant = json_string(value)

    if variant == "comptime":
        return "comptime"
    elif variant == "runtime":
        return "runtime"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_policy_rule(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PolicyRule:
        """Decode one PolicyRule."""
        return decode_policy_rule(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_policy_rule(self)

    @classmethod
    def from_json(cls, value: Json) -> PolicyRule:
        """Return one PolicyRule from one JSON value."""
        return from_json_policy_rule(value)


def encode_policy_rule(writer: BinaryWriter, value: PolicyRule) -> None:
    """Encode one PolicyRule."""
    encode_policy_domain(writer, value.domain)
    encode_policy_subject(writer, value.subject)
    writer.write_string(value.action)
    writer.write_string(value.resource)
    encode_policy_access(writer, value.access)


def decode_policy_rule(reader: BinaryReader) -> PolicyRule:
    """Decode one PolicyRule."""
    domain = decode_policy_domain(reader)
    subject = decode_policy_subject(reader)
    action = reader.read_string()
    resource = reader.read_string()
    access = decode_policy_access(reader)

    return PolicyRule(
        domain=domain,
        subject=subject,
        action=action,
        resource=resource,
        access=access,
    )


def to_json_policy_rule(value: PolicyRule) -> Json:
    """Return one JSON value for one PolicyRule."""
    return {
        "domain": to_json_policy_domain(value.domain),
        "subject": to_json_policy_subject(value.subject),
        "action": value.action,
        "resource": value.resource,
        "access": to_json_policy_access(value.access),
    }


def from_json_policy_rule(value: Json) -> PolicyRule:
    """Return one PolicyRule from one JSON value."""
    object_ = json_object(value)

    return PolicyRule(
        domain=from_json_policy_domain(json_field(object_, "domain")),
        subject=from_json_policy_subject(json_field(object_, "subject")),
        action=json_string(json_field(object_, "action")),
        resource=json_string(json_field(object_, "resource")),
        access=from_json_policy_access(json_field(object_, "access")),
    )


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_policy_subject(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PolicySubject:
        """Decode one PolicySubject."""
        return decode_policy_subject(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_policy_subject(self)

    @classmethod
    def from_json(cls, value: Json) -> PolicySubject:
        """Return one PolicySubject from one JSON value."""
        return from_json_policy_subject(value)


def encode_policy_subject(writer: BinaryWriter, value: PolicySubject) -> None:
    """Encode one PolicySubject."""
    if value.package is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_package_selector(writer, value.package)
    if value.runtime_identity is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.repository.config.runtime.selectors.encode_runtime_identity_selector(
            writer, value.runtime_identity
        )
    if value.worker is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.repository.config.runtime.selectors.encode_runtime_identity_selector(
            writer, value.worker
        )
    if value.mode is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.repository.config.condition.encode_condition_selector(
            writer, value.mode
        )
    if value.role is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.repository.config.condition.encode_condition_selector(
            writer, value.role
        )
    if value.feature is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.repository.config.condition.encode_condition_selector(
            writer, value.feature
        )
    if value.tag is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.repository.config.condition.encode_condition_selector(
            writer, value.tag
        )
    if value.target is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.repository.config.condition.encode_condition_selector(
            writer, value.target
        )
    if value.product is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.repository.config.condition.encode_condition_selector(
            writer, value.product
        )
    if value.platform is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.repository.config.condition.encode_condition_selector(
            writer, value.platform
        )
    if value.host is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.repository.config.condition.encode_condition_selector(
            writer, value.host
        )
    if value.runtime is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.repository.config.condition.encode_condition_selector(
            writer, value.runtime
        )


def decode_policy_subject(reader: BinaryReader) -> PolicySubject:
    """Decode one PolicySubject."""
    package = reader.read_option(lambda: decode_package_selector(reader))
    runtime_identity = reader.read_option(
        lambda: (
            destack._generated.repository.config.runtime.selectors.decode_runtime_identity_selector(
                reader
            )
        )
    )
    worker = reader.read_option(
        lambda: (
            destack._generated.repository.config.runtime.selectors.decode_runtime_identity_selector(
                reader
            )
        )
    )
    mode = reader.read_option(
        lambda: (
            destack._generated.repository.config.condition.decode_condition_selector(
                reader
            )
        )
    )
    role = reader.read_option(
        lambda: (
            destack._generated.repository.config.condition.decode_condition_selector(
                reader
            )
        )
    )
    feature = reader.read_option(
        lambda: (
            destack._generated.repository.config.condition.decode_condition_selector(
                reader
            )
        )
    )
    tag = reader.read_option(
        lambda: (
            destack._generated.repository.config.condition.decode_condition_selector(
                reader
            )
        )
    )
    target = reader.read_option(
        lambda: (
            destack._generated.repository.config.condition.decode_condition_selector(
                reader
            )
        )
    )
    product = reader.read_option(
        lambda: (
            destack._generated.repository.config.condition.decode_condition_selector(
                reader
            )
        )
    )
    platform = reader.read_option(
        lambda: (
            destack._generated.repository.config.condition.decode_condition_selector(
                reader
            )
        )
    )
    host = reader.read_option(
        lambda: (
            destack._generated.repository.config.condition.decode_condition_selector(
                reader
            )
        )
    )
    runtime = reader.read_option(
        lambda: (
            destack._generated.repository.config.condition.decode_condition_selector(
                reader
            )
        )
    )

    return PolicySubject(
        package=package,
        runtime_identity=runtime_identity,
        worker=worker,
        mode=mode,
        role=role,
        feature=feature,
        tag=tag,
        target=target,
        product=product,
        platform=platform,
        host=host,
        runtime=runtime,
    )


def to_json_policy_subject(value: PolicySubject) -> Json:
    """Return one JSON value for one PolicySubject."""
    return {
        **(
            {}
            if value.package is None
            else {"package": to_json_package_selector(value.package)}
        ),
        **(
            {}
            if value.runtime_identity is None
            else {
                "runtimeIdentity": destack._generated.repository.config.runtime.selectors.to_json_runtime_identity_selector(
                    value.runtime_identity
                )
            }
        ),
        **(
            {}
            if value.worker is None
            else {
                "worker": destack._generated.repository.config.runtime.selectors.to_json_runtime_identity_selector(
                    value.worker
                )
            }
        ),
        **(
            {}
            if value.mode is None
            else {
                "mode": destack._generated.repository.config.condition.to_json_condition_selector(
                    value.mode
                )
            }
        ),
        **(
            {}
            if value.role is None
            else {
                "role": destack._generated.repository.config.condition.to_json_condition_selector(
                    value.role
                )
            }
        ),
        **(
            {}
            if value.feature is None
            else {
                "feature": destack._generated.repository.config.condition.to_json_condition_selector(
                    value.feature
                )
            }
        ),
        **(
            {}
            if value.tag is None
            else {
                "tag": destack._generated.repository.config.condition.to_json_condition_selector(
                    value.tag
                )
            }
        ),
        **(
            {}
            if value.target is None
            else {
                "target": destack._generated.repository.config.condition.to_json_condition_selector(
                    value.target
                )
            }
        ),
        **(
            {}
            if value.product is None
            else {
                "product": destack._generated.repository.config.condition.to_json_condition_selector(
                    value.product
                )
            }
        ),
        **(
            {}
            if value.platform is None
            else {
                "platform": destack._generated.repository.config.condition.to_json_condition_selector(
                    value.platform
                )
            }
        ),
        **(
            {}
            if value.host is None
            else {
                "host": destack._generated.repository.config.condition.to_json_condition_selector(
                    value.host
                )
            }
        ),
        **(
            {}
            if value.runtime is None
            else {
                "runtime": destack._generated.repository.config.condition.to_json_condition_selector(
                    value.runtime
                )
            }
        ),
    }


def from_json_policy_subject(value: Json) -> PolicySubject:
    """Return one PolicySubject from one JSON value."""
    object_ = json_object(value)

    return PolicySubject(
        package=json_optional(
            object_, "package", lambda value: from_json_package_selector(value)
        ),
        runtime_identity=json_optional(
            object_,
            "runtimeIdentity",
            lambda value: (
                destack._generated.repository.config.runtime.selectors.from_json_runtime_identity_selector(
                    value
                )
            ),
        ),
        worker=json_optional(
            object_,
            "worker",
            lambda value: (
                destack._generated.repository.config.runtime.selectors.from_json_runtime_identity_selector(
                    value
                )
            ),
        ),
        mode=json_optional(
            object_,
            "mode",
            lambda value: (
                destack._generated.repository.config.condition.from_json_condition_selector(
                    value
                )
            ),
        ),
        role=json_optional(
            object_,
            "role",
            lambda value: (
                destack._generated.repository.config.condition.from_json_condition_selector(
                    value
                )
            ),
        ),
        feature=json_optional(
            object_,
            "feature",
            lambda value: (
                destack._generated.repository.config.condition.from_json_condition_selector(
                    value
                )
            ),
        ),
        tag=json_optional(
            object_,
            "tag",
            lambda value: (
                destack._generated.repository.config.condition.from_json_condition_selector(
                    value
                )
            ),
        ),
        target=json_optional(
            object_,
            "target",
            lambda value: (
                destack._generated.repository.config.condition.from_json_condition_selector(
                    value
                )
            ),
        ),
        product=json_optional(
            object_,
            "product",
            lambda value: (
                destack._generated.repository.config.condition.from_json_condition_selector(
                    value
                )
            ),
        ),
        platform=json_optional(
            object_,
            "platform",
            lambda value: (
                destack._generated.repository.config.condition.from_json_condition_selector(
                    value
                )
            ),
        ),
        host=json_optional(
            object_,
            "host",
            lambda value: (
                destack._generated.repository.config.condition.from_json_condition_selector(
                    value
                )
            ),
        ),
        runtime=json_optional(
            object_,
            "runtime",
            lambda value: (
                destack._generated.repository.config.condition.from_json_condition_selector(
                    value
                )
            ),
        ),
    )


@dataclass(frozen=True, slots=True)
class PackageSelector:
    """Package selector used by static policy rules."""

    # package name glob patterns
    patterns: Sequence[str]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_package_selector(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PackageSelector:
        """Decode one PackageSelector."""
        return decode_package_selector(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_package_selector(self)

    @classmethod
    def from_json(cls, value: Json) -> PackageSelector:
        """Return one PackageSelector from one JSON value."""
        return from_json_package_selector(value)


def encode_package_selector(writer: BinaryWriter, value: PackageSelector) -> None:
    """Encode one PackageSelector."""
    writer.write_unsigned(len(value.patterns))
    for item_value_patterns_0 in value.patterns:
        writer.write_string(item_value_patterns_0)


def decode_package_selector(reader: BinaryReader) -> PackageSelector:
    """Decode one PackageSelector."""
    patterns = [reader.read_string() for _ in range(reader.read_number())]

    return PackageSelector(
        patterns=patterns,
    )


def to_json_package_selector(value: PackageSelector) -> Json:
    """Return one JSON value for one PackageSelector."""
    return {
        "patterns": [item_0 for item_0 in value.patterns],
    }


def from_json_package_selector(value: Json) -> PackageSelector:
    """Return one PackageSelector from one JSON value."""
    object_ = json_object(value)

    return PackageSelector(
        patterns=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "patterns"))
        ],
    )


"""Policy access result selected by one rule."""
PolicyAccess: typing.TypeAlias = typing.Literal["allow"] | typing.Literal["deny"]


def encode_policy_access(writer: BinaryWriter, value: PolicyAccess) -> None:
    """Encode one PolicyAccess."""
    if value == "allow":
        writer.write_unsigned(0)
    elif value == "deny":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_policy_access(reader: BinaryReader) -> PolicyAccess:
    """Decode one PolicyAccess."""
    variant = reader.read_number()

    if variant == 0:
        return "allow"
    elif variant == 1:
        return "deny"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_policy_access(value: PolicyAccess) -> Json:
    """Return one JSON value for one PolicyAccess."""
    return value


def from_json_policy_access(value: Json) -> PolicyAccess:
    """Return one PolicyAccess from one JSON value."""
    variant = json_string(value)

    if variant == "allow":
        return "allow"
    elif variant == "deny":
        return "deny"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


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
