from __future__ import annotations

from datetime import datetime
from typing import TYPE_CHECKING, Optional
from uuid import UUID

import pytz
from asgiref.sync import sync_to_async
from django.core.validators import validate_slug
from django.db import models, transaction
from django.db.models import Q, QuerySet
from django_choices_field import TextChoicesField

from bench.models.symbol import Symbol, SymbolContent, SymbolDefinition, SymbolType
from bench.models.tag import TaggableMixin
from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH, UUIDModel

if TYPE_CHECKING:
    from bench.models.organization import Organization


class ProjectType(models.TextChoices):
    EXECUTABLE = "executable", "Executable"
    LIBRARY = "library", "Library"


class ProjectManager(models.Manager["Project"]):
    @transaction.atomic
    def create_project(
        self,
        organization: Organization,
        name: str,
        slug: str,
        type: ProjectType = ProjectType.EXECUTABLE,
    ):
        project = super().create(organization=organization, name=name, slug=slug, type=type)
        project.head = ProjectVersion.objects.create(project=project)
        project.save()
        return project

    def get_by_slug(self, organization: str, project: str):
        return self.get(organization__slug=organization, slug=project)


class Project(TaggableMixin, UUIDModel):
    """
    A project to instruct an AI to do something.

    Projects are the root of versioning, similar to repositories in Git.
    All versions are available in 'versions' and may not be linear (also like in Git).
    Projects define symbols organized into files.

    Executable projects have main programs (top-level program instructions).
    Library projects define reusable symbols (like in software).
    """

    type = TextChoicesField(choices_enum=ProjectType, default=ProjectType.EXECUTABLE)
    name: models.CharField = models.CharField(max_length=MAX_NAME_LENGTH)
    slug: models.SlugField = models.SlugField(max_length=128, validators=[validate_slug])
    created_at: models.DateTimeField = models.DateTimeField(auto_now_add=True)
    updated_at: models.DateTimeField = models.DateTimeField(auto_now=True)

    # TODO @Feature: basic branching, move Project head into main_branch.head
    head = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, null=True, related_name="project+"
    )
    # branches via ProjectBranch

    organization: models.ForeignKey = models.ForeignKey(
        "Organization", on_delete=models.CASCADE, related_name="projects"
    )

    def __str__(self):
        return f"{self.organization.slug}/{self.slug}"

    @transaction.atomic
    def create_version(
        self,
        name: Optional[str] = None,
        description: Optional[str] = None,
        parent: Optional[ProjectVersion] = None,
        auto_commit: bool = True,
    ) -> "ProjectVersion":
        if parent is None:
            if self.head is None:
                raise ValueError(f"project does not have a head version: {self}")
            assigned_parent = self.head
        else:
            assigned_parent = parent
        del parent  # avoid accidental use

        if not assigned_parent.committed:
            if auto_commit:
                assigned_parent.commit()
            else:
                raise ValueError(f"parent version must be committed: {assigned_parent}")

        new_version = ProjectVersion.objects.create(
            project=self, name=name, description=description
        )
        new_version.parents.add(assigned_parent)

        # copy all project files and their symbol definitions from parent
        # TODO @Performance: copy project version on commit server-side in SQL
        #  This is awfully sequential and slow.
        # 1. copy project files
        new_files: dict[UUID, File] = {}
        for file in assigned_parent.files.all():
            old_id = file.id
            file.pk = None
            file.project_version = new_version
            file.save()
            new_files[old_id] = file
        # 1.1 re-assign project file parents
        for file in assigned_parent.files.all():
            if file.parent_id is not None:
                new_file = new_files[file.id]
                new_file.parent = new_files[(file)]
                new_file.save()
        # 2. copy symbol definitions
        new_definitions: dict[UUID, SymbolDefinition] = {}
        for definition in assigned_parent.definitions.all():
            old_id = definition.id
            definition.pk = None
            definition.parent = None
            definition.file = new_files[definition.file_id]
            definition.project_version = new_version
            definition.save()
            new_definitions[old_id] = definition
        # 2.1 re-assign symbol definition parents
        for definition in assigned_parent.definitions.all():
            if definition.parent_id is not None:
                new_definition = new_definitions[definition.id]
                new_definition.parent = new_definitions[definition.parent_id]
                new_definition.save()

        # head has advanced to new version
        if assigned_parent == self.head:
            self.head = new_version

        return new_version

    objects: ProjectManager = ProjectManager()

    class Meta:
        default_related_name = "projects"
        constraints = [
            models.UniqueConstraint(
                name="bench_project_organization_slug_ak",
                fields=["organization", "slug"],
            )
        ]


