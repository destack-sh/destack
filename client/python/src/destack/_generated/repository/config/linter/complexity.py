# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_bool,
    json_field,
    json_int,
    json_object,
)

import destack._generated.repository.config.linter.core


@dataclass(frozen=True, slots=True)
class LinterComplexityOptions:
    """Complexity-category linter options."""

    # maximum boolean parameters or fields
    max_booleans: int
    # maximum branches in a single conditional (if/match)
    max_branching_factor: int
    # maximum cognitive complexity
    max_cognitive_complexity: int
    # maximum cyclomatic complexity
    max_cyclomatic_complexity: int
    # switch counting variant for `cyclomatic-complexity`
    cyclomatic_complexity_variant: (
        destack._generated.repository.config.linter.core.CyclomaticComplexityVariant
    )
    # maximum nesting depth
    max_depth: int
    # maximum generic parameters (generics including const values)
    max_generic_params: int
    # maximum lines per file
    max_lines: int
    # ignore full-line comments in `max-lines`
    max_lines_skip_comments: bool
    # ignore blank lines in `max-lines`
    max_lines_skip_blank_lines: bool
    # maximum lines per function
    max_lines_per_function: int
    # ignore full-line comments in `max-lines-per-function`
    max_lines_per_function_skip_comments: bool
    # ignore blank lines in `max-lines-per-function`
    max_lines_per_function_skip_blank_lines: bool
    # count immediately invoked functions in `max-lines-per-function`
    max_lines_per_function_iifes: bool
    # maximum callback nesting
    max_nested_callbacks: int
    # maximum function parameters
    max_params: int
    # count `this` parameters in `max-params`
    max_params_count_this: (
        destack._generated.repository.config.linter.core.MaxParamsCountThis
    )
    # maximum statements per function
    max_statements: int
    # ignore top-level functions in `max-statements`
    max_statements_ignore_top_level_functions: bool
    # maximum return statements per function
    max_return_statements: int
    # maximum switch cases per switch statement
    max_switch_cases: int
    # maximum variants in a union type or enum
    max_type_variants: int
    # maximum fields in a struct, class, or interface
    max_type_fields: int
    # maximum type complexity
    max_type_complexity: int
    # maximum occurrences of the same string literal before warning
    max_duplicate_string_occurrences: int
    # minimum lines required to consider a block for duplicate code checks
    min_duplicate_code_lines: int
    # minimum tokens required to consider a block for duplicate code checks
    min_duplicate_code_tokens: int
    # minimum similarity percent for near duplicate code matching
    min_duplicate_code_near_similarity: int
    # maximum statements in a try block
    max_try_block_statements: int
    # ignore non-declaration chains in `no-multi-assign`
    no_multi_assign_ignore_non_declaration: bool
    # allow short-circuit expressions in `no-unused-expressions`
    no_unused_expressions_allow_short_circuit: bool
    # allow ternary expressions in `no-unused-expressions`
    no_unused_expressions_allow_ternary: bool
    # allow tagged templates in `no-unused-expressions`
    no_unused_expressions_allow_tagged_templates: bool
    # enforce JSX-like tree expressions in `no-unused-expressions`
    no_unused_expressions_enforce_for_jsx: bool
    # ignore directive prologues in `no-unused-expressions`
    no_unused_expressions_ignore_directives: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_linter_complexity_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> LinterComplexityOptions:
        """Decode one LinterComplexityOptions."""
        return decode_linter_complexity_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_linter_complexity_options(self)

    @classmethod
    def from_json(cls, value: Json) -> LinterComplexityOptions:
        """Return one LinterComplexityOptions from one JSON value."""
        return from_json_linter_complexity_options(value)


