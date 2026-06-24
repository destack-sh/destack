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
    json_int,
    json_object,
    json_optional,
    json_string,
)

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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_linter_style_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> LinterStyleOptions:
        """Decode one LinterStyleOptions."""
        return decode_linter_style_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_linter_style_options(self)

    @classmethod
    def from_json(cls, value: Json) -> LinterStyleOptions:
        """Return one LinterStyleOptions from one JSON value."""
        return from_json_linter_style_options(value)


def encode_linter_style_options(
    writer: BinaryWriter, value: LinterStyleOptions
) -> None:
    """Encode one LinterStyleOptions."""
    destack._generated.repository.config.linter.core.encode_array_type_style(
        writer, value.array_type
    )
    destack._generated.repository.config.linter.core.encode_filename_case(
        writer, value.filename_case
    )
    writer.write_unsigned(len(value.comment_keywords))
    for item_value_comment_keywords_0 in value.comment_keywords:
        writer.write_string(item_value_comment_keywords_0)
    writer.write_unsigned(len(value.comment_keyword_tags))
    for item_value_comment_keyword_tags_0 in value.comment_keyword_tags:
        writer.write_string(item_value_comment_keyword_tags_0)
    writer.write_unsigned(value.comment_separator_heading_min_lines)
    writer.write_bool(value.prefer_arrow_callback_allow_named_functions)
    writer.write_bool(value.prefer_arrow_callback_allow_unbound_this)
    writer.write_bool(value.no_else_return_allow_else_if)
    writer.write_bool(value.dot_notation_allow_keywords)
    if value.dot_notation_allow_pattern is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.dot_notation_allow_pattern)
    writer.write_bool(value.no_extra_boolean_cast_enforce_for_inner_expressions)
    destack._generated.repository.config.linter.core.encode_grouped_accessor_pairs_order(
        writer, value.grouped_accessor_pairs_order
    )
    writer.write_bool(value.grouped_accessor_pairs_enforce_for_types)
    destack._generated.repository.config.linter.core.encode_operator_assignment_mode(
        writer, value.operator_assignment_mode
    )
    destack._generated.repository.config.linter.core.encode_object_shorthand_mode(
        writer, value.object_shorthand_mode
    )
    writer.write_bool(value.object_shorthand_avoid_quotes)
    writer.write_bool(value.object_shorthand_ignore_constructors)
    if value.object_shorthand_methods_ignore_pattern is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.object_shorthand_methods_ignore_pattern)
    writer.write_bool(value.object_shorthand_avoid_explicit_return_arrows)
    writer.write_bool(value.no_unneeded_ternary_default_assignment)
    destack._generated.repository.config.linter.core.encode_prefer_const_destructuring(
        writer, value.prefer_const_destructuring
    )
    writer.write_bool(value.prefer_const_ignore_read_before_assign)
    destack._generated.repository.config.linter.core.encode_yoda_mode(
        writer, value.yoda_mode
    )
    writer.write_bool(value.yoda_except_range)
    writer.write_bool(value.yoda_only_equality)
    writer.write_bool(value.sort_imports_ignore_case)
    writer.write_bool(value.sort_imports_ignore_declaration_sort)
    writer.write_bool(value.sort_imports_ignore_member_sort)
    writer.write_bool(value.sort_imports_allow_separated_groups)
    writer.write_unsigned(len(value.sort_imports_member_syntax_sort_order))
    for (
        item_value_sort_imports_member_syntax_sort_order_0
    ) in value.sort_imports_member_syntax_sort_order:
        destack._generated.repository.config.linter.core.encode_sort_imports_member_syntax(
            writer, item_value_sort_imports_member_syntax_sort_order_0
        )
    writer.write_bool(value.prefer_nullish_coalescing_ignore_conditional_tests)
    writer.write_bool(value.prefer_nullish_coalescing_ignore_mixed_logical_expressions)
    writer.write_bool(value.prefer_nullish_coalescing_ignore_ternary_tests)


