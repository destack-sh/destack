# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_array,
    json_bool,
    json_field,
    json_object,
    json_string,
    nested_bytes,
)

import destack._generated.repository.config.linter.complexity
import destack._generated.repository.config.linter.correctness
import destack._generated.repository.config.linter.performance
import destack._generated.repository.config.linter.restriction
import destack._generated.repository.config.linter.security
import destack._generated.repository.config.linter.style
import destack._generated.repository.config.linter.suspicious


@dataclass(frozen=True, slots=True)
class LinterOptions:
    """Linter options."""

    # whether linting is enabled
    enabled: bool
    # base preset (none, recommended, all)
    preset: LintPreset
    # category-level severity overrides
    categories: Mapping[LintCategory, LintSeverity]
    # individual rule severity overrides
    overrides: Mapping[str, LintSeverity]
    # include declaration files when evaluating declaration-gated rules
    include_declaration_files: bool
    # correctness-category options
    correctness: (
        destack._generated.repository.config.linter.correctness.LinterCorrectnessOptions
    )
    # suspicious-category options
    suspicious: (
        destack._generated.repository.config.linter.suspicious.LinterSuspiciousOptions
    )
    # performance-category options
    performance: (
        destack._generated.repository.config.linter.performance.LinterPerformanceOptions
    )
    # style-category options
    style: destack._generated.repository.config.linter.style.LinterStyleOptions
    # security-category options
    security: destack._generated.repository.config.linter.security.LinterSecurityOptions
    # complexity-category options
    complexity: (
        destack._generated.repository.config.linter.complexity.LinterComplexityOptions
    )
    # restriction-category options
    restriction: (
        destack._generated.repository.config.linter.restriction.LinterRestrictionOptions
    )

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_linter_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> LinterOptions:
        """Decode one LinterOptions."""
        return decode_linter_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_linter_options(self)

    @classmethod
    def from_json(cls, value: Json) -> LinterOptions:
        """Return one LinterOptions from one JSON value."""
        return from_json_linter_options(value)


def encode_linter_options(writer: BinaryWriter, value: LinterOptions) -> None:
    """Encode one LinterOptions."""
    writer.write_bool(value.enabled)
    encode_lint_preset(writer, value.preset)
    entries_value_categories_0 = []
    for key_value_categories_0, item_value_categories_0 in value.categories.items():

        def write_key_value_categories_0(writer: BinaryWriter) -> None:
            encode_lint_category(writer, key_value_categories_0)

        key_bytes = nested_bytes(write_key_value_categories_0)
        entries_value_categories_0.append(
            (key_value_categories_0, item_value_categories_0, key_bytes)
        )
    entries_value_categories_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_categories_0))
    for entry_value_categories_0 in entries_value_categories_0:
        encode_lint_category(writer, entry_value_categories_0[0])
        encode_lint_severity(writer, entry_value_categories_0[1])
    entries_value_overrides_0 = []
    for key_value_overrides_0, item_value_overrides_0 in value.overrides.items():

        def write_key_value_overrides_0(writer: BinaryWriter) -> None:
            writer.write_string(key_value_overrides_0)

        key_bytes = nested_bytes(write_key_value_overrides_0)
        entries_value_overrides_0.append(
            (key_value_overrides_0, item_value_overrides_0, key_bytes)
        )
    entries_value_overrides_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_overrides_0))
    for entry_value_overrides_0 in entries_value_overrides_0:
        writer.write_string(entry_value_overrides_0[0])
        encode_lint_severity(writer, entry_value_overrides_0[1])
    writer.write_bool(value.include_declaration_files)
    destack._generated.repository.config.linter.correctness.encode_linter_correctness_options(
        writer, value.correctness
    )
    destack._generated.repository.config.linter.suspicious.encode_linter_suspicious_options(
        writer, value.suspicious
    )
    destack._generated.repository.config.linter.performance.encode_linter_performance_options(
        writer, value.performance
    )
    destack._generated.repository.config.linter.style.encode_linter_style_options(
        writer, value.style
    )
    destack._generated.repository.config.linter.security.encode_linter_security_options(
        writer, value.security
    )
    destack._generated.repository.config.linter.complexity.encode_linter_complexity_options(
        writer, value.complexity
    )
    destack._generated.repository.config.linter.restriction.encode_linter_restriction_options(
        writer, value.restriction
    )


def decode_linter_options(reader: BinaryReader) -> LinterOptions:
    """Decode one LinterOptions."""
    enabled = reader.read_bool()
    preset = decode_lint_preset(reader)
    categories = {
        decode_lint_category(reader): decode_lint_severity(reader)
        for _ in range(reader.read_number())
    }
    overrides = {
        reader.read_string(): decode_lint_severity(reader)
        for _ in range(reader.read_number())
    }
    include_declaration_files = reader.read_bool()
    correctness = destack._generated.repository.config.linter.correctness.decode_linter_correctness_options(
        reader
    )
    suspicious = destack._generated.repository.config.linter.suspicious.decode_linter_suspicious_options(
        reader
    )
    performance = destack._generated.repository.config.linter.performance.decode_linter_performance_options(
        reader
    )
    style = (
        destack._generated.repository.config.linter.style.decode_linter_style_options(
            reader
        )
    )
    security = destack._generated.repository.config.linter.security.decode_linter_security_options(
        reader
    )
    complexity = destack._generated.repository.config.linter.complexity.decode_linter_complexity_options(
        reader
    )
    restriction = destack._generated.repository.config.linter.restriction.decode_linter_restriction_options(
        reader
    )

    return LinterOptions(
        enabled=enabled,
        preset=preset,
        categories=categories,
        overrides=overrides,
        include_declaration_files=include_declaration_files,
        correctness=correctness,
        suspicious=suspicious,
        performance=performance,
        style=style,
        security=security,
        complexity=complexity,
        restriction=restriction,
    )


