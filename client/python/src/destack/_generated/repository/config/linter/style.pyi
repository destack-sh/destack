# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.repository.config.linter.core

@dataclass(frozen=True, slots=True)
class LinterStyleOptions:
    """Style-category linter options."""

    # preferred array type syntax
    array_type: destack._generated.repository.config.linter.core.ArrayTypeStyle
    # required filename case style
    filename_case: destack._generated.repository.config.linter.core.FilenameCase
    # allowed uppercase keyword prefixes for inline comments
    comment_keywords: Sequence[str]
    # allowed tags for keyword comments
    comment_keyword_tags: Sequence[str]
    # minimum non empty lines required for separator heading comments
    comment_separator_heading_min_lines: int
    # allow named callbacks in `prefer-arrow-callback`
    prefer_arrow_callback_allow_named_functions: bool
    # allow unbound `this` in `prefer-arrow-callback`
    prefer_arrow_callback_allow_unbound_this: bool
    # allow `else if` chains in `no-else-return`
    no_else_return_allow_else_if: bool
    # allow keyword property names in `dot-notation`
    dot_notation_allow_keywords: bool
    # regex pattern of property names exempt from `dot-notation`
    dot_notation_allow_pattern: str | None
    # check nested boolean contexts in `no-extra-boolean-cast`
    no_extra_boolean_cast_enforce_for_inner_expressions: bool
    # ordering policy for `grouped-accessor-pairs`
    grouped_accessor_pairs_order: (
        destack._generated.repository.config.linter.core.GroupedAccessorPairsOrder
    )
    # enforce `grouped-accessor-pairs` in type-only member bodies
    grouped_accessor_pairs_enforce_for_types: bool
    # enforcement mode for `operator-assignment`
    operator_assignment_mode: (
        destack._generated.repository.config.linter.core.OperatorAssignmentMode
    )
    # enforcement mode for `object-shorthand`
    object_shorthand_mode: (
        destack._generated.repository.config.linter.core.ObjectShorthandMode
    )
    # allow quoted keys to stay longform in `object-shorthand`
    object_shorthand_avoid_quotes: bool
    # ignore constructor-like names in `object-shorthand`
    object_shorthand_ignore_constructors: bool
    # regex exemption for method names in `object-shorthand`
    object_shorthand_methods_ignore_pattern: str | None
    # avoid shorthand for explicit return arrow values in `object-shorthand`
    object_shorthand_avoid_explicit_return_arrows: bool
    # keep default-assignment ternaries in `no-unneeded-ternary`
    no_unneeded_ternary_default_assignment: bool
    # destructuring reporting policy for `prefer-const`
    prefer_const_destructuring: (
        destack._generated.repository.config.linter.core.PreferConstDestructuring
    )
    # ignore read-before-assign bindings in `prefer-const`
    prefer_const_ignore_read_before_assign: bool
    # enforcement mode for `yoda`
    yoda_mode: destack._generated.repository.config.linter.core.YodaMode
    # allow Yoda range tests in `yoda`
    yoda_except_range: bool
    # restrict `yoda` to equality operators
    yoda_only_equality: bool
    # ignore case in `sort-imports`
    sort_imports_ignore_case: bool
    # ignore declaration ordering in `sort-imports`
    sort_imports_ignore_declaration_sort: bool
    # ignore member ordering in `sort-imports`
    sort_imports_ignore_member_sort: bool
    # allow separated declaration groups in `sort-imports`
    sort_imports_allow_separated_groups: bool
    # member syntax ordering in `sort-imports`
    sort_imports_member_syntax_sort_order: Sequence[
        destack._generated.repository.config.linter.core.SortImportsMemberSyntax
    ]
    # ignore conditional test positions in `prefer-nullish-coalescing`
    prefer_nullish_coalescing_ignore_conditional_tests: bool
    # ignore mixed logical expressions in `prefer-nullish-coalescing`
    prefer_nullish_coalescing_ignore_mixed_logical_expressions: bool
    # ignore ternary checks in `prefer-nullish-coalescing`
    prefer_nullish_coalescing_ignore_ternary_tests: bool

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> LinterStyleOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> LinterStyleOptions: ...

def encode_linter_style_options(
    writer: BinaryWriter, value: LinterStyleOptions
) -> None: ...
def decode_linter_style_options(reader: BinaryReader) -> LinterStyleOptions: ...
def to_json_linter_style_options(value: LinterStyleOptions) -> Json: ...
def from_json_linter_style_options(value: Json) -> LinterStyleOptions: ...

__all__ = [
    "LinterStyleOptions",
    "encode_linter_style_options",
    "decode_linter_style_options",
    "to_json_linter_style_options",
    "from_json_linter_style_options",
]