class ProjectBranch(UUIDModel):
    """
    A branch of a project, like in Git. A branch is a pointer to a project version.
    """

    name: models.CharField = models.CharField(max_length=MAX_NAME_LENGTH)
    project: models.ForeignKey = models.ForeignKey(
        "Project", on_delete=models.CASCADE, related_name="branches"
    )
    head: models.ForeignKey = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="branches+"
    )

    class Meta:
        constraints = [
            models.UniqueConstraint(
                name="bench_project_branch_project_name_ak",
                fields=["project", "name"],
            )
        ]


class ProjectVersion(TaggableMixin, UUIDModel):
    """
    A project version records the state of a project at a specific point in time.
    """

    project = models.ForeignKey(Project, on_delete=models.CASCADE, related_name="versions")
    name = models.CharField(max_length=MAX_NAME_LENGTH, null=True)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, null=True)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    committed_at = models.DateTimeField(null=True)

    parents = models.ManyToManyField(
        "ProjectVersion", related_name="children", symmetrical=False, blank=True
    )
    # files via ProjectFile
    # definitions via SymbolDefinition
    libraries = models.ManyToManyField("ProjectVersion", related_name="dependents", blank=True)
    program: models.ForeignKey = models.ForeignKey(
        "Symbol", on_delete=models.CASCADE, null=True, related_name="projects"
    )
    backends: models.ManyToManyField = models.ManyToManyField(
        "Symbol", related_name="referenced_in_projects+", blank=True
    )

    def __str__(self) -> str:
        return f"{self.organization.slug}/{self.project.slug}@{self.id.hex}"

    def available_definitions(self, include_libraries: bool = True) -> QuerySet[SymbolDefinition]:
        # get own and libraries definitions (non-recursive for now)
        if include_libraries:
            return SymbolDefinition.objects.filter(
                Q(project_version_id__in=(self.id, *self.libraries.values_list("id", flat=True)))
            )
        else:
            return self.definitions.all()

    @transaction.atomic
    def create_file(
        self,
        name: str,
        parent: Optional[File] = None,
        definitions: Optional[list[SymbolDefinition]] = None,
    ) -> "File":
        file = File.objects.create(project_version=self, parent=parent, name=name)
        if definitions:
            file.definitions.set(definitions)
        return file

    @transaction.atomic
    def create_path(self, path: str, is_folder: bool, exists_ok: bool = False) -> "File":
        """
        Create a file or folder at the given path, automatically creating parent folders.
        """
        file_parts = path.split("/")
        # create parent folders
        parent = None
        for folder in file_parts[:-1]:
            parent, _ = File.objects.get_or_create(
                project_version=self, parent=parent, name=folder, is_folder=True
            )
        # create file
        file, created = File.objects.get_or_create(
            project_version=self, parent=parent, name=file_parts[-1], is_folder=is_folder
        )
        if not created and not exists_ok:
            raise ValueError(f"file already exists: {file}")
        return file

    def create_file_from_path(self, path: str, exists_ok: bool = False) -> "File":
        return self.create_path(path, is_folder=False, exists_ok=exists_ok)

    def create_folder_from_path(self, path: str, exists_ok: bool = False) -> "File":
        return self.create_path(path, is_folder=True, exists_ok=exists_ok)

    def resolve(
        self, symbol: Symbol, prefetch: list[str] | None = None
    ) -> Optional[SymbolDefinition]:
        """
        Resolve a symbol to a definition in this project version.
        If the symbol isn't defined here, we check the imported libraries.
        """
        return ProjectVersion.resolve_id(self, symbol, prefetch)

    def resolve_sure(self, symbol: Symbol, prefetch: list[str] | None = None) -> SymbolDefinition:
        """
        Resolve a symbol to a definition in this project version.
        If the symbol isn't defined here, we check the imported libraries.
        Raises an exception if the symbol is not defined.
        """
        definition = self.resolve(symbol, prefetch)
        if definition is None:
            available_symbols_str = self._get_available_symbols_debug_str()
            raise ValueError(f"symbol {symbol} is not defined in {self}:\n{available_symbols_str}")
        return definition

    async def aresolve(
        self, symbol: Symbol, prefetch: list[str] | None = None
    ) -> Optional[SymbolDefinition]:
        return await sync_to_async(self.resolve)(symbol, prefetch)

    async def aresolve_sure(
        self, symbol: Symbol, prefetch: list[str] | None = None
    ) -> SymbolDefinition:
        return await sync_to_async(self.resolve_sure)(symbol, prefetch)

    @staticmethod
    def resolve_id(
        project_v: ProjectVersion | UUID, symbol: Symbol | UUID, prefetch: list[str] | None = None
    ) -> Optional[SymbolDefinition]:
        if not isinstance(project_v, UUID):
            project_v = project_v.id
        if not isinstance(symbol, UUID):
            symbol = symbol.id

        # TODO @Architecture @Performance: revisit symbol resolution logic
        #  Symbol resolution should be baked into all queries (not done in Python).
        #  This should also include our GraphQL API for optimal performance.
        #   (hook into strawberry_django_plus query optimizer).
        #  (also see ProjectVersion.available_definitions)
        libraries = ProjectVersion.objects.filter(id=project_v).values_list(
            "libraries__id", flat=True
        )
        available_defs = SymbolDefinition.objects.filter(
            Q(project_version_id__in=libraries) | Q(project_version_id=project_v)
        )
        if prefetch:
            available_defs = available_defs.prefetch_related(*prefetch)
        definition = available_defs.filter(symbol=symbol).select_related("symbol").first()
        return definition

    @transaction.atomic
    def define_symbol(
        self,
        name: str,
        content: SymbolContent,
        file: File,
        parent: Optional[SymbolDefinition] = None,
        index: Optional[int] = None,
        symbol: Optional[Symbol] = None,
    ) -> SymbolDefinition:
        """
        Define a symbol in this project version.
        A corresponding symbol is declared if it's not passed.
        """
        # auto set index if not passed
        if index is None:
            if parent:
                index = parent.children.count()
            else:
                index = file.definitions.count()

        return SymbolDefinition.objects.create_definition(
            content=content,
            project_version=self,
            symbol=symbol,
            name=name,
            file=file,
            parent=parent,
            index=index,
        )

    def get_symbol_definitions(
        self, name: str, type: Optional[SymbolType] = None
    ) -> QuerySet[SymbolDefinition]:
        """
        Gets the definitions of a symbol in this project version.
        """
        if type is not None:
            return self.available_definitions().filter(name=name, type=type)
        else:
            return self.available_definitions().filter(name=name)

    def get_symbol_definition(
        self, name: str, type: Optional[SymbolType] = None
    ) -> Optional[SymbolDefinition]:
        """
        Gets the definition of a symbol in this project version.
        """
        return self.get_symbol_definitions(name, type).first()

    def get_symbol(self, name: str, type: Optional[SymbolType] = None) -> Optional[Symbol]:
        """
        Gets the symbol corresponding to the given definition in this project version.
        """
        symbol_def = self.get_symbol_definition(name, type)
        if symbol_def is None:
            return None
        else:
            return symbol_def.symbol

    def symbol(self, name: str, type: Optional[SymbolType] = None) -> Symbol:
        """
        Gets the symbol corresponding to the given definition in this project version.
        If the symbol doesn't exist, we error.
        """
        return self.symbol_definition(name, type).symbol

    def symbol_definition(self, name: str, type: Optional[SymbolType] = None) -> SymbolDefinition:
        """
        Gets the definition of a symbol in this project version.
        If the definition doesn't exist, we error.
        """
        definition = self.get_symbol_definition(name, type)
        if definition is None:
            available_symbols_str = self._get_available_symbols_debug_str()
            raise ValueError(
                f"symbol {name}{'.' + type if type else ''} is not defined in {self}:\n{available_symbols_str}"
            )
        else:
            return definition

    def _get_available_symbols_debug_str(self, limit: int = 50) -> str:
        available_symbols_count = self.available_definitions().count()
        available_symbols_strs = (str(d) for d in self.available_definitions()[:limit])
        available_symbols_str = (
            f"({min(limit, available_symbols_count)} of {available_symbols_count}"
            f" available symbols: {', '.join(available_symbols_strs)})"
        )
        return available_symbols_str

    def reset(self):
        # deletes all our references and definitions but not their contents
        self.files.all().delete()
        self.definitions.all().delete()
        # TODO @Cleanup: gc unreferenced symbol contents

    @transaction.atomic
    def commit(self, name: Optional[str] = None):
        self.committed_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        self.name = name
        # mark all symbol definitions as committed if they aren't already
        # TODO @Performance: commit symbol content server-side in SQL
        for symbol_def in self.definitions.all().prefetch_related(
            "task", "instruction", "model", "dataset", "dataset_view"
        ):
            if not symbol_def.content.committed:
                symbol_def.content.committed_in = self
                symbol_def.content.save()
        self.save()

    @property
    def committed(self):
        return self.committed_at is not None

    @property
    def organization(self):
        return self.project.organization