def to_json_linter_options(value: LinterOptions) -> Json:
    """Return one JSON value for one LinterOptions."""
    return {
        "enabled": value.enabled,
        "preset": to_json_lint_preset(value.preset),
        "categories": [
            [to_json_lint_category(key_0), to_json_lint_severity(item_0)]
            for key_0, item_0 in value.categories.items()
        ],
        "overrides": {
            key_0: to_json_lint_severity(item_0)
            for key_0, item_0 in value.overrides.items()
        },
        "includeDeclarationFiles": value.include_declaration_files,
        "correctness": destack._generated.repository.config.linter.correctness.to_json_linter_correctness_options(
            value.correctness
        ),
        "suspicious": destack._generated.repository.config.linter.suspicious.to_json_linter_suspicious_options(
            value.suspicious
        ),
        "performance": destack._generated.repository.config.linter.performance.to_json_linter_performance_options(
            value.performance
        ),
        "style": destack._generated.repository.config.linter.style.to_json_linter_style_options(
            value.style
        ),
        "security": destack._generated.repository.config.linter.security.to_json_linter_security_options(
            value.security
        ),
        "complexity": destack._generated.repository.config.linter.complexity.to_json_linter_complexity_options(
            value.complexity
        ),
        "restriction": destack._generated.repository.config.linter.restriction.to_json_linter_restriction_options(
            value.restriction
        ),
    }


def from_json_linter_options(value: Json) -> LinterOptions:
    """Return one LinterOptions from one JSON value."""
    object_ = json_object(value)

    return LinterOptions(
        enabled=json_bool(json_field(object_, "enabled")),
        preset=from_json_lint_preset(json_field(object_, "preset")),
        categories={
            from_json_lint_category(key_0): from_json_lint_severity(item_0)
            for key_0, item_0 in json_array(json_field(object_, "categories"))
        },
        overrides={
            key_0: from_json_lint_severity(item_0)
            for key_0, item_0 in json_object(json_field(object_, "overrides")).items()
        },
        include_declaration_files=json_bool(
            json_field(object_, "includeDeclarationFiles")
        ),
        correctness=destack._generated.repository.config.linter.correctness.from_json_linter_correctness_options(
            json_field(object_, "correctness")
        ),
        suspicious=destack._generated.repository.config.linter.suspicious.from_json_linter_suspicious_options(
            json_field(object_, "suspicious")
        ),
        performance=destack._generated.repository.config.linter.performance.from_json_linter_performance_options(
            json_field(object_, "performance")
        ),
        style=destack._generated.repository.config.linter.style.from_json_linter_style_options(
            json_field(object_, "style")
        ),
        security=destack._generated.repository.config.linter.security.from_json_linter_security_options(
            json_field(object_, "security")
        ),
        complexity=destack._generated.repository.config.linter.complexity.from_json_linter_complexity_options(
            json_field(object_, "complexity")
        ),
        restriction=destack._generated.repository.config.linter.restriction.from_json_linter_restriction_options(
            json_field(object_, "restriction")
        ),
    )


"""Lint rule preset."""
LintPreset: typing.TypeAlias = (
    typing.Literal["none"]
    | typing.Literal["recommended"]
    | typing.Literal["strict"]
    | typing.Literal["all"]
)


def encode_lint_preset(writer: BinaryWriter, value: LintPreset) -> None:
    """Encode one LintPreset."""
    if value == "none":
        writer.write_unsigned(0)
    elif value == "recommended":
        writer.write_unsigned(1)
    elif value == "strict":
        writer.write_unsigned(2)
    elif value == "all":
        writer.write_unsigned(3)
    else:
        raise SerdeError("unknown enum variant")


def decode_lint_preset(reader: BinaryReader) -> LintPreset:
    """Decode one LintPreset."""
    variant = reader.read_number()

    if variant == 0:
        return "none"
    elif variant == 1:
        return "recommended"
    elif variant == 2:
        return "strict"
    elif variant == 3:
        return "all"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_lint_preset(value: LintPreset) -> Json:
    """Return one JSON value for one LintPreset."""
    return value


