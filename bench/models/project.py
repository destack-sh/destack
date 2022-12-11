from __future__ import annotations

from datetime import datetime
from typing import TYPE_CHECKING, Optional
from uuid import UUID

import pytz
from django.core.validators import validate_slug
from django.db import models, transaction
from django.db.models import Q, QuerySet
from django_choices_field import TextChoicesField
from strawberry_django_plus import gql

from bench.models.symbol import Symbol, SymbolContent, SymbolType
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

    Executable projects have main programs (top-level program code).
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

    organization: models.ForeignKey = models.ForeignKey(
        "Organization", on_delete=models.CASCADE, related_name="projects"
    )

    def __str__(self):
        return f"{self.organization.slug}/{self.slug}"

    @property
    def head_(self) -> ProjectVersion:
        if self.head is None:
            raise ValueError(f"project {self} has no head")
        return self.head

    @transaction.atomic
    def create_version(
        self,
        name: Optional[str] = None,
        description: Optional[str] = None,
        parent: Optional[ProjectVersion] = None,
        auto_commit: bool = True,
        commit_name: Optional[str] = None,
        commit_description: Optional[str] = None,
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
                assigned_parent.commit(commit_name, commit_description)
            else:
                raise ValueError(f"parent version must be committed: {assigned_parent}")

        new_version = ProjectVersion.objects.create(
            project=self, name=name, description=description
        )
        new_version.parents.add(assigned_parent)

        # copy project content from parent
        ProjectVersion.copy_project_version(assigned_parent, new_version)

        # head has advanced to new version
        if assigned_parent == self.head:
            self.head = new_version
            self.save()

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
    libraries = models.ManyToManyField("ProjectVersion", related_name="dependents", blank=True)
    main_program = models.ForeignKey(
        "Symbol", related_name="+", null=True, on_delete=models.SET_NULL
    )
    files: models.QuerySet["File"]  # noqa via File
    symbols: models.QuerySet["Symbol"]  # noqa via Symbol
    compilations: models.QuerySet["Compilation"]  # noqa via Compilation

    @staticmethod
    def copy_project_version(source: ProjectVersion, target: ProjectVersion):
        # TODO @Performance: copy project version on commit server-side in SQL
        #  This is awfully sequential and slow, particularly deepcopy of symbols.
        # TODO @Cleanup: content created_at/updated_at are not copied correctly (they are set to now)
        # 1. copy project files
        new_files: dict[UUID, File] = {}
        for file in source.files.all():
            old_id = file.id
            file.pk = None
            file.project_version = target
            file.save()
            new_files[old_id] = file
        # 1.1 re-assign project file parents
        for old_file in source.files.all():
            if old_file.parent_id is not None:
                new_file = new_files[old_file.id]
                new_file.parent = new_files[old_file.parent_id]
                new_file.save()
        # 2. copy symbols and symbol contents
        new_symbols: dict[UUID, Symbol] = {}
        new_contents: dict[UUID, SymbolContent] = {}
        for symbol in source.symbols.all():
            old_content: SymbolContent = symbol.content
            # copy content
            old_id = old_content.id
            content = old_content
            content.pk = None
            content.save()
            new_contents[old_id] = content
            # copy symbol
            old_id = symbol.id
            symbol.pk = None
            symbol.parent = None
            symbol.set_content(content)
            symbol.file = new_files[symbol.file_id]
            symbol.project_version = target
            symbol.save()
            new_symbols[old_id] = symbol
        refs: dict[UUID, SymbolContent | Symbol] = {**new_symbols, **new_contents}
        # 2.1 re-assign references and deep copy symbols
        for old_symbol in source.symbols.all():
            new_symbol = new_symbols[old_symbol.id]
            old_symbol.deepcopy(to=new_symbol, refs=refs)
            # re-assign symbol parent and content
            if old_symbol.parent_id is not None:
                new_symbol.parent = new_symbols[old_symbol.parent_id]
            new_symbol.save()
        if source.main_program:
            target.main_program = new_symbols[source.main_program_id]

    def __str__(self) -> str:
        return f"{self.organization.slug}/{self.project.slug}@{self.id.hex}"

    def available_symbols(self, include_libraries: bool = True) -> QuerySet[Symbol]:
        # get own and libraries symbols (non-recursive for now)
        if include_libraries:
            return Symbol.objects.filter(
                Q(project_version_id__in=(self.id, *self.libraries.values_list("id", flat=True)))
            )
        else:
            return self.symbols.all()

    @transaction.atomic
    def create_file(
        self,
        name: str,
        parent: Optional[File] = None,
        symbols: Optional[list[Symbol]] = None,
    ) -> "File":
        file = File.objects.create(project_version=self, parent=parent, name=name)
        if symbols:
            file.symbols.set(symbols)
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

    @transaction.atomic
    def define_symbol(
        self,
        name: str,
        content: SymbolContent,
        file: File,
        parent: Optional[Symbol] = None,
        index: Optional[int] = None,
    ) -> Symbol:
        """
        Define a symbol in this project version.
        A corresponding symbol is declared if it's not passed.
        """
        # auto set index if not passed
        if index is None:
            if parent:
                index = parent.children.count()
            else:
                index = file.symbols.count()

        return Symbol.objects.create_symbol(
            content=content,
            project_version=self,
            name=name,
            file=file,
            parent=parent,
            index=index,
        )

    def get_symbols(self, name: str, type: Optional[SymbolType] = None) -> QuerySet[Symbol]:
        """
        Gets the symbols of a symbol in this project version.
        """
        if type is not None:
            return self.available_symbols().filter(name=name, type=type)
        else:
            return self.available_symbols().filter(name=name)

    def get_symbol(self, name: str, type: Optional[SymbolType] = None) -> Optional[Symbol]:
        """
        Gets the symbol of a symbol in this project version.

        Raises Symbol.MultipleObjectsReturned if there are multiple symbols.
        """
        try:
            return self.get_symbols(name, type).get()
        except Symbol.MultipleObjectsReturned as e:
            type_name_declr = f"{type} {name}" if type else name
            raise Symbol.MultipleObjectsReturned(
                f"multiple symbols for symbol {type_name_declr} in {self}"
            ) from e
        except Symbol.DoesNotExist:
            return None

    def symbol(self, name: str, type: Optional[SymbolType] = None) -> Symbol:
        """
        Gets the symbol of a symbol in this project version.
        If the symbol doesn't exist, we error.
        """
        symbol = self.get_symbol(name, type)
        if symbol is None:
            available_symbols_str = self._get_available_symbols_debug_str()
            raise ValueError(
                f"symbol {name}{'.' + type if type else ''} is not defined in {self}:\n{available_symbols_str}"
            )
        else:
            return symbol

    def _get_available_symbols_debug_str(self, limit: int = 50) -> str:
        available_symbols_count = self.available_symbols().count()
        available_symbols_strs = (str(d) for d in self.available_symbols()[:limit])
        available_symbols_str = (
            f"({min(limit, available_symbols_count)} of {available_symbols_count}"
            f" available symbols: {', '.join(available_symbols_strs)})"
        )
        return available_symbols_str

    def reset(self):
        # deletes all files and symbols (cascades to contents)
        self.symbols.all().delete()
        self.files.all().delete()

    @transaction.atomic
    def commit(self, name: Optional[str] = None, description: Optional[str] = None):
        if self.committed:
            raise ValueError(f"already committed: {self}")

        self.committed_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        if name is not None:
            self.name = name
        if description is not None:
            self.description = description
        self.save()

    @gql.model_property(only=["committed_at"])
    def committed(self) -> bool:
        return self.committed_at is not None

    @property
    def organization(self):
        return self.project.organization

    class Meta:
        ordering = ["-created_at"]


class File(UUIDModel):
    """
    A file defining symbols for a project version, potentially containing other files if it's a folder.
    """

    project_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="files"
    )
    name: models.CharField = models.CharField(max_length=MAX_NAME_LENGTH)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    is_folder = models.BooleanField(default=False)
    parent = models.ForeignKey(
        "File", on_delete=models.CASCADE, null=True, blank=True, related_name="files"
    )

    files: models.QuerySet["File"]  # noqa via File.parent (if is_folder)
    symbols: models.QuerySet["Symbol"]  # noqa via Symbol.file

    def __str__(self):
        if self.parent:
            return f"{self.parent}/{self.name}"
        else:
            return f"{self.project_version}/{self.name}"

    @gql.model_property(only=["name", "parent"], select_related=["parent"])
    def path(self) -> str:
        return f"{self.parent.path}/{self.name}" if self.parent else self.name

    @property
    def is_root(self) -> bool:
        return self.parent is None

    def add_symbol(self, symbol: Symbol):
        symbol.file = self
        if symbol.parent is None:
            symbol.index = self.symbols.count()
        symbol.save()

    def create_symbol(
        self, name: str, content: SymbolContent, parent: Optional[Symbol] = None, **kwargs
    ) -> Symbol:
        index = self.symbols.count() if parent is None else parent.children.count()
        symbol = Symbol.objects.create_symbol(
            name=name,
            content=content,
            project_version=self.project_version,
            parent=parent,
            file=self,
            index=index,
            **kwargs,
        )
        return symbol

    class Meta:
        ordering = ["name"]
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
