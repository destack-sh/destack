# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.repository.config.linter.core

@dataclass(frozen=True, slots=True)
class LinterCorrectnessOptions:
    """Correctness-category linter options."""

    # ignore explicit `void` wrappers in `no-floating-promises`
    no_floating_promises_ignore_void: bool
    # allow explicit `void` returns in `no-promise-executor-return`
    no_promise_executor_return_allow_void: bool
    # check callback positions in `no-misused-promises`
    no_misused_promises_check_callbacks: bool
    # check conditionals in `no-misused-promises`
    no_misused_promises_check_conditionals: bool
    # check spread positions in `no-misused-promises`
    no_misused_promises_check_spreads: bool
    # check switch discriminants and cases in `use-isnan`
    use_isnan_enforce_for_switch_case: bool
    # check `indexOf` and `lastIndexOf` calls in `use-isnan`
    use_isnan_enforce_for_index_of: bool
    # check property assignments in `no-self-assign`
    no_self_assign_check_properties: bool
    # ignore explicit `void` wrappers in `no-confusing-void-expression`
    no_confusing_void_expression_ignore_void_operator: bool
    # ignore returned void expressions inside void-returning functions
    no_confusing_void_expression_ignore_void_returning_functions: bool
    # assignment policy for `no-cond-assign`
    no_cond_assign_mode: (
        destack._generated.repository.config.linter.core.ConditionAssignmentMode
    )
    # allow empty catch blocks in `no-empty`
    no_empty_allow_empty_catch: bool
    # allow empty object patterns in parameter position for `no-empty-pattern`
    no_empty_pattern_allow_object_patterns_as_parameters: bool
    # allowed empty function kinds in `no-empty-function`
    no_empty_function_allow: Sequence[
        destack._generated.repository.config.linter.core.EmptyFunctionKind
    ]
    # allow empty switch cases in `no-fallthrough`
    no_fallthrough_allow_empty_case: bool
    # regex pattern for intentional `no-fallthrough` comments
    no_fallthrough_comment_pattern: str | None
    # report unused intentional `no-fallthrough` comments
    no_fallthrough_report_unused_comment: bool
    # await policy for the `return-await` rule
    return_await_mode: destack._generated.repository.config.linter.core.ReturnAwaitMode
    # parameter name prefixes ignored by `no-unused-parameters`
    ignored_unused_parameter_prefixes: Sequence[str]
    # ignore destructuring aliases in `no-useless-rename`
    no_useless_rename_ignore_destructuring: bool
    # ignore import aliases in `no-useless-rename`
    no_useless_rename_ignore_import: bool
    # ignore export aliases in `no-useless-rename`
    no_useless_rename_ignore_export: bool
    # regex characters allowed by `no-useless-escape`
    no_useless_escape_allow_regex_characters: Sequence[str]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> LinterCorrectnessOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> LinterCorrectnessOptions: ...

def encode_linter_correctness_options(
    writer: BinaryWriter, value: LinterCorrectnessOptions
) -> None: ...
def decode_linter_correctness_options(
    reader: BinaryReader,
) -> LinterCorrectnessOptions: ...
def to_json_linter_correctness_options(value: LinterCorrectnessOptions) -> Json: ...
def from_json_linter_correctness_options(value: Json) -> LinterCorrectnessOptions: ...

__all__ = [
    "LinterCorrectnessOptions",
    "encode_linter_correctness_options",
    "decode_linter_correctness_options",
    "to_json_linter_correctness_options",
    "from_json_linter_correctness_options",
]