def from_json_lint_preset(value: Json) -> LintPreset:
    """Return one LintPreset from one JSON value."""
    variant = json_string(value)

    if variant == "none":
        return "none"
    elif variant == "recommended":
        return "recommended"
    elif variant == "strict":
        return "strict"
    elif variant == "all":
        return "all"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Lint rule categories."""
LintCategory: typing.TypeAlias = (
    typing.Literal["correctness"]
    | typing.Literal["suspicious"]
    | typing.Literal["performance"]
    | typing.Literal["style"]
    | typing.Literal["security"]
    | typing.Literal["complexity"]
    | typing.Literal["restriction"]
)


def encode_lint_category(writer: BinaryWriter, value: LintCategory) -> None:
    """Encode one LintCategory."""
    if value == "correctness":
        writer.write_unsigned(0)
    elif value == "suspicious":
        writer.write_unsigned(1)
    elif value == "performance":
        writer.write_unsigned(2)
    elif value == "style":
        writer.write_unsigned(3)
    elif value == "security":
        writer.write_unsigned(4)
    elif value == "complexity":
        writer.write_unsigned(5)
    elif value == "restriction":
        writer.write_unsigned(6)
    else:
        raise SerdeError("unknown enum variant")


def decode_lint_category(reader: BinaryReader) -> LintCategory:
    """Decode one LintCategory."""
    variant = reader.read_number()

    if variant == 0:
        return "correctness"
    elif variant == 1:
        return "suspicious"
    elif variant == 2:
        return "performance"
    elif variant == 3:
        return "style"
    elif variant == 4:
        return "security"
    elif variant == 5:
        return "complexity"
    elif variant == 6:
        return "restriction"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_lint_category(value: LintCategory) -> Json:
    """Return one JSON value for one LintCategory."""
    return value


def from_json_lint_category(value: Json) -> LintCategory:
    """Return one LintCategory from one JSON value."""
    variant = json_string(value)

    if variant == "correctness":
        return "correctness"
    elif variant == "suspicious":
        return "suspicious"
    elif variant == "performance":
        return "performance"
    elif variant == "style":
        return "style"
    elif variant == "security":
        return "security"
    elif variant == "complexity":
        return "complexity"
    elif variant == "restriction":
        return "restriction"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Rule severity level."""
LintSeverity: typing.TypeAlias = (
    typing.Literal["off"]
    | typing.Literal["note"]
    | typing.Literal["warning"]
    | typing.Literal["error"]
)


def encode_lint_severity(writer: BinaryWriter, value: LintSeverity) -> None:
    """Encode one LintSeverity."""
    if value == "off":
        writer.write_unsigned(0)
    elif value == "note":
        writer.write_unsigned(1)
    elif value == "warning":
        writer.write_unsigned(2)
    elif value == "error":
        writer.write_unsigned(3)
    else:
        raise SerdeError("unknown enum variant")


def decode_lint_severity(reader: BinaryReader) -> LintSeverity:
    """Decode one LintSeverity."""
    variant = reader.read_number()

    if variant == 0:
        return "off"
    elif variant == 1:
        return "note"
    elif variant == 2:
        return "warning"
    elif variant == 3:
        return "error"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_lint_severity(value: LintSeverity) -> Json:
    """Return one JSON value for one LintSeverity."""
    return value


def from_json_lint_severity(value: Json) -> LintSeverity:
    """Return one LintSeverity from one JSON value."""
    variant = json_string(value)

    if variant == "off":
        return "off"
    elif variant == "note":
        return "note"
    elif variant == "warning":
        return "warning"
    elif variant == "error":
        return "error"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Condition assignment policy for `no-cond-assign`."""
ConditionAssignmentMode: typing.TypeAlias = (
    typing.Literal["exceptParens"] | typing.Literal["always"]
)


def encode_condition_assignment_mode(
    writer: BinaryWriter, value: ConditionAssignmentMode
) -> None:
    """Encode one ConditionAssignmentMode."""
    if value == "exceptParens":
        writer.write_unsigned(0)
    elif value == "always":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_condition_assignment_mode(reader: BinaryReader) -> ConditionAssignmentMode:
    """Decode one ConditionAssignmentMode."""
    variant = reader.read_number()

    if variant == 0:
        return "exceptParens"
    elif variant == 1:
        return "always"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_condition_assignment_mode(value: ConditionAssignmentMode) -> Json:
    """Return one JSON value for one ConditionAssignmentMode."""
    return value


def from_json_condition_assignment_mode(value: Json) -> ConditionAssignmentMode:
    """Return one ConditionAssignmentMode from one JSON value."""
    variant = json_string(value)

    if variant == "exceptParens":
        return "exceptParens"
    elif variant == "always":
        return "always"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Empty function kinds that `no-empty-function` may allow."""
EmptyFunctionKind: typing.TypeAlias = (
    typing.Literal["functions"]
    | typing.Literal["arrowFunctions"]
    | typing.Literal["generatorFunctions"]
    | typing.Literal["methods"]
    | typing.Literal["generatorMethods"]
    | typing.Literal["getters"]
    | typing.Literal["setters"]
    | typing.Literal["constructors"]
    | typing.Literal["asyncFunctions"]
    | typing.Literal["asyncMethods"]
    | typing.Literal["overrideMethods"]
)


def encode_empty_function_kind(writer: BinaryWriter, value: EmptyFunctionKind) -> None:
    """Encode one EmptyFunctionKind."""
    if value == "functions":
        writer.write_unsigned(0)
    elif value == "arrowFunctions":
        writer.write_unsigned(1)
    elif value == "generatorFunctions":
        writer.write_unsigned(2)
    elif value == "methods":
        writer.write_unsigned(3)
    elif value == "generatorMethods":
        writer.write_unsigned(4)
    elif value == "getters":
        writer.write_unsigned(5)
    elif value == "setters":
        writer.write_unsigned(6)
    elif value == "constructors":
        writer.write_unsigned(7)
    elif value == "asyncFunctions":
        writer.write_unsigned(8)
    elif value == "asyncMethods":
        writer.write_unsigned(9)
    elif value == "overrideMethods":
        writer.write_unsigned(10)
    else:
        raise SerdeError("unknown enum variant")


