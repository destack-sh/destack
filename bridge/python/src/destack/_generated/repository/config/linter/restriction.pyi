# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> LinterRestrictionOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> LinterRestrictionOptions: ...

def encode_linter_restriction_options(
    writer: BinaryWriter, value: LinterRestrictionOptions
) -> None: ...
def decode_linter_restriction_options(
    reader: BinaryReader,
) -> LinterRestrictionOptions: ...
def to_json_linter_restriction_options(value: LinterRestrictionOptions) -> Json: ...
def from_json_linter_restriction_options(value: Json) -> LinterRestrictionOptions: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> LintModuleBoundariesOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> LintModuleBoundariesOptions: ...

def encode_lint_module_boundaries_options(
    writer: BinaryWriter, value: LintModuleBoundariesOptions
) -> None: ...
def decode_lint_module_boundaries_options(
    reader: BinaryReader,
) -> LintModuleBoundariesOptions: ...
def to_json_lint_module_boundaries_options(
    value: LintModuleBoundariesOptions,
) -> Json: ...
def from_json_lint_module_boundaries_options(
    value: Json,
) -> LintModuleBoundariesOptions: ...

@dataclass(frozen=True, slots=True)
class LintModuleComponent:
    """One module component declaration."""

    # the unique component name
    name: str
    # glob patterns used to match module paths into this component
    path_patterns: Sequence[str]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> LintModuleComponent: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> LintModuleComponent: ...

def encode_lint_module_component(
    writer: BinaryWriter, value: LintModuleComponent
) -> None: ...
def decode_lint_module_component(reader: BinaryReader) -> LintModuleComponent: ...
def to_json_lint_module_component(value: LintModuleComponent) -> Json: ...
def from_json_lint_module_component(value: Json) -> LintModuleComponent: ...

@dataclass(frozen=True, slots=True)
class LintModuleDependencyRule:
    """One allowed dependency rule between module components."""

    # the source component name
    from_: str
    # destination components this source component may import
    allow: Sequence[str]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> LintModuleDependencyRule: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> LintModuleDependencyRule: ...

def encode_lint_module_dependency_rule(
    writer: BinaryWriter, value: LintModuleDependencyRule
) -> None: ...
def decode_lint_module_dependency_rule(
    reader: BinaryReader,
) -> LintModuleDependencyRule: ...
def to_json_lint_module_dependency_rule(value: LintModuleDependencyRule) -> Json: ...
def from_json_lint_module_dependency_rule(value: Json) -> LintModuleDependencyRule: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> LintModuleDependencyException: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> LintModuleDependencyException: ...

def encode_lint_module_dependency_exception(
    writer: BinaryWriter, value: LintModuleDependencyException
) -> None: ...
def decode_lint_module_dependency_exception(
    reader: BinaryReader,
) -> LintModuleDependencyException: ...
def to_json_lint_module_dependency_exception(
    value: LintModuleDependencyException,
) -> Json: ...
def from_json_lint_module_dependency_exception(
    value: Json,
) -> LintModuleDependencyException: ...

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