def decode_linter_style_options(reader: BinaryReader) -> LinterStyleOptions:
    """Decode one LinterStyleOptions."""
    array_type = (
        destack._generated.repository.config.linter.core.decode_array_type_style(reader)
    )
    filename_case = (
        destack._generated.repository.config.linter.core.decode_filename_case(reader)
    )
    comment_keywords = [reader.read_string() for _ in range(reader.read_number())]
    comment_keyword_tags = [reader.read_string() for _ in range(reader.read_number())]
    comment_separator_heading_min_lines = reader.read_number()
    prefer_arrow_callback_allow_named_functions = reader.read_bool()
    prefer_arrow_callback_allow_unbound_this = reader.read_bool()
    no_else_return_allow_else_if = reader.read_bool()
    dot_notation_allow_keywords = reader.read_bool()
    dot_notation_allow_pattern = reader.read_option(lambda: reader.read_string())
    no_extra_boolean_cast_enforce_for_inner_expressions = reader.read_bool()
    grouped_accessor_pairs_order = destack._generated.repository.config.linter.core.decode_grouped_accessor_pairs_order(
        reader
    )
    grouped_accessor_pairs_enforce_for_types = reader.read_bool()
    operator_assignment_mode = destack._generated.repository.config.linter.core.decode_operator_assignment_mode(
        reader
    )
    object_shorthand_mode = (
        destack._generated.repository.config.linter.core.decode_object_shorthand_mode(
            reader
        )
    )
    object_shorthand_avoid_quotes = reader.read_bool()
    object_shorthand_ignore_constructors = reader.read_bool()
    object_shorthand_methods_ignore_pattern = reader.read_option(
        lambda: reader.read_string()
    )
    object_shorthand_avoid_explicit_return_arrows = reader.read_bool()
    no_unneeded_ternary_default_assignment = reader.read_bool()
    prefer_const_destructuring = destack._generated.repository.config.linter.core.decode_prefer_const_destructuring(
        reader
    )
    prefer_const_ignore_read_before_assign = reader.read_bool()
    yoda_mode = destack._generated.repository.config.linter.core.decode_yoda_mode(
        reader
    )
    yoda_except_range = reader.read_bool()
    yoda_only_equality = reader.read_bool()
    sort_imports_ignore_case = reader.read_bool()
    sort_imports_ignore_declaration_sort = reader.read_bool()
    sort_imports_ignore_member_sort = reader.read_bool()
    sort_imports_allow_separated_groups = reader.read_bool()
    sort_imports_member_syntax_sort_order = [
        destack._generated.repository.config.linter.core.decode_sort_imports_member_syntax(
            reader
        )
        for _ in range(reader.read_number())
    ]
    prefer_nullish_coalescing_ignore_conditional_tests = reader.read_bool()
    prefer_nullish_coalescing_ignore_mixed_logical_expressions = reader.read_bool()
    prefer_nullish_coalescing_ignore_ternary_tests = reader.read_bool()

    return LinterStyleOptions(
        array_type=array_type,
        filename_case=filename_case,
        comment_keywords=comment_keywords,
        comment_keyword_tags=comment_keyword_tags,
        comment_separator_heading_min_lines=comment_separator_heading_min_lines,
        prefer_arrow_callback_allow_named_functions=prefer_arrow_callback_allow_named_functions,
        prefer_arrow_callback_allow_unbound_this=prefer_arrow_callback_allow_unbound_this,
        no_else_return_allow_else_if=no_else_return_allow_else_if,
        dot_notation_allow_keywords=dot_notation_allow_keywords,
        dot_notation_allow_pattern=dot_notation_allow_pattern,
        no_extra_boolean_cast_enforce_for_inner_expressions=no_extra_boolean_cast_enforce_for_inner_expressions,
        grouped_accessor_pairs_order=grouped_accessor_pairs_order,
        grouped_accessor_pairs_enforce_for_types=grouped_accessor_pairs_enforce_for_types,
        operator_assignment_mode=operator_assignment_mode,
        object_shorthand_mode=object_shorthand_mode,
        object_shorthand_avoid_quotes=object_shorthand_avoid_quotes,
        object_shorthand_ignore_constructors=object_shorthand_ignore_constructors,
        object_shorthand_methods_ignore_pattern=object_shorthand_methods_ignore_pattern,
        object_shorthand_avoid_explicit_return_arrows=object_shorthand_avoid_explicit_return_arrows,
        no_unneeded_ternary_default_assignment=no_unneeded_ternary_default_assignment,
        prefer_const_destructuring=prefer_const_destructuring,
        prefer_const_ignore_read_before_assign=prefer_const_ignore_read_before_assign,
        yoda_mode=yoda_mode,
        yoda_except_range=yoda_except_range,
        yoda_only_equality=yoda_only_equality,
        sort_imports_ignore_case=sort_imports_ignore_case,
        sort_imports_ignore_declaration_sort=sort_imports_ignore_declaration_sort,
        sort_imports_ignore_member_sort=sort_imports_ignore_member_sort,
        sort_imports_allow_separated_groups=sort_imports_allow_separated_groups,
        sort_imports_member_syntax_sort_order=sort_imports_member_syntax_sort_order,
        prefer_nullish_coalescing_ignore_conditional_tests=prefer_nullish_coalescing_ignore_conditional_tests,
        prefer_nullish_coalescing_ignore_mixed_logical_expressions=prefer_nullish_coalescing_ignore_mixed_logical_expressions,
        prefer_nullish_coalescing_ignore_ternary_tests=prefer_nullish_coalescing_ignore_ternary_tests,
    )