def decode_empty_function_kind(reader: BinaryReader) -> EmptyFunctionKind:
    """Decode one EmptyFunctionKind."""
    variant = reader.read_number()

    if variant == 0:
        return "functions"
    elif variant == 1:
        return "arrowFunctions"
    elif variant == 2:
        return "generatorFunctions"
    elif variant == 3:
        return "methods"
    elif variant == 4:
        return "generatorMethods"
    elif variant == 5:
        return "getters"
    elif variant == 6:
        return "setters"
    elif variant == 7:
        return "constructors"
    elif variant == 8:
        return "asyncFunctions"
    elif variant == 9:
        return "asyncMethods"
    elif variant == 10:
        return "overrideMethods"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_empty_function_kind(value: EmptyFunctionKind) -> Json:
    """Return one JSON value for one EmptyFunctionKind."""
    return value


def from_json_empty_function_kind(value: Json) -> EmptyFunctionKind:
    """Return one EmptyFunctionKind from one JSON value."""
    variant = json_string(value)

    if variant == "functions":
        return "functions"
    elif variant == "arrowFunctions":
        return "arrowFunctions"
    elif variant == "generatorFunctions":
        return "generatorFunctions"
    elif variant == "methods":
        return "methods"
    elif variant == "generatorMethods":
        return "generatorMethods"
    elif variant == "getters":
        return "getters"
    elif variant == "setters":
        return "setters"
    elif variant == "constructors":
        return "constructors"
    elif variant == "asyncFunctions":
        return "asyncFunctions"
    elif variant == "asyncMethods":
        return "asyncMethods"
    elif variant == "overrideMethods":
        return "overrideMethods"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Return-await mode for the `return-await` rule."""
ReturnAwaitMode: typing.TypeAlias = (
    typing.Literal["inTryCatch"]
    | typing.Literal["errorHandlingCorrectnessOnly"]
    | typing.Literal["always"]
    | typing.Literal["never"]
)


def encode_return_await_mode(writer: BinaryWriter, value: ReturnAwaitMode) -> None:
    """Encode one ReturnAwaitMode."""
    if value == "inTryCatch":
        writer.write_unsigned(0)
    elif value == "errorHandlingCorrectnessOnly":
        writer.write_unsigned(1)
    elif value == "always":
        writer.write_unsigned(2)
    elif value == "never":
        writer.write_unsigned(3)
    else:
        raise SerdeError("unknown enum variant")


def decode_return_await_mode(reader: BinaryReader) -> ReturnAwaitMode:
    """Decode one ReturnAwaitMode."""
    variant = reader.read_number()

    if variant == 0:
        return "inTryCatch"
    elif variant == 1:
        return "errorHandlingCorrectnessOnly"
    elif variant == 2:
        return "always"
    elif variant == 3:
        return "never"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_return_await_mode(value: ReturnAwaitMode) -> Json:
    """Return one JSON value for one ReturnAwaitMode."""
    return value


def from_json_return_await_mode(value: Json) -> ReturnAwaitMode:
    """Return one ReturnAwaitMode from one JSON value."""
    variant = json_string(value)

    if variant == "inTryCatch":
        return "inTryCatch"
    elif variant == "errorHandlingCorrectnessOnly":
        return "errorHandlingCorrectnessOnly"
    elif variant == "always":
        return "always"
    elif variant == "never":
        return "never"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Required Unicode regex flag for `require-unicode-regexp`."""
UnicodeRegexpRequireFlag: typing.TypeAlias = typing.Literal["u"] | typing.Literal["v"]


def encode_unicode_regexp_require_flag(
    writer: BinaryWriter, value: UnicodeRegexpRequireFlag
) -> None:
    """Encode one UnicodeRegexpRequireFlag."""
    if value == "u":
        writer.write_unsigned(0)
    elif value == "v":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_unicode_regexp_require_flag(
    reader: BinaryReader,
) -> UnicodeRegexpRequireFlag:
    """Decode one UnicodeRegexpRequireFlag."""
    variant = reader.read_number()

    if variant == 0:
        return "u"
    elif variant == 1:
        return "v"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_unicode_regexp_require_flag(value: UnicodeRegexpRequireFlag) -> Json:
    """Return one JSON value for one UnicodeRegexpRequireFlag."""
    return value


