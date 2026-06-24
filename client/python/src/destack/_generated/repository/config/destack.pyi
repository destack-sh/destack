# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.repository.config.compiler
import destack._generated.repository.config.condition
import destack._generated.repository.config.dependency
import destack._generated.repository.config.export
import destack._generated.repository.config.formatter
import destack._generated.repository.config.linter.core
import destack._generated.repository.config.policy
import destack._generated.repository.config.product.product
import destack._generated.repository.config.profile
import destack._generated.repository.config.runtime.runtime
import destack._generated.repository.config.stage
import destack._generated.repository.config.target.target
import destack._generated.repository.config.task
import destack._generated.repository.config.topology
import destack._generated.repository.config.vendor

@dataclass(frozen=True, slots=True)
class WorkspaceLayout:
    """Workspace package layout."""

    # package root glob patterns
    packages: Sequence[str] | None
    # named groups of package paths
    groups: Mapping[str, Sequence[str]] | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> WorkspaceLayout: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> WorkspaceLayout: ...

def encode_workspace_layout(writer: BinaryWriter, value: WorkspaceLayout) -> None: ...
def decode_workspace_layout(reader: BinaryReader) -> WorkspaceLayout: ...
def to_json_workspace_layout(value: WorkspaceLayout) -> Json: ...
def from_json_workspace_layout(value: Json) -> WorkspaceLayout: ...

@dataclass(frozen=True, slots=True)
class Destack:
    """Destack configuration document."""

    # package name
    name: str | None
    # package version
    version: str | None
    # package release stage
    stage: destack._generated.repository.config.stage.Stage | None
    # whether the package is private
    private: bool | None
    # package description
    description: str | None
    # package license identifier
    license: str | None
    # package repository metadata
    repository: typing.Any | None
    # package homepage
    homepage: str | None
    # package keywords
    keywords: Sequence[str]
    # repository wide workspace package and group configuration
    workspace: WorkspaceLayout | None
    # config path inherited before this config
    extends: str | None
    # specific files to include in the project
    files: Sequence[str]
    # glob patterns for files to include
    include: Sequence[str]
    # glob patterns for files to exclude
    exclude: Sequence[str]
    # public package exports
    exports: Mapping[str, destack._generated.repository.config.export.Export]
    # package dependencies
    dependencies: Mapping[
        str, destack._generated.repository.config.dependency.Dependency
    ]
    # dependencies enabled by condition predicates
    conditional_dependencies: Sequence[
        destack._generated.repository.config.dependency.ConditionalDependencies
    ]
    # package dependency overrides
    overrides: Mapping[str, destack._generated.repository.config.dependency.Dependency]
    # package patch files
    patches: Mapping[str, destack._generated.repository.config.dependency.PackagePatch]
    # vendored dependency resolution declaration
    vendor: destack._generated.repository.config.vendor.Vendor
    # package topology definition
    topology: destack._generated.repository.config.topology.Topology
    # compiler configuration
    compiler: destack._generated.repository.config.compiler.CompilerOptions
    # package policy declarations and rules
    policy: destack._generated.repository.config.policy.Policy
    # runtime configuration
    runtime: destack._generated.repository.config.runtime.runtime.RuntimeOptions
    # formatter configuration
    formatter: destack._generated.repository.config.formatter.FormatterOptions
    # linter configuration
    linter: destack._generated.repository.config.linter.core.LinterOptions
    # build targets
    targets: Mapping[str, destack._generated.repository.config.target.target.Target]
    # deliverable products
    products: Mapping[str, destack._generated.repository.config.product.product.Product]
    # named profiles for semantic configuration
    profiles: Mapping[str, destack._generated.repository.config.profile.ProfileOptions]
    # named source graph conditions
    conditions: destack._generated.repository.config.condition.ConditionCatalog
    # named toolchain and shell tasks
    tasks: Mapping[str, destack._generated.repository.config.task.Task]
    # default target for the package
    default_target: str | None
    # default product for the package
    default_product: str | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Destack: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Destack: ...

def encode_destack(writer: BinaryWriter, value: Destack) -> None: ...
def decode_destack(reader: BinaryReader) -> Destack: ...
def to_json_destack(value: Destack) -> Json: ...
def from_json_destack(value: Json) -> Destack: ...

__all__ = [
    "WorkspaceLayout",
    "encode_workspace_layout",
    "decode_workspace_layout",
    "to_json_workspace_layout",
    "from_json_workspace_layout",
    "Destack",
    "encode_destack",
    "decode_destack",
    "to_json_destack",
    "from_json_destack",
]
