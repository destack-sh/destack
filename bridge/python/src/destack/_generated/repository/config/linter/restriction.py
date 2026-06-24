# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_array,
    json_bool,
    json_field,
    json_number,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.repository.config.compiler
import destack._generated.repository.config.linter.core


@dataclass(frozen=True, slots=True)
class LinterRestrictionOptions:
    """Restriction-category linter options."""

    # bitwise operators allowed by `no-bitwise`
    allowed_bitwise_operators: Sequence[
        destack._generated.repository.config.linter.core.BitwiseOperator
    ]
    # allow `x | 0` int32 cast hints in `no-bitwise`
    allow_bitwise_int32_hint: bool
    # console methods allowed by `no-console`
    allowed_console_methods: Sequence[str]
    # allow labels on loop statements in `no-labels`
    allow_loop_labels: bool
    # allow labels on switch statements in `no-labels`
    allow_switch_labels: bool
    # magic numbers to allow
    allowed_magic_numbers: Sequence[float]
    # allow `++` and `--` in for-loop afterthoughts for `no-plusplus`
    allow_plusplus_for_loop_afterthoughts: bool
    # where `no-warning-comments` should match terms
    warning_comment_location: (
        destack._generated.repository.config.linter.core.WarningCommentLocation
    )
    # decoration characters to ignore at the start of `no-warning-comments`
    warning_comment_decoration: Sequence[str]
    # globals to restrict
    restricted_globals: Sequence[str]
    # import paths to restrict
    restricted_imports: Sequence[str]
    # comment terms to warn on
    warning_comment_terms: Sequence[str]
    # module boundary constraints for module boundary aware lints
    module_boundaries: LintModuleBoundariesOptions

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_linter_restriction_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> LinterRestrictionOptions:
        """Decode one LinterRestrictionOptions."""
        return decode_linter_restriction_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_linter_restriction_options(self)

    @classmethod
    def from_json(cls, value: Json) -> LinterRestrictionOptions:
        """Return one LinterRestrictionOptions from one JSON value."""
        return from_json_linter_restriction_options(value)


def encode_linter_restriction_options(
    writer: BinaryWriter, value: LinterRestrictionOptions
) -> None:
    """Encode one LinterRestrictionOptions."""
    writer.write_unsigned(len(value.allowed_bitwise_operators))
    for item_value_allowed_bitwise_operators_0 in value.allowed_bitwise_operators:
        destack._generated.repository.config.linter.core.encode_bitwise_operator(
            writer, item_value_allowed_bitwise_operators_0
        )
    writer.write_bool(value.allow_bitwise_int32_hint)
    writer.write_unsigned(len(value.allowed_console_methods))
    for item_value_allowed_console_methods_0 in value.allowed_console_methods:
        writer.write_string(item_value_allowed_console_methods_0)
    writer.write_bool(value.allow_loop_labels)
    writer.write_bool(value.allow_switch_labels)
    writer.write_unsigned(len(value.allowed_magic_numbers))
    for item_value_allowed_magic_numbers_0 in value.allowed_magic_numbers:
        writer.write_f64(item_value_allowed_magic_numbers_0)
    writer.write_bool(value.allow_plusplus_for_loop_afterthoughts)
    destack._generated.repository.config.linter.core.encode_warning_comment_location(
        writer, value.warning_comment_location
    )
    writer.write_unsigned(len(value.warning_comment_decoration))
    for item_value_warning_comment_decoration_0 in value.warning_comment_decoration:
        writer.write_string(item_value_warning_comment_decoration_0)
    writer.write_unsigned(len(value.restricted_globals))
    for item_value_restricted_globals_0 in value.restricted_globals:
        writer.write_string(item_value_restricted_globals_0)
    writer.write_unsigned(len(value.restricted_imports))
    for item_value_restricted_imports_0 in value.restricted_imports:
        writer.write_string(item_value_restricted_imports_0)
    writer.write_unsigned(len(value.warning_comment_terms))
    for item_value_warning_comment_terms_0 in value.warning_comment_terms:
        writer.write_string(item_value_warning_comment_terms_0)
    encode_lint_module_boundaries_options(writer, value.module_boundaries)


