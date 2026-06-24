# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> LinterOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> LinterOptions: ...

def encode_linter_options(writer: BinaryWriter, value: LinterOptions) -> None: ...
def decode_linter_options(reader: BinaryReader) -> LinterOptions: ...
def to_json_linter_options(value: LinterOptions) -> Json: ...
def from_json_linter_options(value: Json) -> LinterOptions: ...

"""Lint rule preset."""
LintPreset: typing.TypeAlias = (
    typing.Literal["none"]
    | typing.Literal["recommended"]
    | typing.Literal["strict"]
    | typing.Literal["all"]
)

def encode_lint_preset(writer: BinaryWriter, value: LintPreset) -> None: ...
def decode_lint_preset(reader: BinaryReader) -> LintPreset: ...
def to_json_lint_preset(value: LintPreset) -> Json: ...
def from_json_lint_preset(value: Json) -> LintPreset: ...

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

def encode_lint_category(writer: BinaryWriter, value: LintCategory) -> None: ...
def decode_lint_category(reader: BinaryReader) -> LintCategory: ...
def to_json_lint_category(value: LintCategory) -> Json: ...
def from_json_lint_category(value: Json) -> LintCategory: ...

"""Rule severity level."""
LintSeverity: typing.TypeAlias = (
    typing.Literal["off"]
    | typing.Literal["note"]
    | typing.Literal["warning"]
    | typing.Literal["error"]
)

def encode_lint_severity(writer: BinaryWriter, value: LintSeverity) -> None: ...
def decode_lint_severity(reader: BinaryReader) -> LintSeverity: ...
def to_json_lint_severity(value: LintSeverity) -> Json: ...
def from_json_lint_severity(value: Json) -> LintSeverity: ...

"""Condition assignment policy for `no-cond-assign`."""
ConditionAssignmentMode: typing.TypeAlias = (
    typing.Literal["exceptParens"] | typing.Literal["always"]
)

def encode_condition_assignment_mode(
    writer: BinaryWriter, value: ConditionAssignmentMode
) -> None: ...
def decode_condition_assignment_mode(
    reader: BinaryReader,
) -> ConditionAssignmentMode: ...
def to_json_condition_assignment_mode(value: ConditionAssignmentMode) -> Json: ...
def from_json_condition_assignment_mode(value: Json) -> ConditionAssignmentMode: ...

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

def encode_empty_function_kind(
    writer: BinaryWriter, value: EmptyFunctionKind
) -> None: ...
def decode_empty_function_kind(reader: BinaryReader) -> EmptyFunctionKind: ...
def to_json_empty_function_kind(value: EmptyFunctionKind) -> Json: ...
def from_json_empty_function_kind(value: Json) -> EmptyFunctionKind: ...

"""Return-await mode for the `return-await` rule."""
ReturnAwaitMode: typing.TypeAlias = (
    typing.Literal["inTryCatch"]
    | typing.Literal["errorHandlingCorrectnessOnly"]
    | typing.Literal["always"]
    | typing.Literal["never"]
)

def encode_return_await_mode(writer: BinaryWriter, value: ReturnAwaitMode) -> None: ...
def decode_return_await_mode(reader: BinaryReader) -> ReturnAwaitMode: ...
def to_json_return_await_mode(value: ReturnAwaitMode) -> Json: ...
def from_json_return_await_mode(value: Json) -> ReturnAwaitMode: ...

"""Required Unicode regex flag for `require-unicode-regexp`."""
UnicodeRegexpRequireFlag: typing.TypeAlias = typing.Literal["u"] | typing.Literal["v"]

def encode_unicode_regexp_require_flag(
    writer: BinaryWriter, value: UnicodeRegexpRequireFlag
) -> None: ...
def decode_unicode_regexp_require_flag(
    reader: BinaryReader,
) -> UnicodeRegexpRequireFlag: ...
def to_json_unicode_regexp_require_flag(value: UnicodeRegexpRequireFlag) -> Json: ...
def from_json_unicode_regexp_require_flag(value: Json) -> UnicodeRegexpRequireFlag: ...

"""Preferred array type syntax for the `array-type` rule."""
ArrayTypeStyle: typing.TypeAlias = typing.Literal["array"] | typing.Literal["generic"]

def encode_array_type_style(writer: BinaryWriter, value: ArrayTypeStyle) -> None: ...
def decode_array_type_style(reader: BinaryReader) -> ArrayTypeStyle: ...
def to_json_array_type_style(value: ArrayTypeStyle) -> Json: ...
def from_json_array_type_style(value: Json) -> ArrayTypeStyle: ...

"""Filename case style for the `filename-case` rule."""
FilenameCase: typing.TypeAlias = (
    typing.Literal["kebab"]
    | typing.Literal["snake"]
    | typing.Literal["camel"]
    | typing.Literal["pascal"]
)

def encode_filename_case(writer: BinaryWriter, value: FilenameCase) -> None: ...
def decode_filename_case(reader: BinaryReader) -> FilenameCase: ...
def to_json_filename_case(value: FilenameCase) -> Json: ...
def from_json_filename_case(value: Json) -> FilenameCase: ...

"""Ordering policy for `grouped-accessor-pairs`."""
GroupedAccessorPairsOrder: typing.TypeAlias = (
    typing.Literal["anyOrder"]
    | typing.Literal["getBeforeSet"]
    | typing.Literal["setBeforeGet"]
)