def encode_linter_complexity_options(
    writer: BinaryWriter, value: LinterComplexityOptions
) -> None:
    """Encode one LinterComplexityOptions."""
    writer.write_unsigned(value.max_booleans)
    writer.write_unsigned(value.max_branching_factor)
    writer.write_unsigned(value.max_cognitive_complexity)
    writer.write_unsigned(value.max_cyclomatic_complexity)
    destack._generated.repository.config.linter.core.encode_cyclomatic_complexity_variant(
        writer, value.cyclomatic_complexity_variant
    )
    writer.write_unsigned(value.max_depth)
    writer.write_unsigned(value.max_generic_params)
    writer.write_unsigned(value.max_lines)
    writer.write_bool(value.max_lines_skip_comments)
    writer.write_bool(value.max_lines_skip_blank_lines)
    writer.write_unsigned(value.max_lines_per_function)
    writer.write_bool(value.max_lines_per_function_skip_comments)
    writer.write_bool(value.max_lines_per_function_skip_blank_lines)
    writer.write_bool(value.max_lines_per_function_iifes)
    writer.write_unsigned(value.max_nested_callbacks)
    writer.write_unsigned(value.max_params)
    destack._generated.repository.config.linter.core.encode_max_params_count_this(
        writer, value.max_params_count_this
    )
    writer.write_unsigned(value.max_statements)
    writer.write_bool(value.max_statements_ignore_top_level_functions)
    writer.write_unsigned(value.max_return_statements)
    writer.write_unsigned(value.max_switch_cases)
    writer.write_unsigned(value.max_type_variants)
    writer.write_unsigned(value.max_type_fields)
    writer.write_unsigned(value.max_type_complexity)
    writer.write_unsigned(value.max_duplicate_string_occurrences)
    writer.write_unsigned(value.min_duplicate_code_lines)
    writer.write_unsigned(value.min_duplicate_code_tokens)
    writer.write_byte(value.min_duplicate_code_near_similarity)
    writer.write_unsigned(value.max_try_block_statements)
    writer.write_bool(value.no_multi_assign_ignore_non_declaration)
    writer.write_bool(value.no_unused_expressions_allow_short_circuit)
    writer.write_bool(value.no_unused_expressions_allow_ternary)
    writer.write_bool(value.no_unused_expressions_allow_tagged_templates)
    writer.write_bool(value.no_unused_expressions_enforce_for_jsx)
    writer.write_bool(value.no_unused_expressions_ignore_directives)


def decode_linter_complexity_options(reader: BinaryReader) -> LinterComplexityOptions:
    """Decode one LinterComplexityOptions."""
    max_booleans = reader.read_number()
    max_branching_factor = reader.read_number()
    max_cognitive_complexity = reader.read_number()
    max_cyclomatic_complexity = reader.read_number()
    cyclomatic_complexity_variant = destack._generated.repository.config.linter.core.decode_cyclomatic_complexity_variant(
        reader
    )
    max_depth = reader.read_number()
    max_generic_params = reader.read_number()
    max_lines = reader.read_number()
    max_lines_skip_comments = reader.read_bool()
    max_lines_skip_blank_lines = reader.read_bool()
    max_lines_per_function = reader.read_number()
    max_lines_per_function_skip_comments = reader.read_bool()
    max_lines_per_function_skip_blank_lines = reader.read_bool()
    max_lines_per_function_iifes = reader.read_bool()
    max_nested_callbacks = reader.read_number()
    max_params = reader.read_number()
    max_params_count_this = (
        destack._generated.repository.config.linter.core.decode_max_params_count_this(
            reader
        )
    )
    max_statements = reader.read_number()
    max_statements_ignore_top_level_functions = reader.read_bool()
    max_return_statements = reader.read_number()
    max_switch_cases = reader.read_number()
    max_type_variants = reader.read_number()
    max_type_fields = reader.read_number()
    max_type_complexity = reader.read_number()
    max_duplicate_string_occurrences = reader.read_number()
    min_duplicate_code_lines = reader.read_number()
    min_duplicate_code_tokens = reader.read_number()
    min_duplicate_code_near_similarity = reader.read_byte()
    max_try_block_statements = reader.read_number()
    no_multi_assign_ignore_non_declaration = reader.read_bool()
    no_unused_expressions_allow_short_circuit = reader.read_bool()
    no_unused_expressions_allow_ternary = reader.read_bool()
    no_unused_expressions_allow_tagged_templates = reader.read_bool()
    no_unused_expressions_enforce_for_jsx = reader.read_bool()
    no_unused_expressions_ignore_directives = reader.read_bool()

    return LinterComplexityOptions(
        max_booleans=max_booleans,
        max_branching_factor=max_branching_factor,
        max_cognitive_complexity=max_cognitive_complexity,
        max_cyclomatic_complexity=max_cyclomatic_complexity,
        cyclomatic_complexity_variant=cyclomatic_complexity_variant,
        max_depth=max_depth,
        max_generic_params=max_generic_params,
        max_lines=max_lines,
        max_lines_skip_comments=max_lines_skip_comments,
        max_lines_skip_blank_lines=max_lines_skip_blank_lines,
        max_lines_per_function=max_lines_per_function,
        max_lines_per_function_skip_comments=max_lines_per_function_skip_comments,
        max_lines_per_function_skip_blank_lines=max_lines_per_function_skip_blank_lines,
        max_lines_per_function_iifes=max_lines_per_function_iifes,
        max_nested_callbacks=max_nested_callbacks,
        max_params=max_params,
        max_params_count_this=max_params_count_this,
        max_statements=max_statements,
        max_statements_ignore_top_level_functions=max_statements_ignore_top_level_functions,
        max_return_statements=max_return_statements,
        max_switch_cases=max_switch_cases,
        max_type_variants=max_type_variants,
        max_type_fields=max_type_fields,
        max_type_complexity=max_type_complexity,
        max_duplicate_string_occurrences=max_duplicate_string_occurrences,
        min_duplicate_code_lines=min_duplicate_code_lines,
        min_duplicate_code_tokens=min_duplicate_code_tokens,
        min_duplicate_code_near_similarity=min_duplicate_code_near_similarity,
        max_try_block_statements=max_try_block_statements,
        no_multi_assign_ignore_non_declaration=no_multi_assign_ignore_non_declaration,
        no_unused_expressions_allow_short_circuit=no_unused_expressions_allow_short_circuit,
        no_unused_expressions_allow_ternary=no_unused_expressions_allow_ternary,
        no_unused_expressions_allow_tagged_templates=no_unused_expressions_allow_tagged_templates,
        no_unused_expressions_enforce_for_jsx=no_unused_expressions_enforce_for_jsx,
        no_unused_expressions_ignore_directives=no_unused_expressions_ignore_directives,
    )


