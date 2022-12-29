from __future__ import annotations

from collections import deque
from datetime import datetime
from typing import TYPE_CHECKING, Deque, Iterator, Optional, TypedDict, TypeVar
from uuid import UUID

import pytz
from django.core.validators import validate_slug
from django.db import models, transaction
from django.db.models import Q, QuerySet
from django_choices_field import TextChoicesField
from strawberry_django_plus import gql

from bench.models import Compilation
from bench.models.symbol import (
    Requirement,
    RunConfiguration,
    Statement,
    StatementType,
    SymbolContent,
    SymbolType,
)
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


RefDict = TypedDict("RefDict", {"source": str, "target": str, "type": str})


class Project(TaggableMixin, UUIDModel):
    """
    A project to instruct an AI to do some things.

    Projects are the root of versioning, similar to repositories in Git.
    All versions are available in 'versions' and may not be linear (also like in Git).
    Projects contain statements organized into files (organized into directories).

    Executable projects have main programs (top-level code).
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

    @gql.model_property(only=["organization", "slug"], select_related=["organization"])
    def path(self) -> str:
        return f"{self.organization.slug}.{self.slug}"

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
        refs = ProjectVersion.objects.copy(assigned_parent, new_version)
        new_version.parents_refs = [
            RefDict(source=str(k), target=str(v.id), type=type(v).__name__) for k, v in refs.items()
        ]
        new_version.save()

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


T = TypeVar("T")


def walk_children_bfs(objects: list[T], child_attr: str) -> Iterator[T]:
    """
    Walk all children of an object in breadth-first order.
    """
    queue: Deque[T] = deque(objects)
    while queue:
        obj = queue.popleft()
        # copy children before yielding to avoid concurrent modification while copying
        children = list(getattr(obj, child_attr).all())
        yield obj
        queue.extend(children)


class ProjectVersionManager(models.Manager["ProjectVersion"]):
    def get_by_slug(self, organization: str, project: str, version: str):
        return self.get(
            project__organization__slug=organization,
            project__slug=project,
            slug=version,
        )

    def copy(
        self, source: ProjectVersion, target: ProjectVersion
    ) -> dict[
        UUID, File | Statement | SymbolContent | Compilation | Requirement | RunConfiguration
    ]:
        # TODO @Performance: copy project version on commit server-side (in SQL)
        #  This is awfully sequential and slow, particularly deepcopy of symbol contents.
        #  For one, we can likely just bulk save if we defer parent/child relations to a second pass.
        # TODO @Cleanup: content created_at/updated_at are not copied correctly (they are set to now)
        # 1. copy project files
        new_files: dict[UUID, File] = {}
        for file in walk_children_bfs(source.files.filter(deleted_at=None), "files"):
            old_id = file.id
            file.pk = None
            file.project_version = target
            file.parent = new_files.get(file.parent_id)
            file.save()
            new_files[old_id] = file
        # 2. copy statements and their contents
        new_statements: dict[UUID, Statement] = {}
        new_contents: dict[UUID, SymbolContent] = {}
        new_compilations: dict[UUID, Compilation] = {}
        new_requirements: dict[UUID, Requirement] = {}
        new_runconfigs: dict[UUID, RunConfiguration] = {}
        for statement in walk_children_bfs(
            source.statements.filter(deleted_at=None, parent=None), "children"
        ):
            old_content = statement.content if statement.type == StatementType.DEFINITION else None
            old_compilation = (
                statement.compilation if statement.type == StatementType.COMPILATION else None
            )
            old_requirement = (
                statement.requirement if statement.type == StatementType.REQUIREMENT else None
            )
            old_runconfig = (
                statement.runconfig if statement.type == StatementType.RUNCONFIG else None
            )
            # copy statement
            old_id = statement.id
            statement.pk = None
            statement.revision = 0  # reset revision
            statement.file = new_files[statement.file_id]
            statement.project_version = target
            statement.set_content(None)
            statement.compilation = None
            statement.requirement = None
            statement.reference = None
            statement.parent = new_statements.get(statement.parent_id)
            new_statements[old_id] = statement
            # if statement is a definition, copy symbol content
            if old_content is not None:
                old_id = old_content.id
                content = old_content
                content.pk = None
                content.save()
                statement.set_content(content)
                new_contents[old_id] = content
            # if statement is a compilation, copy compilation
            if old_compilation is not None:
                old_id = old_compilation.id
                compilation = old_compilation
                compilation.pk = None
                compilation.save()
                statement.compilation = compilation
                new_compilations[old_id] = compilation
            # if statement is a requirement, copy requirement
            if old_requirement is not None:
                old_id = old_requirement.id
                requirement = old_requirement
                requirement.pk = None
                requirement.save()
                statement.requirement = requirement
                new_requirements[old_id] = requirement
            # if statement is a runconfig, copy runconfig
            if old_runconfig is not None:
                old_id = old_runconfig.id
                runconfig = old_runconfig
                runconfig.pk = None
                runconfig.save()
                statement.runconfig = runconfig
                new_runconfigs[old_id] = runconfig
            statement.save()
        refs = {
            **new_contents,
            **new_requirements,
            **new_compilations,
            **new_statements,
            **new_files,
        }
        # 3 re-assign references and deep copy other relations
        for old_statement in source.statements.filter(deleted_at=None):
            new_statement = new_statements[old_statement.id]
            old_statement.deepcopy(new_statement, refs)
            new_statement.save()

        return refs


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
    parents_refs = models.JSONField(default=dict)
    dependencies = models.ManyToManyField("ProjectVersion", related_name="dependents", blank=True)
    files: models.QuerySet["File"]  # noqa via File
    symbols: models.QuerySet["Symbol"]  # noqa via Symbol
    statements: models.QuerySet["Statement"]  # noqa via Statement

    def __str__(self) -> str:
        return f"{self.organization.slug}/{self.project.slug}@{self.id.hex}"

    def reset(self):
        """Hard deletes all files (cascades to statements and their contents)."""
        self.files.all().delete()

    def bootstrap(self):
        """Creates default files and statements."""
        # TODO @Cleanup: replace the bootstrapped files with templated snippets in Bench format
        # (and move out of ProjectVersion?)
        if self.project.type == ProjectType.EXECUTABLE:
            main_file = self.create_file("main", FileType.INSTRUCT)
            self.add_comment("Instruct AI on something.", main_file)
        elif self.project.type == ProjectType.LIBRARY:
            main_file = self.create_file("main", FileType.INSTRUCT)
            self.add_comment("Your AI library root.", main_file)
        else:
            raise ValueError(f"unexpected project type {self.project.type}")
        project_file = self.create_file("bench", FileType.PROJECT)
        self.add_comment("# Project metadata", project_file)
        self.add_blank(project_file)

        self.add_comment("# Requirements", project_file)
        # default requirements
        self.add_requirement(Project.objects.get_by_slug("symbolx", "stdlib").head_, project_file)

        # TODO @Feature: bootstrap project version with default/template content
        if self.project.type == ProjectType.EXECUTABLE:
            self.add_comment("# Compilations", project_file)
            self.add_blank(project_file)
            self.add_comment("# Run", project_file)
            self.add_blank(project_file)

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

    def dependency(self, organization_slug: str, project_slug: str) -> ProjectVersion:
        return self.dependencies.filter(
            project__organization__slug=organization_slug, project__slug=project_slug
        ).get()

    @transaction.atomic
    def derive_dependencies(self) -> None:
        """Re-derives dependencies from all dependency statements"""
        self.dependencies.clear()

        # only use one dependency statement per project, error if there are multiple
        dependency_by_project: dict[Project, ProjectVersion] = {}
        for statement in self.statements.filter(type=StatementType.REQUIREMENT):
            dependency = statement.requirement.project_version
            dependency_project = dependency.project
            if dependency_project in dependency_by_project:
                raise ValueError(f"multiple dependency statements for project: {statement}")
            dependency_by_project[dependency_project] = dependency
            self.dependencies.add(dependency)

    @transaction.atomic
    def create_file(
        self,
        name: str,
        type: FileType,
        parent: Optional[File] = None,
    ) -> "File":
        file = File.objects.create(project_version=self, parent=parent, name=name, type=type)
        return file

    @transaction.atomic
    def create_path(
        self, path: str, type: FileType, exists_ok: bool = False, id: Optional[UUID] = None
    ) -> "File":
        """
        Create a file or directory at the given path, automatically creating parent directories.
        """
        file_parts = path.split("/")
        # create parent directories
        parent = None
        for directory in file_parts[:-1]:
            parent, _ = File.objects.get_or_create(
                project_version=self, parent=parent, name=directory, type=type
            )
        # create file
        file, created = File.objects.get_or_create(
            project_version=self,
            parent=parent,
            name=file_parts[-1],
            type=type,
            defaults={"id": id} if id is not None else {},
        )
        if not created and not exists_ok:
            raise ValueError(f"file already exists: {file}")
        return file

    def create_file_from_path(
        self, path: str, type: FileType, exists_ok: bool = False, id: Optional[UUID] = None
    ) -> "File":
        return self.create_path(path, type, exists_ok=exists_ok, id=id)

    def get_file(self, path: str, type: FileType) -> "File":
        try:
            file_parts = path.split("/")
            parent = None
            for directory in file_parts[:-1]:
                parent = File.objects.get(project_version=self, parent=parent, name=directory)
            return File.objects.get(
                project_version=self, parent=parent, name=file_parts[-1], type=type
            )
        except File.DoesNotExist:
            raise ValueError(f"project {self} does not contain {path}.{type}")

    @property
    def project_file(self) -> File:
        return self.files.filter(type=FileType.PROJECT, deleted_at=None).get()

    def available_statements(self, include_dependencies: bool = True) -> QuerySet[Statement]:
        # get own and dependencies symbols (non-recursive for now)
        if include_dependencies:
            return Statement.objects.filter(
                Q(project_version_id__in=(self.id, *self.dependencies.values_list("id", flat=True)))
            )
        else:
            return self.statements.all()

    def add_comment(self, text: str, file: File, index: Optional[int] = None) -> Statement:
        return Statement.objects.create_statement(
            project_version=self,
            file=file,
            parent=None,
            index=index,
            type=StatementType.COMMENT,
            text=text,
            name=None,
        )

    def add_blank(self, file: File, index: Optional[int] = None) -> Statement:
        return Statement.objects.create_statement(
            project_version=self,
            file=file,
            parent=None,
            index=index,
            type=StatementType.BLANK,
            name=None,
        )

    @transaction.atomic
    def add_requirement(
        self, dependency_v: ProjectVersion, file: File, index: Optional[int] = None
    ) -> Statement:
        requirement = Requirement.objects.create(project_version=dependency_v)
        statement = Statement.objects.create_statement(
            project_version=self,
            file=file,
            parent=None,
            index=index,
            type=StatementType.REQUIREMENT,
            requirement=requirement,
            name=dependency_v.project.name,
        )
        self.derive_dependencies()
        return statement

    def define_symbol(
        self,
        name: str,
        content: SymbolContent,
        file: File,
        parent: Optional[Statement] = None,
        index: Optional[int] = None,
    ) -> Statement:
        """Define a symbol in this project version in the given file."""
        definition = Statement.objects.create_definition(
            content=content,
            project_version=self,
            name=name,
            file=file,
            parent=parent,
            index=index,
        )
        return definition

    def import_statement(
        self,
        statement: Statement,
        alias: Optional[str],
        file: File,
        parent: Optional[Statement] = None,
        index: Optional[int] = None,
    ) -> Statement:
        """Imports a statement from another file."""
        if statement.file == file:
            raise ValueError(f"cannot import {statement} in same file {file}")
        if statement.type == StatementType.IMPORT:
            raise ValueError(f"cannot import an import statement {statement}")
        if (
            statement.file.project_version != self
            and statement.file.project_version not in self.dependencies.all()
        ):
            raise ValueError(
                f"cannot import {statement} from {statement.file.project_version} (not a dependency)"
            )

        import_statement = Statement.objects.create_import(
            statement=statement,
            project_version=self,
            file=file,
            name=alias or statement.name,
            parent=parent,
            index=index or 0,  # imports are always at the top by default
        )
        return import_statement

    def get_import_of(self, file: File, statement: Statement) -> Optional[Statement]:
        return self.statements.filter(
            file=file, type=StatementType.IMPORT, reference=statement
        ).first()

    def reference_statement(
        self,
        statement: Statement,
        alias: Optional[str],
        file: File,
        parent: Optional[Statement] = None,
        index: Optional[int] = None,
    ) -> Statement:
        """References a statement from another file."""
        if statement.file != file:
            raise ValueError(f"cannot reference statement {statement} from {file}")

        reference_statement = Statement.objects.create_reference(
            statement=statement,
            project_version=self,
            file=file,
            name=alias or statement.name,
            parent=parent,
            index=index,
        )
        return reference_statement

    def get_statements(
        self,
        file: Optional[File],
        name: str,
        type: Optional[StatementType],
        symbol_type: Optional[SymbolType] = None,
    ) -> QuerySet[Statement]:
        qs = self.available_statements().filter(name=name)
        if type is not None:
            qs = qs.filter(type=type)
        if symbol_type is not None:
            qs = qs.filter(symbol_type=symbol_type)
        if file is not None:
            qs = qs.filter(file=file)
        return qs

    def get_statement(
        self,
        file: Optional[File],
        name: str,
        type: Optional[StatementType] = None,
        symbol_type: Optional[SymbolType] = None,
    ) -> Optional[Statement]:
        try:
            return self.get_statements(file, name, type, symbol_type).get()
        except Statement.MultipleObjectsReturned as e:
            raise Statement.MultipleObjectsReturned(
                f"multiple statements with file={file} name={name} type={type} symbol_type={symbol_type} in {self}"
            ) from e
        except Statement.DoesNotExist:
            return None

    def statement(
        self,
        file: Optional[File],
        name: str,
        type: Optional[StatementType] = None,
        symbol_type: Optional[SymbolType] = None,
    ) -> Statement:
        statement = self.get_statement(file, name, type, symbol_type)
        if statement is None:
            available_symbols_str = self._get_available_symbols_str()
            raise ValueError(
                f"symbol file={file} name={name} type={type} symbol_type={symbol_type} is not defined {self}:\n{available_symbols_str}"
            )
        else:
            return statement

    def _get_available_symbols_str(self, limit: int = 50) -> str:
        available_symbols_count = self.available_statements().count()
        available_symbols_strs = (str(d) for d in self.available_statements()[:limit])
        available_symbols_str = (
            f"({min(limit, available_symbols_count)} of {available_symbols_count}"
            f" available statements: {', '.join(available_symbols_strs)})"
        )
        return available_symbols_str

    objects = ProjectVersionManager()

    class Meta:
        ordering = ["-created_at"]


class FileType(models.TextChoices):
    """The type of file determines what it is intended to contain."""

    DIRECTORY = "directory", "Directory"  # contains sub-directories and files
    INSTRUCT = "instruct", "Instructions"  # actual instructions
    PROJECT = "bench", "Project metadata"  # meta, dependencies, etc.


class FileManager(models.Manager):
    def get_queryset(self) -> models.QuerySet[Statement]:
        # soft-deleted statements are not returned by default
        return super().get_queryset().filter(deleted_at__isnull=True)


class File(UUIDModel):
    """
    A file containing statements, potentially containing other files if it's a directory.
    A file - and the statements it contains - may be soft-deleted.
    Nothing is actually deleted, but soft deleted objects are not visible and not copied on commit.
    """

    project_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="files"
    )
    type = TextChoicesField(FileType, default=FileType.INSTRUCT)
    name: models.CharField = models.CharField(max_length=MAX_NAME_LENGTH)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    deleted_at = models.DateTimeField(null=True, blank=True)
    is_directory = models.BooleanField(default=False)
    parent = models.ForeignKey(
        "File", on_delete=models.CASCADE, null=True, blank=True, related_name="files"
    )

    files: models.QuerySet["File"]  # noqa via File.parent (if it's a directory)
    statements: models.QuerySet["Statement"]  # noqa via Statement.file
    symbols: models.QuerySet["Symbol"]  # noqa via Symbol.file

    def __str__(self):
        if self.parent:
            return f"{self.parent}/{self.name}.{self.type}"
        else:
            return f"{self.project_version}/{self.name}.{self.type}"

    @gql.model_property(only=["name", "parent"], select_related=["parent"])
    def path(self) -> str:
        return (
            f"{self.parent.path}/{self.name}.{self.type}"
            if self.parent
            else f"{self.name}.{self.type}"
        )

    @gql.model_property(only=["name", "parent"], select_related=["parent"])
    def path_without_extension(self) -> str:
        return f"{self.parent.path_without_extension}/{self.name}" if self.parent else self.name

    @property
    def is_root(self) -> bool:
        return self.parent is None

    @property
    def root_statements(self) -> models.QuerySet["Statement"]:
        return self.statements.filter(parent=None)

    @property
    def active_root_statements(self):
        return self.root_statements.filter(deleted_at__isnull=True, commented=False)

    def define_symbol(
        self, name: str, content: SymbolContent, parent: Optional[Statement] = None, **kwargs
    ) -> Statement:
        return self.project_version.define_symbol(
            file=self, name=name, content=content, parent=parent, **kwargs
        )

    @transaction.atomic
    def soft_delete(self):
        self.deleted_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        self.statements.update(deleted_at=self.deleted_at)
        self.save()

    @transaction.atomic
    def restore(self):
        self.deleted_at = None
        self.statements.update(deleted_at=None)
        self.save()

    objects = FileManager()

    class Meta:
        ordering = ["name"]
        constraints = [
            # ensure that the path is unique per project version (includes parent directory)
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