def decode_linter_restriction_options(reader: BinaryReader) -> LinterRestrictionOptions:
    """Decode one LinterRestrictionOptions."""
    allowed_bitwise_operators = [
        destack._generated.repository.config.linter.core.decode_bitwise_operator(reader)
        for _ in range(reader.read_number())
    ]
    allow_bitwise_int32_hint = reader.read_bool()
    allowed_console_methods = [
        reader.read_string() for _ in range(reader.read_number())
    ]
    allow_loop_labels = reader.read_bool()
    allow_switch_labels = reader.read_bool()
    allowed_magic_numbers = [reader.read_f64() for _ in range(reader.read_number())]
    allow_plusplus_for_loop_afterthoughts = reader.read_bool()
    warning_comment_location = destack._generated.repository.config.linter.core.decode_warning_comment_location(
        reader
    )
    warning_comment_decoration = [
        reader.read_string() for _ in range(reader.read_number())
    ]
    restricted_globals = [reader.read_string() for _ in range(reader.read_number())]
    restricted_imports = [reader.read_string() for _ in range(reader.read_number())]
    warning_comment_terms = [reader.read_string() for _ in range(reader.read_number())]
    module_boundaries = decode_lint_module_boundaries_options(reader)

    return LinterRestrictionOptions(
        allowed_bitwise_operators=allowed_bitwise_operators,
        allow_bitwise_int32_hint=allow_bitwise_int32_hint,
        allowed_console_methods=allowed_console_methods,
        allow_loop_labels=allow_loop_labels,
        allow_switch_labels=allow_switch_labels,
        allowed_magic_numbers=allowed_magic_numbers,
        allow_plusplus_for_loop_afterthoughts=allow_plusplus_for_loop_afterthoughts,
        warning_comment_location=warning_comment_location,
        warning_comment_decoration=warning_comment_decoration,
        restricted_globals=restricted_globals,
        restricted_imports=restricted_imports,
        warning_comment_terms=warning_comment_terms,
        module_boundaries=module_boundaries,
    )


def to_json_linter_restriction_options(value: LinterRestrictionOptions) -> Json:
    """Return one JSON value for one LinterRestrictionOptions."""
    return {
        "allowedBitwiseOperators": [
            destack._generated.repository.config.linter.core.to_json_bitwise_operator(
                item_0
            )
            for item_0 in value.allowed_bitwise_operators
        ],
        "allowBitwiseInt32Hint": value.allow_bitwise_int32_hint,
        "allowedConsoleMethods": [item_0 for item_0 in value.allowed_console_methods],
        "allowLoopLabels": value.allow_loop_labels,
        "allowSwitchLabels": value.allow_switch_labels,
        "allowedMagicNumbers": [item_0 for item_0 in value.allowed_magic_numbers],
        "allowPlusplusForLoopAfterthoughts": value.allow_plusplus_for_loop_afterthoughts,
        "warningCommentLocation": destack._generated.repository.config.linter.core.to_json_warning_comment_location(
            value.warning_comment_location
        ),
        "warningCommentDecoration": [
            item_0 for item_0 in value.warning_comment_decoration
        ],
        "restrictedGlobals": [item_0 for item_0 in value.restricted_globals],
        "restrictedImports": [item_0 for item_0 in value.restricted_imports],
        "warningCommentTerms": [item_0 for item_0 in value.warning_comment_terms],
        "moduleBoundaries": to_json_lint_module_boundaries_options(
            value.module_boundaries
        ),
    }