def from_json_unicode_regexp_require_flag(value: Json) -> UnicodeRegexpRequireFlag:
    """Return one UnicodeRegexpRequireFlag from one JSON value."""
    variant = json_string(value)

    if variant == "u":
        return "u"
    elif variant == "v":
        return "v"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Preferred array type syntax for the `array-type` rule."""
ArrayTypeStyle: typing.TypeAlias = typing.Literal["array"] | typing.Literal["generic"]


def encode_array_type_style(writer: BinaryWriter, value: ArrayTypeStyle) -> None:
    """Encode one ArrayTypeStyle."""
    if value == "array":
        writer.write_unsigned(0)
    elif value == "generic":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_array_type_style(reader: BinaryReader) -> ArrayTypeStyle:
    """Decode one ArrayTypeStyle."""
    variant = reader.read_number()

    if variant == 0:
        return "array"
    elif variant == 1:
        return "generic"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_array_type_style(value: ArrayTypeStyle) -> Json:
    """Return one JSON value for one ArrayTypeStyle."""
    return value


def from_json_array_type_style(value: Json) -> ArrayTypeStyle:
    """Return one ArrayTypeStyle from one JSON value."""
    variant = json_string(value)

    if variant == "array":
        return "array"
    elif variant == "generic":
        return "generic"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Filename case style for the `filename-case` rule."""
FilenameCase: typing.TypeAlias = (
    typing.Literal["kebab"]
    | typing.Literal["snake"]
    | typing.Literal["camel"]
    | typing.Literal["pascal"]
)


def encode_filename_case(writer: BinaryWriter, value: FilenameCase) -> None:
    """Encode one FilenameCase."""
    if value == "kebab":
        writer.write_unsigned(0)
    elif value == "snake":
        writer.write_unsigned(1)
    elif value == "camel":
        writer.write_unsigned(2)
    elif value == "pascal":
        writer.write_unsigned(3)
    else:
        raise SerdeError("unknown enum variant")


def decode_filename_case(reader: BinaryReader) -> FilenameCase:
    """Decode one FilenameCase."""
    variant = reader.read_number()

    if variant == 0:
        return "kebab"
    elif variant == 1:
        return "snake"
    elif variant == 2:
        return "camel"
    elif variant == 3:
        return "pascal"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_filename_case(value: FilenameCase) -> Json:
    """Return one JSON value for one FilenameCase."""
    return value


def from_json_filename_case(value: Json) -> FilenameCase:
    """Return one FilenameCase from one JSON value."""
    variant = json_string(value)

    if variant == "kebab":
        return "kebab"
    elif variant == "snake":
        return "snake"
    elif variant == "camel":
        return "camel"
    elif variant == "pascal":
        return "pascal"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Ordering policy for `grouped-accessor-pairs`."""
GroupedAccessorPairsOrder: typing.TypeAlias = (
    typing.Literal["anyOrder"]
    | typing.Literal["getBeforeSet"]
    | typing.Literal["setBeforeGet"]
)


def encode_grouped_accessor_pairs_order(
    writer: BinaryWriter, value: GroupedAccessorPairsOrder
) -> None:
    """Encode one GroupedAccessorPairsOrder."""
    if value == "anyOrder":
        writer.write_unsigned(0)
    elif value == "getBeforeSet":
        writer.write_unsigned(1)
    elif value == "setBeforeGet":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_grouped_accessor_pairs_order(
    reader: BinaryReader,
) -> GroupedAccessorPairsOrder:
    """Decode one GroupedAccessorPairsOrder."""
    variant = reader.read_number()

    if variant == 0:
        return "anyOrder"
    elif variant == 1:
        return "getBeforeSet"
    elif variant == 2:
        return "setBeforeGet"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_grouped_accessor_pairs_order(value: GroupedAccessorPairsOrder) -> Json:
    """Return one JSON value for one GroupedAccessorPairsOrder."""
    return value


def from_json_grouped_accessor_pairs_order(value: Json) -> GroupedAccessorPairsOrder:
    """Return one GroupedAccessorPairsOrder from one JSON value."""
    variant = json_string(value)

    if variant == "anyOrder":
        return "anyOrder"
    elif variant == "getBeforeSet":
        return "getBeforeSet"
    elif variant == "setBeforeGet":
        return "setBeforeGet"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Enforcement mode for `operator-assignment`."""
OperatorAssignmentMode: typing.TypeAlias = (
    typing.Literal["always"] | typing.Literal["never"]
)


def encode_operator_assignment_mode(
    writer: BinaryWriter, value: OperatorAssignmentMode
) -> None:
    """Encode one OperatorAssignmentMode."""
    if value == "always":
        writer.write_unsigned(0)
    elif value == "never":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_operator_assignment_mode(reader: BinaryReader) -> OperatorAssignmentMode:
    """Decode one OperatorAssignmentMode."""
    variant = reader.read_number()

    if variant == 0:
        return "always"
    elif variant == 1:
        return "never"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_operator_assignment_mode(value: OperatorAssignmentMode) -> Json:
    """Return one JSON value for one OperatorAssignmentMode."""
    return value