class File(UUIDModel):
    """
    A file defining symbols for a project version, potentially containing other files if it's a folder.
    """

    project_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="files"
    )
    name: models.CharField = models.CharField(max_length=MAX_NAME_LENGTH)

    is_folder = models.BooleanField(default=False)
    parent = models.ForeignKey("File", on_delete=models.CASCADE, null=True, related_name="files")

    # definitions via SymbolDefinition
    # files via File (if in a folder)

    def __str__(self):
        if self.parent:
            return f"{self.parent}/{self.name}"
        else:
            return f"{self.project_version}/{self.name}"

    def add_definition(self, definition: SymbolDefinition):
        definition.file = self
        definition.index = self.definitions.count()
        definition.save()

    def create_definition(
        self,
        name: str,
        content: SymbolContent,
        symbol: Optional[Symbol] = None,
        parent: Optional[SymbolDefinition] = None,
    ) -> SymbolDefinition:
        definition = SymbolDefinition.objects.create_definition(
            name=name,
            content=content,
            project_version=self.project_version,
            symbol=symbol,
            parent=parent,
            file=self,
            index=self.definitions.count(),
        )
        return definition

    @property
    def root(self) -> bool:
        return self.parent is None

    @property
    def path(self) -> str:
        return f"{self.parent.path}/{self.name}" if self.parent else self.name

    class Meta:
        constraints = [
            # ensure that the path is unique per project version (includes parent folder)
            models.UniqueConstraint(
                name="bench_project_file_project_name_ak",
                fields=["project_version_id", "name"],
                condition=models.Q(parent_id__isnull=True),
            ),
            models.UniqueConstraint(
                name="bench_project_file_project_parent_name_ak",
                fields=["project_version_id", "parent_id", "name"],
                condition=models.Q(parent_id__isnull=False),
            ),
        ]
