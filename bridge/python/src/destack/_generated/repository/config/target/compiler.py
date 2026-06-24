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
    json_optional,
    json_string,
)

import destack._generated.repository.config.compiler


@dataclass(frozen=True, slots=True)
class TargetCompilerOptions:
    """Target compiler behavior options."""

    # target tree tag builder override
    tree: str | None
    # target well-known derives
    derive: Sequence[destack._generated.repository.config.compiler.Derive]
    # static semantic restrictions for this target
    restrictions: destack._generated.repository.config.compiler.CompilerRestrictions
    # optimization level
    optimize: OptimizeLevel
    # generated safety check policies
    checks: CheckPolicySet

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target_compiler_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TargetCompilerOptions:
        """Decode one TargetCompilerOptions."""
        return decode_target_compiler_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target_compiler_options(self)

    @classmethod
    def from_json(cls, value: Json) -> TargetCompilerOptions:
        """Return one TargetCompilerOptions from one JSON value."""
        return from_json_target_compiler_options(value)


def encode_target_compiler_options(
    writer: BinaryWriter, value: TargetCompilerOptions
) -> None:
    """Encode one TargetCompilerOptions."""
    if value.tree is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.tree)
    writer.write_unsigned(len(value.derive))
    for item_value_derive_0 in value.derive:
        destack._generated.repository.config.compiler.encode_derive(
            writer, item_value_derive_0
        )
    destack._generated.repository.config.compiler.encode_compiler_restrictions(
        writer, value.restrictions
    )
    encode_optimize_level(writer, value.optimize)
    encode_check_policy_set(writer, value.checks)


def decode_target_compiler_options(reader: BinaryReader) -> TargetCompilerOptions:
    """Decode one TargetCompilerOptions."""
    tree = reader.read_option(lambda: reader.read_string())
    derive = [
        destack._generated.repository.config.compiler.decode_derive(reader)
        for _ in range(reader.read_number())
    ]
    restrictions = (
        destack._generated.repository.config.compiler.decode_compiler_restrictions(
            reader
        )
    )
    optimize = decode_optimize_level(reader)
    checks = decode_check_policy_set(reader)

    return TargetCompilerOptions(
        tree=tree,
        derive=derive,
        restrictions=restrictions,
        optimize=optimize,
        checks=checks,
    )


def to_json_target_compiler_options(value: TargetCompilerOptions) -> Json:
    """Return one JSON value for one TargetCompilerOptions."""
    return {
        **({} if value.tree is None else {"tree": value.tree}),
        "derive": [
            destack._generated.repository.config.compiler.to_json_derive(item_0)
            for item_0 in value.derive
        ],
        "restrictions": destack._generated.repository.config.compiler.to_json_compiler_restrictions(
            value.restrictions
        ),
        "optimize": to_json_optimize_level(value.optimize),
        "checks": to_json_check_policy_set(value.checks),
    }


def from_json_target_compiler_options(value: Json) -> TargetCompilerOptions:
    """Return one TargetCompilerOptions from one JSON value."""
    object_ = json_object(value)

    return TargetCompilerOptions(
        tree=json_optional(object_, "tree", lambda value: json_string(value)),
        derive=[
            destack._generated.repository.config.compiler.from_json_derive(item_0)
            for item_0 in json_array(json_field(object_, "derive"))
        ],
        restrictions=destack._generated.repository.config.compiler.from_json_compiler_restrictions(
            json_field(object_, "restrictions")
        ),
        optimize=from_json_optimize_level(json_field(object_, "optimize")),
        checks=from_json_check_policy_set(json_field(object_, "checks")),
    )


"""Optimization level for builds."""
OptimizeLevel: typing.TypeAlias = (
    typing.Literal["o0"]
    | typing.Literal["o1"]
    | typing.Literal["o2"]
    | typing.Literal["o3"]
    | typing.Literal["o4"]
)


def encode_optimize_level(writer: BinaryWriter, value: OptimizeLevel) -> None:
    """Encode one OptimizeLevel."""
    if value == "o0":
        writer.write_unsigned(0)
    elif value == "o1":
        writer.write_unsigned(1)
    elif value == "o2":
        writer.write_unsigned(2)
    elif value == "o3":
        writer.write_unsigned(3)
    elif value == "o4":
        writer.write_unsigned(4)
    else:
        raise SerdeError("unknown enum variant")


def decode_optimize_level(reader: BinaryReader) -> OptimizeLevel:
    """Decode one OptimizeLevel."""
    variant = reader.read_number()

    if variant == 0:
        return "o0"
    elif variant == 1:
        return "o1"
    elif variant == 2:
        return "o2"
    elif variant == 3:
        return "o3"
    elif variant == 4:
        return "o4"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_optimize_level(value: OptimizeLevel) -> Json:
    """Return one JSON value for one OptimizeLevel."""
    return value