def from_json_linter_restriction_options(value: Json) -> LinterRestrictionOptions:
    """Return one LinterRestrictionOptions from one JSON value."""
    object_ = json_object(value)

    return LinterRestrictionOptions(
        allowed_bitwise_operators=[
            destack._generated.repository.config.linter.core.from_json_bitwise_operator(
                item_0
            )
            for item_0 in json_array(json_field(object_, "allowedBitwiseOperators"))
        ],
        allow_bitwise_int32_hint=json_bool(
            json_field(object_, "allowBitwiseInt32Hint")
        ),
        allowed_console_methods=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "allowedConsoleMethods"))
        ],
        allow_loop_labels=json_bool(json_field(object_, "allowLoopLabels")),
        allow_switch_labels=json_bool(json_field(object_, "allowSwitchLabels")),
        allowed_magic_numbers=[
            json_number(item_0)
            for item_0 in json_array(json_field(object_, "allowedMagicNumbers"))
        ],
        allow_plusplus_for_loop_afterthoughts=json_bool(
            json_field(object_, "allowPlusplusForLoopAfterthoughts")
        ),
        warning_comment_location=destack._generated.repository.config.linter.core.from_json_warning_comment_location(
            json_field(object_, "warningCommentLocation")
        ),
        warning_comment_decoration=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "warningCommentDecoration"))
        ],
        restricted_globals=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "restrictedGlobals"))
        ],
        restricted_imports=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "restrictedImports"))
        ],
        warning_comment_terms=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "warningCommentTerms"))
        ],
        module_boundaries=from_json_lint_module_boundaries_options(
            json_field(object_, "moduleBoundaries")
        ),
    )


@dataclass(frozen=True, slots=True)
class LintModuleBoundariesOptions:
    """Module boundary lint options for module boundary aware rules."""

    # policy for modules that do not match any configured component
    unknown_component_policy: (
        destack._generated.repository.config.compiler.DiagnosticPolicy
    )
    # declared components and their path match patterns
    components: Sequence[LintModuleComponent]
    # allowed component to component dependency rules
    dependency_rules: Sequence[LintModuleDependencyRule]
    # explicit dependency exceptions
    exceptions: Sequence[LintModuleDependencyException]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_lint_module_boundaries_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> LintModuleBoundariesOptions:
        """Decode one LintModuleBoundariesOptions."""
        return decode_lint_module_boundaries_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_lint_module_boundaries_options(self)

    @classmethod
    def from_json(cls, value: Json) -> LintModuleBoundariesOptions:
        """Return one LintModuleBoundariesOptions from one JSON value."""
        return from_json_lint_module_boundaries_options(value)


def encode_lint_module_boundaries_options(
    writer: BinaryWriter, value: LintModuleBoundariesOptions
) -> None:
    """Encode one LintModuleBoundariesOptions."""
    destack._generated.repository.config.compiler.encode_diagnostic_policy(
        writer, value.unknown_component_policy
    )
    writer.write_unsigned(len(value.components))
    for item_value_components_0 in value.components:
        encode_lint_module_component(writer, item_value_components_0)
    writer.write_unsigned(len(value.dependency_rules))
    for item_value_dependency_rules_0 in value.dependency_rules:
        encode_lint_module_dependency_rule(writer, item_value_dependency_rules_0)
    writer.write_unsigned(len(value.exceptions))
    for item_value_exceptions_0 in value.exceptions:
        encode_lint_module_dependency_exception(writer, item_value_exceptions_0)


def decode_lint_module_boundaries_options(
    reader: BinaryReader,
) -> LintModuleBoundariesOptions:
    """Decode one LintModuleBoundariesOptions."""
    unknown_component_policy = (
        destack._generated.repository.config.compiler.decode_diagnostic_policy(reader)
    )
    components = [
        decode_lint_module_component(reader) for _ in range(reader.read_number())
    ]
    dependency_rules = [
        decode_lint_module_dependency_rule(reader) for _ in range(reader.read_number())
    ]
    exceptions = [
        decode_lint_module_dependency_exception(reader)
        for _ in range(reader.read_number())
    ]

    return LintModuleBoundariesOptions(
        unknown_component_policy=unknown_component_policy,
        components=components,
        dependency_rules=dependency_rules,
        exceptions=exceptions,
    )


def to_json_lint_module_boundaries_options(value: LintModuleBoundariesOptions) -> Json:
    """Return one JSON value for one LintModuleBoundariesOptions."""
    return {
        "unknownComponentPolicy": destack._generated.repository.config.compiler.to_json_diagnostic_policy(
            value.unknown_component_policy
        ),
        "components": [
            to_json_lint_module_component(item_0) for item_0 in value.components
        ],
        "dependencyRules": [
            to_json_lint_module_dependency_rule(item_0)
            for item_0 in value.dependency_rules
        ],
        "exceptions": [
            to_json_lint_module_dependency_exception(item_0)
            for item_0 in value.exceptions
        ],
    }