def to_json_linter_complexity_options(value: LinterComplexityOptions) -> Json:
    """Return one JSON value for one LinterComplexityOptions."""
    return {
        "maxBooleans": value.max_booleans,
        "maxBranchingFactor": value.max_branching_factor,
        "maxCognitiveComplexity": value.max_cognitive_complexity,
        "maxCyclomaticComplexity": value.max_cyclomatic_complexity,
        "cyclomaticComplexityVariant": destack._generated.repository.config.linter.core.to_json_cyclomatic_complexity_variant(
            value.cyclomatic_complexity_variant
        ),
        "maxDepth": value.max_depth,
        "maxGenericParams": value.max_generic_params,
        "maxLines": value.max_lines,
        "maxLinesSkipComments": value.max_lines_skip_comments,
        "maxLinesSkipBlankLines": value.max_lines_skip_blank_lines,
        "maxLinesPerFunction": value.max_lines_per_function,
        "maxLinesPerFunctionSkipComments": value.max_lines_per_function_skip_comments,
        "maxLinesPerFunctionSkipBlankLines": value.max_lines_per_function_skip_blank_lines,
        "maxLinesPerFunctionIifes": value.max_lines_per_function_iifes,
        "maxNestedCallbacks": value.max_nested_callbacks,
        "maxParams": value.max_params,
        "maxParamsCountThis": destack._generated.repository.config.linter.core.to_json_max_params_count_this(
            value.max_params_count_this
        ),
        "maxStatements": value.max_statements,
        "maxStatementsIgnoreTopLevelFunctions": value.max_statements_ignore_top_level_functions,
        "maxReturnStatements": value.max_return_statements,
        "maxSwitchCases": value.max_switch_cases,
        "maxTypeVariants": value.max_type_variants,
        "maxTypeFields": value.max_type_fields,
        "maxTypeComplexity": value.max_type_complexity,
        "maxDuplicateStringOccurrences": value.max_duplicate_string_occurrences,
        "minDuplicateCodeLines": value.min_duplicate_code_lines,
        "minDuplicateCodeTokens": value.min_duplicate_code_tokens,
        "minDuplicateCodeNearSimilarity": value.min_duplicate_code_near_similarity,
        "maxTryBlockStatements": value.max_try_block_statements,
        "noMultiAssignIgnoreNonDeclaration": value.no_multi_assign_ignore_non_declaration,
        "noUnusedExpressionsAllowShortCircuit": value.no_unused_expressions_allow_short_circuit,
        "noUnusedExpressionsAllowTernary": value.no_unused_expressions_allow_ternary,
        "noUnusedExpressionsAllowTaggedTemplates": value.no_unused_expressions_allow_tagged_templates,
        "noUnusedExpressionsEnforceForJsx": value.no_unused_expressions_enforce_for_jsx,
        "noUnusedExpressionsIgnoreDirectives": value.no_unused_expressions_ignore_directives,
    }