def encode_grouped_accessor_pairs_order(
    writer: BinaryWriter, value: GroupedAccessorPairsOrder
) -> None: ...
def decode_grouped_accessor_pairs_order(
    reader: BinaryReader,
) -> GroupedAccessorPairsOrder: ...
def to_json_grouped_accessor_pairs_order(value: GroupedAccessorPairsOrder) -> Json: ...
def from_json_grouped_accessor_pairs_order(
    value: Json,
) -> GroupedAccessorPairsOrder: ...

"""Enforcement mode for `operator-assignment`."""
OperatorAssignmentMode: typing.TypeAlias = (
    typing.Literal["always"] | typing.Literal["never"]
)

def encode_operator_assignment_mode(
    writer: BinaryWriter, value: OperatorAssignmentMode
) -> None: ...
def decode_operator_assignment_mode(reader: BinaryReader) -> OperatorAssignmentMode: ...
def to_json_operator_assignment_mode(value: OperatorAssignmentMode) -> Json: ...
def from_json_operator_assignment_mode(value: Json) -> OperatorAssignmentMode: ...

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
) -> None: ...
def decode_object_shorthand_mode(reader: BinaryReader) -> ObjectShorthandMode: ...
def to_json_object_shorthand_mode(value: ObjectShorthandMode) -> Json: ...
def from_json_object_shorthand_mode(value: Json) -> ObjectShorthandMode: ...

"""Destructuring policy for `prefer-const`."""
PreferConstDestructuring: typing.TypeAlias = (
    typing.Literal["any"] | typing.Literal["all"]
)

def encode_prefer_const_destructuring(
    writer: BinaryWriter, value: PreferConstDestructuring
) -> None: ...
def decode_prefer_const_destructuring(
    reader: BinaryReader,
) -> PreferConstDestructuring: ...
def to_json_prefer_const_destructuring(value: PreferConstDestructuring) -> Json: ...
def from_json_prefer_const_destructuring(value: Json) -> PreferConstDestructuring: ...

"""Enforcement mode for `yoda`."""
YodaMode: typing.TypeAlias = typing.Literal["always"] | typing.Literal["never"]

def encode_yoda_mode(writer: BinaryWriter, value: YodaMode) -> None: ...
def decode_yoda_mode(reader: BinaryReader) -> YodaMode: ...
def to_json_yoda_mode(value: YodaMode) -> Json: ...
def from_json_yoda_mode(value: Json) -> YodaMode: ...

"""Member syntax groups for `sort-imports`."""
SortImportsMemberSyntax: typing.TypeAlias = (
    typing.Literal["none"]
    | typing.Literal["all"]
    | typing.Literal["multiple"]
    | typing.Literal["single"]
)

def encode_sort_imports_member_syntax(
    writer: BinaryWriter, value: SortImportsMemberSyntax
) -> None: ...
def decode_sort_imports_member_syntax(
    reader: BinaryReader,
) -> SortImportsMemberSyntax: ...
def to_json_sort_imports_member_syntax(value: SortImportsMemberSyntax) -> Json: ...
def from_json_sort_imports_member_syntax(value: Json) -> SortImportsMemberSyntax: ...

"""Switch counting variant for `cyclomatic-complexity`."""
CyclomaticComplexityVariant: typing.TypeAlias = (
    typing.Literal["classic"] | typing.Literal["modified"]
)

def encode_cyclomatic_complexity_variant(
    writer: BinaryWriter, value: CyclomaticComplexityVariant
) -> None: ...
def decode_cyclomatic_complexity_variant(
    reader: BinaryReader,
) -> CyclomaticComplexityVariant: ...
def to_json_cyclomatic_complexity_variant(
    value: CyclomaticComplexityVariant,
) -> Json: ...
def from_json_cyclomatic_complexity_variant(
    value: Json,
) -> CyclomaticComplexityVariant: ...

"""`this` parameter counting policy for `max-params`."""
MaxParamsCountThis: typing.TypeAlias = (
    typing.Literal["never"] | typing.Literal["exceptVoid"] | typing.Literal["always"]
)

def encode_max_params_count_this(
    writer: BinaryWriter, value: MaxParamsCountThis
) -> None: ...
def decode_max_params_count_this(reader: BinaryReader) -> MaxParamsCountThis: ...
def to_json_max_params_count_this(value: MaxParamsCountThis) -> Json: ...
def from_json_max_params_count_this(value: Json) -> MaxParamsCountThis: ...

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

def encode_bitwise_operator(writer: BinaryWriter, value: BitwiseOperator) -> None: ...
def decode_bitwise_operator(reader: BinaryReader) -> BitwiseOperator: ...
def to_json_bitwise_operator(value: BitwiseOperator) -> Json: ...
def from_json_bitwise_operator(value: Json) -> BitwiseOperator: ...

"""Warning comment term matching location for `no-warning-comments`."""
WarningCommentLocation: typing.TypeAlias = (
    typing.Literal["start"] | typing.Literal["anywhere"]
)

def encode_warning_comment_location(
    writer: BinaryWriter, value: WarningCommentLocation
) -> None: ...
def decode_warning_comment_location(reader: BinaryReader) -> WarningCommentLocation: ...
def to_json_warning_comment_location(value: WarningCommentLocation) -> Json: ...
def from_json_warning_comment_location(value: Json) -> WarningCommentLocation: ...

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