def from_json_operator_assignment_mode(value: Json) -> OperatorAssignmentMode:
    """Return one OperatorAssignmentMode from one JSON value."""
    variant = json_string(value)

    if variant == "always":
        return "always"
    elif variant == "never":
        return "never"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Enforcement mode for `object-shorthand`."""
ObjectShorthandMode: typing.TypeAlias = (
    typing.Literal["always"]
    | typing.Literal["methods"]
    | typing.Literal["properties"]
    | typing.Literal["never"]
    | typing.Literal["consistent"]
    | typing.Literal["consistentAsNeeded"]
)


def encode_object_shorthand_mode(
    writer: BinaryWriter, value: ObjectShorthandMode
) -> None:
    """Encode one ObjectShorthandMode."""
    if value == "always":
        writer.write_unsigned(0)
    elif value == "methods":
        writer.write_unsigned(1)
    elif value == "properties":
        writer.write_unsigned(2)
    elif value == "never":
        writer.write_unsigned(3)
    elif value == "consistent":
        writer.write_unsigned(4)
    elif value == "consistentAsNeeded":
        writer.write_unsigned(5)
    else:
        raise SerdeError("unknown enum variant")


def decode_object_shorthand_mode(reader: BinaryReader) -> ObjectShorthandMode:
    """Decode one ObjectShorthandMode."""
    variant = reader.read_number()

    if variant == 0:
        return "always"
    elif variant == 1:
        return "methods"
    elif variant == 2:
        return "properties"
    elif variant == 3:
        return "never"
    elif variant == 4:
        return "consistent"
    elif variant == 5:
        return "consistentAsNeeded"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_object_shorthand_mode(value: ObjectShorthandMode) -> Json:
    """Return one JSON value for one ObjectShorthandMode."""
    return value


def from_json_object_shorthand_mode(value: Json) -> ObjectShorthandMode:
    """Return one ObjectShorthandMode from one JSON value."""
    variant = json_string(value)

    if variant == "always":
        return "always"
    elif variant == "methods":
        return "methods"
    elif variant == "properties":
        return "properties"
    elif variant == "never":
        return "never"
    elif variant == "consistent":
        return "consistent"
    elif variant == "consistentAsNeeded":
        return "consistentAsNeeded"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Destructuring policy for `prefer-const`."""
PreferConstDestructuring: typing.TypeAlias = (
    typing.Literal["any"] | typing.Literal["all"]
)


def encode_prefer_const_destructuring(
    writer: BinaryWriter, value: PreferConstDestructuring
) -> None:
    """Encode one PreferConstDestructuring."""
    if value == "any":
        writer.write_unsigned(0)
    elif value == "all":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_prefer_const_destructuring(reader: BinaryReader) -> PreferConstDestructuring:
    """Decode one PreferConstDestructuring."""
    variant = reader.read_number()

    if variant == 0:
        return "any"
    elif variant == 1:
        return "all"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_prefer_const_destructuring(value: PreferConstDestructuring) -> Json:
    """Return one JSON value for one PreferConstDestructuring."""
    return value


def from_json_prefer_const_destructuring(value: Json) -> PreferConstDestructuring:
    """Return one PreferConstDestructuring from one JSON value."""
    variant = json_string(value)

    if variant == "any":
        return "any"
    elif variant == "all":
        return "all"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Enforcement mode for `yoda`."""
YodaMode: typing.TypeAlias = typing.Literal["always"] | typing.Literal["never"]


def encode_yoda_mode(writer: BinaryWriter, value: YodaMode) -> None:
    """Encode one YodaMode."""
    if value == "always":
        writer.write_unsigned(0)
    elif value == "never":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_yoda_mode(reader: BinaryReader) -> YodaMode:
    """Decode one YodaMode."""
    variant = reader.read_number()

    if variant == 0:
        return "always"
    elif variant == 1:
        return "never"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_yoda_mode(value: YodaMode) -> Json:
    """Return one JSON value for one YodaMode."""
    return value


def from_json_yoda_mode(value: Json) -> YodaMode:
    """Return one YodaMode from one JSON value."""
    variant = json_string(value)

    if variant == "always":
        return "always"
    elif variant == "never":
        return "never"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Member syntax groups for `sort-imports`."""
SortImportsMemberSyntax: typing.TypeAlias = (
    typing.Literal["none"]
    | typing.Literal["all"]
    | typing.Literal["multiple"]
    | typing.Literal["single"]
)


def encode_sort_imports_member_syntax(
    writer: BinaryWriter, value: SortImportsMemberSyntax
) -> None:
    """Encode one SortImportsMemberSyntax."""
    if value == "none":
        writer.write_unsigned(0)
    elif value == "all":
        writer.write_unsigned(1)
    elif value == "multiple":
        writer.write_unsigned(2)
    elif value == "single":
        writer.write_unsigned(3)
    else:
        raise SerdeError("unknown enum variant")


def decode_sort_imports_member_syntax(reader: BinaryReader) -> SortImportsMemberSyntax:
    """Decode one SortImportsMemberSyntax."""
    variant = reader.read_number()

    if variant == 0:
        return "none"
    elif variant == 1:
        return "all"
    elif variant == 2:
        return "multiple"
    elif variant == 3:
        return "single"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_sort_imports_member_syntax(value: SortImportsMemberSyntax) -> Json:
    """Return one JSON value for one SortImportsMemberSyntax."""
    return value


