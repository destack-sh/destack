# generated client target, do not edit

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
    json_object,
    json_optional,
    json_string,
)

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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_linter_correctness_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> LinterCorrectnessOptions:
        """Decode one LinterCorrectnessOptions."""
        return decode_linter_correctness_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_linter_correctness_options(self)

    @classmethod
    def from_json(cls, value: Json) -> LinterCorrectnessOptions:
        """Return one LinterCorrectnessOptions from one JSON value."""
        return from_json_linter_correctness_options(value)


def encode_linter_correctness_options(
    writer: BinaryWriter, value: LinterCorrectnessOptions
) -> None:
    """Encode one LinterCorrectnessOptions."""
    writer.write_bool(value.no_floating_promises_ignore_void)
    writer.write_bool(value.no_promise_executor_return_allow_void)
    writer.write_bool(value.no_misused_promises_check_callbacks)
    writer.write_bool(value.no_misused_promises_check_conditionals)
    writer.write_bool(value.no_misused_promises_check_spreads)
    writer.write_bool(value.use_isnan_enforce_for_switch_case)
    writer.write_bool(value.use_isnan_enforce_for_index_of)
    writer.write_bool(value.no_self_assign_check_properties)
    writer.write_bool(value.no_confusing_void_expression_ignore_void_operator)
    writer.write_bool(
        value.no_confusing_void_expression_ignore_void_returning_functions
    )
    destack._generated.repository.config.linter.core.encode_condition_assignment_mode(
        writer, value.no_cond_assign_mode
    )
    writer.write_bool(value.no_empty_allow_empty_catch)
    writer.write_bool(value.no_empty_pattern_allow_object_patterns_as_parameters)
    writer.write_unsigned(len(value.no_empty_function_allow))
    for item_value_no_empty_function_allow_0 in value.no_empty_function_allow:
        destack._generated.repository.config.linter.core.encode_empty_function_kind(
            writer, item_value_no_empty_function_allow_0
        )
    writer.write_bool(value.no_fallthrough_allow_empty_case)
    if value.no_fallthrough_comment_pattern is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.no_fallthrough_comment_pattern)
    writer.write_bool(value.no_fallthrough_report_unused_comment)
    destack._generated.repository.config.linter.core.encode_return_await_mode(
        writer, value.return_await_mode
    )
    writer.write_unsigned(len(value.ignored_unused_parameter_prefixes))
    for (
        item_value_ignored_unused_parameter_prefixes_0
    ) in value.ignored_unused_parameter_prefixes:
        writer.write_string(item_value_ignored_unused_parameter_prefixes_0)
    writer.write_bool(value.no_useless_rename_ignore_destructuring)
    writer.write_bool(value.no_useless_rename_ignore_import)
    writer.write_bool(value.no_useless_rename_ignore_export)
    writer.write_unsigned(len(value.no_useless_escape_allow_regex_characters))
    for (
        item_value_no_useless_escape_allow_regex_characters_0
    ) in value.no_useless_escape_allow_regex_characters:
        writer.write_string(item_value_no_useless_escape_allow_regex_characters_0)


