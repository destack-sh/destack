# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence

from destack.source.component import (
    ComponentId,
)

from destack.source.module import (
    ModuleId,
)

from destack.source.package import (
    PackageId,
)

from destack.source.profile import (
    ProfileId,
)

from destack.source.target import (
    TargetId,
)

class ArtifactKey:
    """External artifact key crossing bridge boundaries."""

    """Parsed module DIR."""
    @staticmethod
    def dir_parsed(module: ModuleId) -> ArtifactKey: ...

    """Parsed non-code module data."""
    @staticmethod
    def data(module: ModuleId) -> ArtifactKey: ...

    """Explicit global environment for one profile."""
    @staticmethod
    def global_environment(profile: ProfileId) -> ArtifactKey: ...

    """Active dependency index for one profile."""
    @staticmethod
    def package_index(profile: ProfileId) -> ArtifactKey: ...

    """Module import edge index for one profile."""
    @staticmethod
    def module_index(profile: ProfileId) -> ArtifactKey: ...

    """Component partition for one profile."""
    @staticmethod
    def component_graph(profile: ProfileId) -> ArtifactKey: ...

    """Bound DIR."""
    @staticmethod
    def dir_bound(module: ModuleId, profile: ProfileId) -> ArtifactKey: ...

    """Imported DIR."""
    @staticmethod
    def dir_imported(module: ModuleId, profile: ProfileId) -> ArtifactKey: ...

    """Expanded DIR."""
    @staticmethod
    def dir_expanded(module: ModuleId, profile: ProfileId) -> ArtifactKey: ...

    """Exported DIR."""
    @staticmethod
    def dir_exported(module: ModuleId, profile: ProfileId) -> ArtifactKey: ...

    """Resolved DIR imports."""
    @staticmethod
    def dir_resolved(module: ModuleId, profile: ProfileId) -> ArtifactKey: ...

    """Checked DIR component."""
    @staticmethod
    def dir_checked_component(entry: ModuleId, component: ComponentId, profile: ProfileId) -> ArtifactKey: ...

    """Checked DIR facade."""
    @staticmethod
    def dir_checked(module: ModuleId, profile: ProfileId) -> ArtifactKey: ...

    """Materialized DIR."""
    @staticmethod
    def dir_materialized(module: ModuleId, profile: ProfileId) -> ArtifactKey: ...

    """Elaborated DIR."""
    @staticmethod
    def dir_elaborated(module: ModuleId, profile: ProfileId) -> ArtifactKey: ...

    """Lowered MIR before optimization."""
    @staticmethod
    def mir_lowered(module: ModuleId, profile: ProfileId, target: TargetId) -> ArtifactKey: ...

    """Verified MIR after required semantic verification."""
    @staticmethod
    def mir_verified(module: ModuleId, profile: ProfileId, target: TargetId) -> ArtifactKey: ...

    """Optimized MIR."""
    @staticmethod
    def mir_optimized(module: ModuleId, profile: ProfileId, target: TargetId) -> ArtifactKey: ...

    """Query index for one module profile."""
    @staticmethod
    def module_query_index(module: ModuleId, profile: ProfileId) -> ArtifactKey: ...

    """Query index for one workspace profile."""
    @staticmethod
    def workspace_query_index(profile: ProfileId) -> ArtifactKey: ...

    """One generated module output for one target."""
    @staticmethod
    def module_output(module: ModuleId, target: TargetId) -> ArtifactKey: ...

    """Output entries for one package target."""
    @staticmethod
    def package_output(package: PackageId, target: TargetId) -> ArtifactKey: ...

    """Realized lint diagnostics for one module profile."""
    @staticmethod
    def module_linted(module: ModuleId, profile: ProfileId) -> ArtifactKey: ...

    """Realized lint diagnostics for one package."""
    @staticmethod
    def package_linted(package: PackageId) -> ArtifactKey: ...

    """Realized lint diagnostics for the workspace."""
    @staticmethod
    def workspace_linted() -> ArtifactKey: ...

    @property
    def kind(self) -> str: ...

    @property
    def component(self) -> ComponentId | None: ...

    @property
    def entry(self) -> ModuleId | None: ...

    @property
    def module(self) -> ModuleId | None: ...

    @property
    def package(self) -> PackageId | None: ...

    @property
    def profile(self) -> ProfileId | None: ...

    @property
    def target(self) -> TargetId | None: ...