def from_json_linter_complexity_options(value: Json) -> LinterComplexityOptions:
    """Return one LinterComplexityOptions from one JSON value."""
    object_ = json_object(value)

    return LinterComplexityOptions(
        max_booleans=json_int(json_field(object_, "maxBooleans")),
        max_branching_factor=json_int(json_field(object_, "maxBranchingFactor")),
        max_cognitive_complexity=json_int(
            json_field(object_, "maxCognitiveComplexity")
        ),
        max_cyclomatic_complexity=json_int(
            json_field(object_, "maxCyclomaticComplexity")
        ),
        cyclomatic_complexity_variant=destack._generated.repository.config.linter.core.from_json_cyclomatic_complexity_variant(
            json_field(object_, "cyclomaticComplexityVariant")
        ),
        max_depth=json_int(json_field(object_, "maxDepth")),
        max_generic_params=json_int(json_field(object_, "maxGenericParams")),
        max_lines=json_int(json_field(object_, "maxLines")),
        max_lines_skip_comments=json_bool(json_field(object_, "maxLinesSkipComments")),
        max_lines_skip_blank_lines=json_bool(
            json_field(object_, "maxLinesSkipBlankLines")
        ),
        max_lines_per_function=json_int(json_field(object_, "maxLinesPerFunction")),
        max_lines_per_function_skip_comments=json_bool(
            json_field(object_, "maxLinesPerFunctionSkipComments")
        ),
        max_lines_per_function_skip_blank_lines=json_bool(
            json_field(object_, "maxLinesPerFunctionSkipBlankLines")
        ),
        max_lines_per_function_iifes=json_bool(
            json_field(object_, "maxLinesPerFunctionIifes")
        ),
        max_nested_callbacks=json_int(json_field(object_, "maxNestedCallbacks")),
        max_params=json_int(json_field(object_, "maxParams")),
        max_params_count_this=destack._generated.repository.config.linter.core.from_json_max_params_count_this(
            json_field(object_, "maxParamsCountThis")
        ),
        max_statements=json_int(json_field(object_, "maxStatements")),
        max_statements_ignore_top_level_functions=json_bool(
            json_field(object_, "maxStatementsIgnoreTopLevelFunctions")
        ),
        max_return_statements=json_int(json_field(object_, "maxReturnStatements")),
        max_switch_cases=json_int(json_field(object_, "maxSwitchCases")),
        max_type_variants=json_int(json_field(object_, "maxTypeVariants")),
        max_type_fields=json_int(json_field(object_, "maxTypeFields")),
        max_type_complexity=json_int(json_field(object_, "maxTypeComplexity")),
        max_duplicate_string_occurrences=json_int(
            json_field(object_, "maxDuplicateStringOccurrences")
        ),
        min_duplicate_code_lines=json_int(json_field(object_, "minDuplicateCodeLines")),
        min_duplicate_code_tokens=json_int(
            json_field(object_, "minDuplicateCodeTokens")
        ),
        min_duplicate_code_near_similarity=json_int(
            json_field(object_, "minDuplicateCodeNearSimilarity")
        ),
        max_try_block_statements=json_int(json_field(object_, "maxTryBlockStatements")),
        no_multi_assign_ignore_non_declaration=json_bool(
            json_field(object_, "noMultiAssignIgnoreNonDeclaration")
        ),
        no_unused_expressions_allow_short_circuit=json_bool(
            json_field(object_, "noUnusedExpressionsAllowShortCircuit")
        ),
        no_unused_expressions_allow_ternary=json_bool(
            json_field(object_, "noUnusedExpressionsAllowTernary")
        ),
        no_unused_expressions_allow_tagged_templates=json_bool(
            json_field(object_, "noUnusedExpressionsAllowTaggedTemplates")
        ),
        no_unused_expressions_enforce_for_jsx=json_bool(
            json_field(object_, "noUnusedExpressionsEnforceForJsx")
        ),
        no_unused_expressions_ignore_directives=json_bool(
            json_field(object_, "noUnusedExpressionsIgnoreDirectives")
        ),
    )


__all__ = [
    "LinterComplexityOptions",
    "encode_linter_complexity_options",
    "decode_linter_complexity_options",
    "to_json_linter_complexity_options",
    "from_json_linter_complexity_options",
]