def decode_linter_correctness_options(reader: BinaryReader) -> LinterCorrectnessOptions:
    """Decode one LinterCorrectnessOptions."""
    no_floating_promises_ignore_void = reader.read_bool()
    no_promise_executor_return_allow_void = reader.read_bool()
    no_misused_promises_check_callbacks = reader.read_bool()
    no_misused_promises_check_conditionals = reader.read_bool()
    no_misused_promises_check_spreads = reader.read_bool()
    use_isnan_enforce_for_switch_case = reader.read_bool()
    use_isnan_enforce_for_index_of = reader.read_bool()
    no_self_assign_check_properties = reader.read_bool()
    no_confusing_void_expression_ignore_void_operator = reader.read_bool()
    no_confusing_void_expression_ignore_void_returning_functions = reader.read_bool()
    no_cond_assign_mode = destack._generated.repository.config.linter.core.decode_condition_assignment_mode(
        reader
    )
    no_empty_allow_empty_catch = reader.read_bool()
    no_empty_pattern_allow_object_patterns_as_parameters = reader.read_bool()
    no_empty_function_allow = [
        destack._generated.repository.config.linter.core.decode_empty_function_kind(
            reader
        )
        for _ in range(reader.read_number())
    ]
    no_fallthrough_allow_empty_case = reader.read_bool()
    no_fallthrough_comment_pattern = reader.read_option(lambda: reader.read_string())
    no_fallthrough_report_unused_comment = reader.read_bool()
    return_await_mode = (
        destack._generated.repository.config.linter.core.decode_return_await_mode(
            reader
        )
    )
    ignored_unused_parameter_prefixes = [
        reader.read_string() for _ in range(reader.read_number())
    ]
    no_useless_rename_ignore_destructuring = reader.read_bool()
    no_useless_rename_ignore_import = reader.read_bool()
    no_useless_rename_ignore_export = reader.read_bool()
    no_useless_escape_allow_regex_characters = [
        reader.read_string() for _ in range(reader.read_number())
    ]

    return LinterCorrectnessOptions(
        no_floating_promises_ignore_void=no_floating_promises_ignore_void,
        no_promise_executor_return_allow_void=no_promise_executor_return_allow_void,
        no_misused_promises_check_callbacks=no_misused_promises_check_callbacks,
        no_misused_promises_check_conditionals=no_misused_promises_check_conditionals,
        no_misused_promises_check_spreads=no_misused_promises_check_spreads,
        use_isnan_enforce_for_switch_case=use_isnan_enforce_for_switch_case,
        use_isnan_enforce_for_index_of=use_isnan_enforce_for_index_of,
        no_self_assign_check_properties=no_self_assign_check_properties,
        no_confusing_void_expression_ignore_void_operator=no_confusing_void_expression_ignore_void_operator,
        no_confusing_void_expression_ignore_void_returning_functions=no_confusing_void_expression_ignore_void_returning_functions,
        no_cond_assign_mode=no_cond_assign_mode,
        no_empty_allow_empty_catch=no_empty_allow_empty_catch,
        no_empty_pattern_allow_object_patterns_as_parameters=no_empty_pattern_allow_object_patterns_as_parameters,
        no_empty_function_allow=no_empty_function_allow,
        no_fallthrough_allow_empty_case=no_fallthrough_allow_empty_case,
        no_fallthrough_comment_pattern=no_fallthrough_comment_pattern,
        no_fallthrough_report_unused_comment=no_fallthrough_report_unused_comment,
        return_await_mode=return_await_mode,
        ignored_unused_parameter_prefixes=ignored_unused_parameter_prefixes,
        no_useless_rename_ignore_destructuring=no_useless_rename_ignore_destructuring,
        no_useless_rename_ignore_import=no_useless_rename_ignore_import,
        no_useless_rename_ignore_export=no_useless_rename_ignore_export,
        no_useless_escape_allow_regex_characters=no_useless_escape_allow_regex_characters,
    )


def to_json_linter_correctness_options(value: LinterCorrectnessOptions) -> Json:
    """Return one JSON value for one LinterCorrectnessOptions."""
    return {
        "noFloatingPromisesIgnoreVoid": value.no_floating_promises_ignore_void,
        "noPromiseExecutorReturnAllowVoid": value.no_promise_executor_return_allow_void,
        "noMisusedPromisesCheckCallbacks": value.no_misused_promises_check_callbacks,
        "noMisusedPromisesCheckConditionals": value.no_misused_promises_check_conditionals,
        "noMisusedPromisesCheckSpreads": value.no_misused_promises_check_spreads,
        "useIsnanEnforceForSwitchCase": value.use_isnan_enforce_for_switch_case,
        "useIsnanEnforceForIndexOf": value.use_isnan_enforce_for_index_of,
        "noSelfAssignCheckProperties": value.no_self_assign_check_properties,
        "noConfusingVoidExpressionIgnoreVoidOperator": value.no_confusing_void_expression_ignore_void_operator,
        "noConfusingVoidExpressionIgnoreVoidReturningFunctions": value.no_confusing_void_expression_ignore_void_returning_functions,
        "noCondAssignMode": destack._generated.repository.config.linter.core.to_json_condition_assignment_mode(
            value.no_cond_assign_mode
        ),
        "noEmptyAllowEmptyCatch": value.no_empty_allow_empty_catch,
        "noEmptyPatternAllowObjectPatternsAsParameters": value.no_empty_pattern_allow_object_patterns_as_parameters,
        "noEmptyFunctionAllow": [
            destack._generated.repository.config.linter.core.to_json_empty_function_kind(
                item_0
            )
            for item_0 in value.no_empty_function_allow
        ],
        "noFallthroughAllowEmptyCase": value.no_fallthrough_allow_empty_case,
        **(
            {}
            if value.no_fallthrough_comment_pattern is None
            else {"noFallthroughCommentPattern": value.no_fallthrough_comment_pattern}
        ),
        "noFallthroughReportUnusedComment": value.no_fallthrough_report_unused_comment,
        "returnAwaitMode": destack._generated.repository.config.linter.core.to_json_return_await_mode(
            value.return_await_mode
        ),
        "ignoredUnusedParameterPrefixes": [
            item_0 for item_0 in value.ignored_unused_parameter_prefixes
        ],
        "noUselessRenameIgnoreDestructuring": value.no_useless_rename_ignore_destructuring,
        "noUselessRenameIgnoreImport": value.no_useless_rename_ignore_import,
        "noUselessRenameIgnoreExport": value.no_useless_rename_ignore_export,
        "noUselessEscapeAllowRegexCharacters": [
            item_0 for item_0 in value.no_useless_escape_allow_regex_characters
        ],
    }