def from_json_lint_module_boundaries_options(
    value: Json,
) -> LintModuleBoundariesOptions:
    """Return one LintModuleBoundariesOptions from one JSON value."""
    object_ = json_object(value)

    return LintModuleBoundariesOptions(
        unknown_component_policy=destack._generated.repository.config.compiler.from_json_diagnostic_policy(
            json_field(object_, "unknownComponentPolicy")
        ),
        components=[
            from_json_lint_module_component(item_0)
            for item_0 in json_array(json_field(object_, "components"))
        ],
        dependency_rules=[
            from_json_lint_module_dependency_rule(item_0)
            for item_0 in json_array(json_field(object_, "dependencyRules"))
        ],
        exceptions=[
            from_json_lint_module_dependency_exception(item_0)
            for item_0 in json_array(json_field(object_, "exceptions"))
        ],
    )


@dataclass(frozen=True, slots=True)
class LintModuleComponent:
    """One module component declaration."""

    # the unique component name
    name: str
    # glob patterns used to match module paths into this component
    path_patterns: Sequence[str]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_lint_module_component(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> LintModuleComponent:
        """Decode one LintModuleComponent."""
        return decode_lint_module_component(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_lint_module_component(self)

    @classmethod
    def from_json(cls, value: Json) -> LintModuleComponent:
        """Return one LintModuleComponent from one JSON value."""
        return from_json_lint_module_component(value)


def encode_lint_module_component(
    writer: BinaryWriter, value: LintModuleComponent
) -> None:
    """Encode one LintModuleComponent."""
    writer.write_string(value.name)
    writer.write_unsigned(len(value.path_patterns))
    for item_value_path_patterns_0 in value.path_patterns:
        writer.write_string(item_value_path_patterns_0)


def decode_lint_module_component(reader: BinaryReader) -> LintModuleComponent:
    """Decode one LintModuleComponent."""
    name = reader.read_string()
    path_patterns = [reader.read_string() for _ in range(reader.read_number())]

    return LintModuleComponent(
        name=name,
        path_patterns=path_patterns,
    )


def to_json_lint_module_component(value: LintModuleComponent) -> Json:
    """Return one JSON value for one LintModuleComponent."""
    return {
        "name": value.name,
        "pathPatterns": [item_0 for item_0 in value.path_patterns],
    }


def from_json_lint_module_component(value: Json) -> LintModuleComponent:
    """Return one LintModuleComponent from one JSON value."""
    object_ = json_object(value)

    return LintModuleComponent(
        name=json_string(json_field(object_, "name")),
        path_patterns=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "pathPatterns"))
        ],
    )