def from_json_optimize_level(value: Json) -> OptimizeLevel:
    """Return one OptimizeLevel from one JSON value."""
    variant = json_string(value)

    if variant == "o0":
        return "o0"
    elif variant == "o1":
        return "o1"
    elif variant == "o2":
        return "o2"
    elif variant == "o3":
        return "o3"
    elif variant == "o4":
        return "o4"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class CheckPolicySet:
    """Generated check policy set."""

    # integer overflow check policy
    overflow: CheckPolicy
    # bounds check policy for array and slice accesses
    bounds: CheckPolicy
    # null check policy for reference operations
    null: CheckPolicy
    # division check policy for divide and remainder operations
    division: CheckPolicy
    # shift range check policy
    shift: CheckPolicy
    # check failure behavior
    failure: CheckFailurePolicy

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_check_policy_set(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CheckPolicySet:
        """Decode one CheckPolicySet."""
        return decode_check_policy_set(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_check_policy_set(self)

    @classmethod
    def from_json(cls, value: Json) -> CheckPolicySet:
        """Return one CheckPolicySet from one JSON value."""
        return from_json_check_policy_set(value)


def encode_check_policy_set(writer: BinaryWriter, value: CheckPolicySet) -> None:
    """Encode one CheckPolicySet."""
    encode_check_policy(writer, value.overflow)
    encode_check_policy(writer, value.bounds)
    encode_check_policy(writer, value.null)
    encode_check_policy(writer, value.division)
    encode_check_policy(writer, value.shift)
    encode_check_failure_policy(writer, value.failure)


def decode_check_policy_set(reader: BinaryReader) -> CheckPolicySet:
    """Decode one CheckPolicySet."""
    overflow = decode_check_policy(reader)
    bounds = decode_check_policy(reader)
    null = decode_check_policy(reader)
    division = decode_check_policy(reader)
    shift = decode_check_policy(reader)
    failure = decode_check_failure_policy(reader)

    return CheckPolicySet(
        overflow=overflow,
        bounds=bounds,
        null=null,
        division=division,
        shift=shift,
        failure=failure,
    )


def to_json_check_policy_set(value: CheckPolicySet) -> Json:
    """Return one JSON value for one CheckPolicySet."""
    return {
        "overflow": to_json_check_policy(value.overflow),
        "bounds": to_json_check_policy(value.bounds),
        "null": to_json_check_policy(value.null),
        "division": to_json_check_policy(value.division),
        "shift": to_json_check_policy(value.shift),
        "failure": to_json_check_failure_policy(value.failure),
    }


def from_json_check_policy_set(value: Json) -> CheckPolicySet:
    """Return one CheckPolicySet from one JSON value."""
    object_ = json_object(value)

    return CheckPolicySet(
        overflow=from_json_check_policy(json_field(object_, "overflow")),
        bounds=from_json_check_policy(json_field(object_, "bounds")),
        null=from_json_check_policy(json_field(object_, "null")),
        division=from_json_check_policy(json_field(object_, "division")),
        shift=from_json_check_policy(json_field(object_, "shift")),
        failure=from_json_check_failure_policy(json_field(object_, "failure")),
    )


"""Generated safety check policy."""
CheckPolicy: typing.TypeAlias = (
    typing.Literal["always"] | typing.Literal["debug"] | typing.Literal["never"]
)


def encode_check_policy(writer: BinaryWriter, value: CheckPolicy) -> None:
    """Encode one CheckPolicy."""
    if value == "always":
        writer.write_unsigned(0)
    elif value == "debug":
        writer.write_unsigned(1)
    elif value == "never":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_check_policy(reader: BinaryReader) -> CheckPolicy:
    """Decode one CheckPolicy."""
    variant = reader.read_number()

    if variant == 0:
        return "always"
    elif variant == 1:
        return "debug"
    elif variant == 2:
        return "never"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_check_policy(value: CheckPolicy) -> Json:
    """Return one JSON value for one CheckPolicy."""
    return value


def from_json_check_policy(value: Json) -> CheckPolicy:
    """Return one CheckPolicy from one JSON value."""
    variant = json_string(value)

    if variant == "always":
        return "always"
    elif variant == "debug":
        return "debug"
    elif variant == "never":
        return "never"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Check failure behavior."""
CheckFailurePolicy: typing.TypeAlias = typing.Literal["panic"] | typing.Literal["abort"]


def encode_check_failure_policy(
    writer: BinaryWriter, value: CheckFailurePolicy
) -> None:
    """Encode one CheckFailurePolicy."""
    if value == "panic":
        writer.write_unsigned(0)
    elif value == "abort":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_check_failure_policy(reader: BinaryReader) -> CheckFailurePolicy:
    """Decode one CheckFailurePolicy."""
    variant = reader.read_number()

    if variant == 0:
        return "panic"
    elif variant == 1:
        return "abort"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_check_failure_policy(value: CheckFailurePolicy) -> Json:
    """Return one JSON value for one CheckFailurePolicy."""
    return value


def from_json_check_failure_policy(value: Json) -> CheckFailurePolicy:
    """Return one CheckFailurePolicy from one JSON value."""
    variant = json_string(value)

    if variant == "panic":
        return "panic"
    elif variant == "abort":
        return "abort"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "TargetCompilerOptions",
    "encode_target_compiler_options",
    "decode_target_compiler_options",
    "to_json_target_compiler_options",
    "from_json_target_compiler_options",
    "OptimizeLevel",
    "encode_optimize_level",
    "decode_optimize_level",
    "to_json_optimize_level",
    "from_json_optimize_level",
    "CheckPolicySet",
    "encode_check_policy_set",
    "decode_check_policy_set",
    "to_json_check_policy_set",
    "from_json_check_policy_set",
    "CheckPolicy",
    "encode_check_policy",
    "decode_check_policy",
    "to_json_check_policy",
    "from_json_check_policy",
    "CheckFailurePolicy",
    "encode_check_failure_policy",
    "decode_check_failure_policy",
    "to_json_check_failure_policy",
    "from_json_check_failure_policy",
]