def from_json_sort_imports_member_syntax(value: Json) -> SortImportsMemberSyntax:
    """Return one SortImportsMemberSyntax from one JSON value."""
    variant = json_string(value)

    if variant == "none":
        return "none"
    elif variant == "all":
        return "all"
    elif variant == "multiple":
        return "multiple"
    elif variant == "single":
        return "single"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Switch counting variant for `cyclomatic-complexity`."""
CyclomaticComplexityVariant: typing.TypeAlias = (
    typing.Literal["classic"] | typing.Literal["modified"]
)


def encode_cyclomatic_complexity_variant(
    writer: BinaryWriter, value: CyclomaticComplexityVariant
) -> None:
    """Encode one CyclomaticComplexityVariant."""
    if value == "classic":
        writer.write_unsigned(0)
    elif value == "modified":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_cyclomatic_complexity_variant(
    reader: BinaryReader,
) -> CyclomaticComplexityVariant:
    """Decode one CyclomaticComplexityVariant."""
    variant = reader.read_number()

    if variant == 0:
        return "classic"
    elif variant == 1:
        return "modified"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_cyclomatic_complexity_variant(value: CyclomaticComplexityVariant) -> Json:
    """Return one JSON value for one CyclomaticComplexityVariant."""
    return value


def from_json_cyclomatic_complexity_variant(value: Json) -> CyclomaticComplexityVariant:
    """Return one CyclomaticComplexityVariant from one JSON value."""
    variant = json_string(value)

    if variant == "classic":
        return "classic"
    elif variant == "modified":
        return "modified"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""`this` parameter counting policy for `max-params`."""
MaxParamsCountThis: typing.TypeAlias = (
    typing.Literal["never"] | typing.Literal["exceptVoid"] | typing.Literal["always"]
)


def encode_max_params_count_this(
    writer: BinaryWriter, value: MaxParamsCountThis
) -> None:
    """Encode one MaxParamsCountThis."""
    if value == "never":
        writer.write_unsigned(0)
    elif value == "exceptVoid":
        writer.write_unsigned(1)
    elif value == "always":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_max_params_count_this(reader: BinaryReader) -> MaxParamsCountThis:
    """Decode one MaxParamsCountThis."""
    variant = reader.read_number()

    if variant == 0:
        return "never"
    elif variant == 1:
        return "exceptVoid"
    elif variant == 2:
        return "always"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_max_params_count_this(value: MaxParamsCountThis) -> Json:
    """Return one JSON value for one MaxParamsCountThis."""
    return value


def from_json_max_params_count_this(value: Json) -> MaxParamsCountThis:
    """Return one MaxParamsCountThis from one JSON value."""
    variant = json_string(value)

    if variant == "never":
        return "never"
    elif variant == "exceptVoid":
        return "exceptVoid"
    elif variant == "always":
        return "always"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Bitwise operators configurable for the `no-bitwise` rule."""
BitwiseOperator: typing.TypeAlias = (
    typing.Literal["and"]
    | typing.Literal["xor"]
    | typing.Literal["or"]
    | typing.Literal["not"]
    | typing.Literal["shiftLeft"]
    | typing.Literal["shiftRight"]
    | typing.Literal["unsignedShiftRight"]
    | typing.Literal["andAssign"]
    | typing.Literal["xorAssign"]
    | typing.Literal["orAssign"]
    | typing.Literal["shiftLeftAssign"]
    | typing.Literal["shiftRightAssign"]
    | typing.Literal["unsignedShiftRightAssign"]
)


def encode_bitwise_operator(writer: BinaryWriter, value: BitwiseOperator) -> None:
    """Encode one BitwiseOperator."""
    if value == "and":
        writer.write_unsigned(0)
    elif value == "xor":
        writer.write_unsigned(1)
    elif value == "or":
        writer.write_unsigned(2)
    elif value == "not":
        writer.write_unsigned(3)
    elif value == "shiftLeft":
        writer.write_unsigned(4)
    elif value == "shiftRight":
        writer.write_unsigned(5)
    elif value == "unsignedShiftRight":
        writer.write_unsigned(6)
    elif value == "andAssign":
        writer.write_unsigned(7)
    elif value == "xorAssign":
        writer.write_unsigned(8)
    elif value == "orAssign":
        writer.write_unsigned(9)
    elif value == "shiftLeftAssign":
        writer.write_unsigned(10)
    elif value == "shiftRightAssign":
        writer.write_unsigned(11)
    elif value == "unsignedShiftRightAssign":
        writer.write_unsigned(12)
    else:
        raise SerdeError("unknown enum variant")


def decode_bitwise_operator(reader: BinaryReader) -> BitwiseOperator:
    """Decode one BitwiseOperator."""
    variant = reader.read_number()

    if variant == 0:
        return "and"
    elif variant == 1:
        return "xor"
    elif variant == 2:
        return "or"
    elif variant == 3:
        return "not"
    elif variant == 4:
        return "shiftLeft"
    elif variant == 5:
        return "shiftRight"
    elif variant == 6:
        return "unsignedShiftRight"
    elif variant == 7:
        return "andAssign"
    elif variant == 8:
        return "xorAssign"
    elif variant == 9:
        return "orAssign"
    elif variant == 10:
        return "shiftLeftAssign"
    elif variant == 11:
        return "shiftRightAssign"
    elif variant == 12:
        return "unsignedShiftRightAssign"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_bitwise_operator(value: BitwiseOperator) -> Json:
    """Return one JSON value for one BitwiseOperator."""
    return value


def from_json_bitwise_operator(value: Json) -> BitwiseOperator:
    """Return one BitwiseOperator from one JSON value."""
    variant = json_string(value)

    if variant == "and":
        return "and"
    elif variant == "xor":
        return "xor"
    elif variant == "or":
        return "or"
    elif variant == "not":
        return "not"
    elif variant == "shiftLeft":
        return "shiftLeft"
    elif variant == "shiftRight":
        return "shiftRight"
    elif variant == "unsignedShiftRight":
        return "unsignedShiftRight"
    elif variant == "andAssign":
        return "andAssign"
    elif variant == "xorAssign":
        return "xorAssign"
    elif variant == "orAssign":
        return "orAssign"
    elif variant == "shiftLeftAssign":
        return "shiftLeftAssign"
    elif variant == "shiftRightAssign":
        return "shiftRightAssign"
    elif variant == "unsignedShiftRightAssign":
        return "unsignedShiftRightAssign"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Warning comment term matching location for `no-warning-comments`."""