@dataclass(frozen=True, slots=True)
class LintModuleDependencyRule:
    """One allowed dependency rule between module components."""

    # the source component name
    from_: str
    # destination components this source component may import
    allow: Sequence[str]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_lint_module_dependency_rule(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> LintModuleDependencyRule:
        """Decode one LintModuleDependencyRule."""
        return decode_lint_module_dependency_rule(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_lint_module_dependency_rule(self)

    @classmethod
    def from_json(cls, value: Json) -> LintModuleDependencyRule:
        """Return one LintModuleDependencyRule from one JSON value."""
        return from_json_lint_module_dependency_rule(value)


def encode_lint_module_dependency_rule(
    writer: BinaryWriter, value: LintModuleDependencyRule
) -> None:
    """Encode one LintModuleDependencyRule."""
    writer.write_string(value.from_)
    writer.write_unsigned(len(value.allow))
    for item_value_allow_0 in value.allow:
        writer.write_string(item_value_allow_0)


def decode_lint_module_dependency_rule(
    reader: BinaryReader,
) -> LintModuleDependencyRule:
    """Decode one LintModuleDependencyRule."""
    from_ = reader.read_string()
    allow = [reader.read_string() for _ in range(reader.read_number())]

    return LintModuleDependencyRule(
        from_=from_,
        allow=allow,
    )


def to_json_lint_module_dependency_rule(value: LintModuleDependencyRule) -> Json:
    """Return one JSON value for one LintModuleDependencyRule."""
    return {
        "from": value.from_,
        "allow": [item_0 for item_0 in value.allow],
    }


def from_json_lint_module_dependency_rule(value: Json) -> LintModuleDependencyRule:
    """Return one LintModuleDependencyRule from one JSON value."""
    object_ = json_object(value)

    return LintModuleDependencyRule(
        from_=json_string(json_field(object_, "from")),
        allow=[
            json_string(item_0) for item_0 in json_array(json_field(object_, "allow"))
        ],
    )


@dataclass(frozen=True, slots=True)
class LintModuleDependencyException:
    """One module dependency exception for specific module path patterns."""

    # the source component name for this exception
    from_: str
    # the destination component name for this exception
    to: str
    # module path patterns where this exception is allowed
    path_patterns: Sequence[str]
    # optional human-readable reason for this exception
    reason: str | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_lint_module_dependency_exception(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> LintModuleDependencyException:
        """Decode one LintModuleDependencyException."""
        return decode_lint_module_dependency_exception(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_lint_module_dependency_exception(self)

    @classmethod
    def from_json(cls, value: Json) -> LintModuleDependencyException:
        """Return one LintModuleDependencyException from one JSON value."""
        return from_json_lint_module_dependency_exception(value)


def encode_lint_module_dependency_exception(
    writer: BinaryWriter, value: LintModuleDependencyException
) -> None:
    """Encode one LintModuleDependencyException."""
    writer.write_string(value.from_)
    writer.write_string(value.to)
    writer.write_unsigned(len(value.path_patterns))
    for item_value_path_patterns_0 in value.path_patterns:
        writer.write_string(item_value_path_patterns_0)
    if value.reason is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.reason)


def decode_lint_module_dependency_exception(
    reader: BinaryReader,
) -> LintModuleDependencyException:
    """Decode one LintModuleDependencyException."""
    from_ = reader.read_string()
    to = reader.read_string()
    path_patterns = [reader.read_string() for _ in range(reader.read_number())]
    reason = reader.read_option(lambda: reader.read_string())

    return LintModuleDependencyException(
        from_=from_,
        to=to,
        path_patterns=path_patterns,
        reason=reason,
    )


def to_json_lint_module_dependency_exception(
    value: LintModuleDependencyException,
) -> Json:
    """Return one JSON value for one LintModuleDependencyException."""
    return {
        "from": value.from_,
        "to": value.to,
        "pathPatterns": [item_0 for item_0 in value.path_patterns],
        **({} if value.reason is None else {"reason": value.reason}),
    }


def from_json_lint_module_dependency_exception(
    value: Json,
) -> LintModuleDependencyException:
    """Return one LintModuleDependencyException from one JSON value."""
    object_ = json_object(value)

    return LintModuleDependencyException(
        from_=json_string(json_field(object_, "from")),
        to=json_string(json_field(object_, "to")),
        path_patterns=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "pathPatterns"))
        ],
        reason=json_optional(object_, "reason", lambda value: json_string(value)),
    )


__all__ = [
    "LinterRestrictionOptions",
    "encode_linter_restriction_options",
    "decode_linter_restriction_options",
    "to_json_linter_restriction_options",
    "from_json_linter_restriction_options",
    "LintModuleBoundariesOptions",
    "encode_lint_module_boundaries_options",
    "decode_lint_module_boundaries_options",
    "to_json_lint_module_boundaries_options",
    "from_json_lint_module_boundaries_options",
    "LintModuleComponent",
    "encode_lint_module_component",
    "decode_lint_module_component",
    "to_json_lint_module_component",
    "from_json_lint_module_component",
    "LintModuleDependencyRule",
    "encode_lint_module_dependency_rule",
    "decode_lint_module_dependency_rule",
    "to_json_lint_module_dependency_rule",
    "from_json_lint_module_dependency_rule",
    "LintModuleDependencyException",
    "encode_lint_module_dependency_exception",
    "decode_lint_module_dependency_exception",
    "to_json_lint_module_dependency_exception",
    "from_json_lint_module_dependency_exception",
]