def to_json_linter_style_options(value: LinterStyleOptions) -> Json:
    """Return one JSON value for one LinterStyleOptions."""
    return {
        "arrayType": destack._generated.repository.config.linter.core.to_json_array_type_style(
            value.array_type
        ),
        "filenameCase": destack._generated.repository.config.linter.core.to_json_filename_case(
            value.filename_case
        ),
        "commentKeywords": [item_0 for item_0 in value.comment_keywords],
        "commentKeywordTags": [item_0 for item_0 in value.comment_keyword_tags],
        "commentSeparatorHeadingMinLines": value.comment_separator_heading_min_lines,
        "preferArrowCallbackAllowNamedFunctions": value.prefer_arrow_callback_allow_named_functions,
        "preferArrowCallbackAllowUnboundThis": value.prefer_arrow_callback_allow_unbound_this,
        "noElseReturnAllowElseIf": value.no_else_return_allow_else_if,
        "dotNotationAllowKeywords": value.dot_notation_allow_keywords,
        **(
            {}
            if value.dot_notation_allow_pattern is None
            else {"dotNotationAllowPattern": value.dot_notation_allow_pattern}
        ),
        "noExtraBooleanCastEnforceForInnerExpressions": value.no_extra_boolean_cast_enforce_for_inner_expressions,
        "groupedAccessorPairsOrder": destack._generated.repository.config.linter.core.to_json_grouped_accessor_pairs_order(
            value.grouped_accessor_pairs_order
        ),
        "groupedAccessorPairsEnforceForTypes": value.grouped_accessor_pairs_enforce_for_types,
        "operatorAssignmentMode": destack._generated.repository.config.linter.core.to_json_operator_assignment_mode(
            value.operator_assignment_mode
        ),
        "objectShorthandMode": destack._generated.repository.config.linter.core.to_json_object_shorthand_mode(
            value.object_shorthand_mode
        ),
        "objectShorthandAvoidQuotes": value.object_shorthand_avoid_quotes,
        "objectShorthandIgnoreConstructors": value.object_shorthand_ignore_constructors,
        **(
            {}
            if value.object_shorthand_methods_ignore_pattern is None
            else {
                "objectShorthandMethodsIgnorePattern": value.object_shorthand_methods_ignore_pattern
            }
        ),
        "objectShorthandAvoidExplicitReturnArrows": value.object_shorthand_avoid_explicit_return_arrows,
        "noUnneededTernaryDefaultAssignment": value.no_unneeded_ternary_default_assignment,
        "preferConstDestructuring": destack._generated.repository.config.linter.core.to_json_prefer_const_destructuring(
            value.prefer_const_destructuring
        ),
        "preferConstIgnoreReadBeforeAssign": value.prefer_const_ignore_read_before_assign,
        "yodaMode": destack._generated.repository.config.linter.core.to_json_yoda_mode(
            value.yoda_mode
        ),
        "yodaExceptRange": value.yoda_except_range,
        "yodaOnlyEquality": value.yoda_only_equality,
        "sortImportsIgnoreCase": value.sort_imports_ignore_case,
        "sortImportsIgnoreDeclarationSort": value.sort_imports_ignore_declaration_sort,
        "sortImportsIgnoreMemberSort": value.sort_imports_ignore_member_sort,
        "sortImportsAllowSeparatedGroups": value.sort_imports_allow_separated_groups,
        "sortImportsMemberSyntaxSortOrder": [
            destack._generated.repository.config.linter.core.to_json_sort_imports_member_syntax(
                item_0
            )
            for item_0 in value.sort_imports_member_syntax_sort_order
        ],
        "preferNullishCoalescingIgnoreConditionalTests": value.prefer_nullish_coalescing_ignore_conditional_tests,
        "preferNullishCoalescingIgnoreMixedLogicalExpressions": value.prefer_nullish_coalescing_ignore_mixed_logical_expressions,
        "preferNullishCoalescingIgnoreTernaryTests": value.prefer_nullish_coalescing_ignore_ternary_tests,
    }


