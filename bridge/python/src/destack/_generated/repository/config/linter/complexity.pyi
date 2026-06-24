# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> LinterComplexityOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> LinterComplexityOptions: ...

def encode_linter_complexity_options(
    writer: BinaryWriter, value: LinterComplexityOptions
) -> None: ...
def decode_linter_complexity_options(
    reader: BinaryReader,
) -> LinterComplexityOptions: ...
def to_json_linter_complexity_options(value: LinterComplexityOptions) -> Json: ...
def from_json_linter_complexity_options(value: Json) -> LinterComplexityOptions: ...

__all__ = [
    "LinterComplexityOptions",
    "encode_linter_complexity_options",
    "decode_linter_complexity_options",
    "to_json_linter_complexity_options",
    "from_json_linter_complexity_options",
]