def from_json_linter_correctness_options(value: Json) -> LinterCorrectnessOptions:
    """Return one LinterCorrectnessOptions from one JSON value."""
    object_ = json_object(value)

    return LinterCorrectnessOptions(
        no_floating_promises_ignore_void=json_bool(
            json_field(object_, "noFloatingPromisesIgnoreVoid")
        ),
        no_promise_executor_return_allow_void=json_bool(
            json_field(object_, "noPromiseExecutorReturnAllowVoid")
        ),
        no_misused_promises_check_callbacks=json_bool(
            json_field(object_, "noMisusedPromisesCheckCallbacks")
        ),
        no_misused_promises_check_conditionals=json_bool(
            json_field(object_, "noMisusedPromisesCheckConditionals")
        ),
        no_misused_promises_check_spreads=json_bool(
            json_field(object_, "noMisusedPromisesCheckSpreads")
        ),
        use_isnan_enforce_for_switch_case=json_bool(
            json_field(object_, "useIsnanEnforceForSwitchCase")
        ),
        use_isnan_enforce_for_index_of=json_bool(
            json_field(object_, "useIsnanEnforceForIndexOf")
        ),
        no_self_assign_check_properties=json_bool(
            json_field(object_, "noSelfAssignCheckProperties")
        ),
        no_confusing_void_expression_ignore_void_operator=json_bool(
            json_field(object_, "noConfusingVoidExpressionIgnoreVoidOperator")
        ),
        no_confusing_void_expression_ignore_void_returning_functions=json_bool(
            json_field(object_, "noConfusingVoidExpressionIgnoreVoidReturningFunctions")
        ),
        no_cond_assign_mode=destack._generated.repository.config.linter.core.from_json_condition_assignment_mode(
            json_field(object_, "noCondAssignMode")
        ),
        no_empty_allow_empty_catch=json_bool(
            json_field(object_, "noEmptyAllowEmptyCatch")
        ),
        no_empty_pattern_allow_object_patterns_as_parameters=json_bool(
            json_field(object_, "noEmptyPatternAllowObjectPatternsAsParameters")
        ),
        no_empty_function_allow=[
            destack._generated.repository.config.linter.core.from_json_empty_function_kind(
                item_0
            )
            for item_0 in json_array(json_field(object_, "noEmptyFunctionAllow"))
        ],
        no_fallthrough_allow_empty_case=json_bool(
            json_field(object_, "noFallthroughAllowEmptyCase")
        ),
        no_fallthrough_comment_pattern=json_optional(
            object_, "noFallthroughCommentPattern", lambda value: json_string(value)
        ),
        no_fallthrough_report_unused_comment=json_bool(
            json_field(object_, "noFallthroughReportUnusedComment")
        ),
        return_await_mode=destack._generated.repository.config.linter.core.from_json_return_await_mode(
            json_field(object_, "returnAwaitMode")
        ),
        ignored_unused_parameter_prefixes=[
            json_string(item_0)
            for item_0 in json_array(
                json_field(object_, "ignoredUnusedParameterPrefixes")
            )
        ],
        no_useless_rename_ignore_destructuring=json_bool(
            json_field(object_, "noUselessRenameIgnoreDestructuring")
        ),
        no_useless_rename_ignore_import=json_bool(
            json_field(object_, "noUselessRenameIgnoreImport")
        ),
        no_useless_rename_ignore_export=json_bool(
            json_field(object_, "noUselessRenameIgnoreExport")
        ),
        no_useless_escape_allow_regex_characters=[
            json_string(item_0)
            for item_0 in json_array(
                json_field(object_, "noUselessEscapeAllowRegexCharacters")
            )
        ],
    )


__all__ = [
    "LinterCorrectnessOptions",
    "encode_linter_correctness_options",
    "decode_linter_correctness_options",
    "to_json_linter_correctness_options",
    "from_json_linter_correctness_options",
]