def from_json_linter_style_options(value: Json) -> LinterStyleOptions:
    """Return one LinterStyleOptions from one JSON value."""
    object_ = json_object(value)

    return LinterStyleOptions(
        array_type=destack._generated.repository.config.linter.core.from_json_array_type_style(
            json_field(object_, "arrayType")
        ),
        filename_case=destack._generated.repository.config.linter.core.from_json_filename_case(
            json_field(object_, "filenameCase")
        ),
        comment_keywords=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "commentKeywords"))
        ],
        comment_keyword_tags=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "commentKeywordTags"))
        ],
        comment_separator_heading_min_lines=json_int(
            json_field(object_, "commentSeparatorHeadingMinLines")
        ),
        prefer_arrow_callback_allow_named_functions=json_bool(
            json_field(object_, "preferArrowCallbackAllowNamedFunctions")
        ),
        prefer_arrow_callback_allow_unbound_this=json_bool(
            json_field(object_, "preferArrowCallbackAllowUnboundThis")
        ),
        no_else_return_allow_else_if=json_bool(
            json_field(object_, "noElseReturnAllowElseIf")
        ),
        dot_notation_allow_keywords=json_bool(
            json_field(object_, "dotNotationAllowKeywords")
        ),
        dot_notation_allow_pattern=json_optional(
            object_, "dotNotationAllowPattern", lambda value: json_string(value)
        ),
        no_extra_boolean_cast_enforce_for_inner_expressions=json_bool(
            json_field(object_, "noExtraBooleanCastEnforceForInnerExpressions")
        ),
        grouped_accessor_pairs_order=destack._generated.repository.config.linter.core.from_json_grouped_accessor_pairs_order(
            json_field(object_, "groupedAccessorPairsOrder")
        ),
        grouped_accessor_pairs_enforce_for_types=json_bool(
            json_field(object_, "groupedAccessorPairsEnforceForTypes")
        ),
        operator_assignment_mode=destack._generated.repository.config.linter.core.from_json_operator_assignment_mode(
            json_field(object_, "operatorAssignmentMode")
        ),
        object_shorthand_mode=destack._generated.repository.config.linter.core.from_json_object_shorthand_mode(
            json_field(object_, "objectShorthandMode")
        ),
        object_shorthand_avoid_quotes=json_bool(
            json_field(object_, "objectShorthandAvoidQuotes")
        ),
        object_shorthand_ignore_constructors=json_bool(
            json_field(object_, "objectShorthandIgnoreConstructors")
        ),
        object_shorthand_methods_ignore_pattern=json_optional(
            object_,
            "objectShorthandMethodsIgnorePattern",
            lambda value: json_string(value),
        ),
        object_shorthand_avoid_explicit_return_arrows=json_bool(
            json_field(object_, "objectShorthandAvoidExplicitReturnArrows")
        ),
        no_unneeded_ternary_default_assignment=json_bool(
            json_field(object_, "noUnneededTernaryDefaultAssignment")
        ),
        prefer_const_destructuring=destack._generated.repository.config.linter.core.from_json_prefer_const_destructuring(
            json_field(object_, "preferConstDestructuring")
        ),
        prefer_const_ignore_read_before_assign=json_bool(
            json_field(object_, "preferConstIgnoreReadBeforeAssign")
        ),
        yoda_mode=destack._generated.repository.config.linter.core.from_json_yoda_mode(
            json_field(object_, "yodaMode")
        ),
        yoda_except_range=json_bool(json_field(object_, "yodaExceptRange")),
        yoda_only_equality=json_bool(json_field(object_, "yodaOnlyEquality")),
        sort_imports_ignore_case=json_bool(
            json_field(object_, "sortImportsIgnoreCase")
        ),
        sort_imports_ignore_declaration_sort=json_bool(
            json_field(object_, "sortImportsIgnoreDeclarationSort")
        ),
        sort_imports_ignore_member_sort=json_bool(
            json_field(object_, "sortImportsIgnoreMemberSort")
        ),
        sort_imports_allow_separated_groups=json_bool(
            json_field(object_, "sortImportsAllowSeparatedGroups")
        ),
        sort_imports_member_syntax_sort_order=[
            destack._generated.repository.config.linter.core.from_json_sort_imports_member_syntax(
                item_0
            )
            for item_0 in json_array(
                json_field(object_, "sortImportsMemberSyntaxSortOrder")
            )
        ],
        prefer_nullish_coalescing_ignore_conditional_tests=json_bool(
            json_field(object_, "preferNullishCoalescingIgnoreConditionalTests")
        ),
        prefer_nullish_coalescing_ignore_mixed_logical_expressions=json_bool(
            json_field(object_, "preferNullishCoalescingIgnoreMixedLogicalExpressions")
        ),
        prefer_nullish_coalescing_ignore_ternary_tests=json_bool(
            json_field(object_, "preferNullishCoalescingIgnoreTernaryTests")
        ),
    )


__all__ = [
    "LinterStyleOptions",
    "encode_linter_style_options",
    "decode_linter_style_options",
    "to_json_linter_style_options",
    "from_json_linter_style_options",
]