WarningCommentLocation: typing.TypeAlias = (
    typing.Literal["start"] | typing.Literal["anywhere"]
)


def encode_warning_comment_location(
    writer: BinaryWriter, value: WarningCommentLocation
) -> None:
    """Encode one WarningCommentLocation."""
    if value == "start":
        writer.write_unsigned(0)
    elif value == "anywhere":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_warning_comment_location(reader: BinaryReader) -> WarningCommentLocation:
    """Decode one WarningCommentLocation."""
    variant = reader.read_number()

    if variant == 0:
        return "start"
    elif variant == 1:
        return "anywhere"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_warning_comment_location(value: WarningCommentLocation) -> Json:
    """Return one JSON value for one WarningCommentLocation."""
    return value


def from_json_warning_comment_location(value: Json) -> WarningCommentLocation:
    """Return one WarningCommentLocation from one JSON value."""
    variant = json_string(value)

    if variant == "start":
        return "start"
    elif variant == "anywhere":
        return "anywhere"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "LinterOptions",
    "encode_linter_options",
    "decode_linter_options",
    "to_json_linter_options",
    "from_json_linter_options",
    "LintPreset",
    "encode_lint_preset",
    "decode_lint_preset",
    "to_json_lint_preset",
    "from_json_lint_preset",
    "LintCategory",
    "encode_lint_category",
    "decode_lint_category",
    "to_json_lint_category",
    "from_json_lint_category",
    "LintSeverity",
    "encode_lint_severity",
    "decode_lint_severity",
    "to_json_lint_severity",
    "from_json_lint_severity",
    "ConditionAssignmentMode",
    "encode_condition_assignment_mode",
    "decode_condition_assignment_mode",
    "to_json_condition_assignment_mode",
    "from_json_condition_assignment_mode",
    "EmptyFunctionKind",
    "encode_empty_function_kind",
    "decode_empty_function_kind",
    "to_json_empty_function_kind",
    "from_json_empty_function_kind",
    "ReturnAwaitMode",
    "encode_return_await_mode",
    "decode_return_await_mode",
    "to_json_return_await_mode",
    "from_json_return_await_mode",
    "UnicodeRegexpRequireFlag",
    "encode_unicode_regexp_require_flag",
    "decode_unicode_regexp_require_flag",
    "to_json_unicode_regexp_require_flag",
    "from_json_unicode_regexp_require_flag",
    "ArrayTypeStyle",
    "encode_array_type_style",
    "decode_array_type_style",
    "to_json_array_type_style",
    "from_json_array_type_style",
    "FilenameCase",
    "encode_filename_case",
    "decode_filename_case",
    "to_json_filename_case",
    "from_json_filename_case",
    "GroupedAccessorPairsOrder",
    "encode_grouped_accessor_pairs_order",
    "decode_grouped_accessor_pairs_order",
    "to_json_grouped_accessor_pairs_order",
    "from_json_grouped_accessor_pairs_order",
    "OperatorAssignmentMode",
    "encode_operator_assignment_mode",
    "decode_operator_assignment_mode",
    "to_json_operator_assignment_mode",
    "from_json_operator_assignment_mode",
    "ObjectShorthandMode",
    "encode_object_shorthand_mode",
    "decode_object_shorthand_mode",
    "to_json_object_shorthand_mode",
    "from_json_object_shorthand_mode",
    "PreferConstDestructuring",
    "encode_prefer_const_destructuring",
    "decode_prefer_const_destructuring",
    "to_json_prefer_const_destructuring",
    "from_json_prefer_const_destructuring",
    "YodaMode",
    "encode_yoda_mode",
    "decode_yoda_mode",
    "to_json_yoda_mode",
    "from_json_yoda_mode",
    "SortImportsMemberSyntax",
    "encode_sort_imports_member_syntax",
    "decode_sort_imports_member_syntax",
    "to_json_sort_imports_member_syntax",
    "from_json_sort_imports_member_syntax",
    "CyclomaticComplexityVariant",
    "encode_cyclomatic_complexity_variant",
    "decode_cyclomatic_complexity_variant",
    "to_json_cyclomatic_complexity_variant",
    "from_json_cyclomatic_complexity_variant",
    "MaxParamsCountThis",
    "encode_max_params_count_this",
    "decode_max_params_count_this",
    "to_json_max_params_count_this",
    "from_json_max_params_count_this",
    "BitwiseOperator",
    "encode_bitwise_operator",
    "decode_bitwise_operator",
    "to_json_bitwise_operator",
    "from_json_bitwise_operator",
    "WarningCommentLocation",
    "encode_warning_comment_location",
    "decode_warning_comment_location",
    "to_json_warning_comment_location",
    "from_json_warning_comment_location",
]
