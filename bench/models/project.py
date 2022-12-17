from __future__ import annotations

from collections import deque
from datetime import datetime
from typing import TYPE_CHECKING, Deque, Iterator, Optional, TypeVar
from uuid import UUID

import pytz
from django.core.validators import validate_slug
from django.db import models, transaction
from django.db.models import Q, QuerySet
from django_choices_field import TextChoicesField
from strawberry_django_plus import gql

from bench.models.symbol import Statement, StatementType, Symbol, SymbolContent, SymbolType
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


T = TypeVar("T")


def walk_children_bfs(objects: list[T], child_attr: str) -> Iterator[T]:
    """
    Walk all children of an object in breadth-first order.
    """
    queue: Deque["Symbol"] = deque(objects)
    while queue:
        obj = queue.popleft()
        # copy children before yielding to avoid concurrent modification while copying
        children = list(getattr(obj, child_attr).all())
        yield obj
        queue.extend(children)


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
    dependencies = models.ManyToManyField("ProjectVersion", related_name="dependents", blank=True)
    files: models.QuerySet["File"]  # noqa via File
    symbols: models.QuerySet["Symbol"]  # noqa via Symbol
    statements: models.QuerySet["Statement"]  # noqa via Statement
    compilations: models.QuerySet["Compilation"]  # noqa via Compilation

    @staticmethod
    def copy_project_version(source: ProjectVersion, target: ProjectVersion):
        # TODO @Performance: copy project version on commit server-side in SQL
        #  This is awfully sequential and slow, particularly deepcopy of symbols.
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
        # 2. copy statements, symbols and symbol contents
        new_statements: dict[UUID, Statement] = {}
        new_symbols: dict[UUID, Symbol] = {}
        new_contents: dict[UUID, SymbolContent] = {}
        for statement in walk_children_bfs(
            source.statements.filter(deleted_at=None, parent=None), "children"
        ):
            symbol = statement.symbol if statement.type == StatementType.DEFINITION else None
            # copy statement
            old_id = statement.id
            statement.pk = None
            statement.file = new_files[statement.file_id]
            statement.project_version = target
            statement.parent = new_statements.get(statement.parent_id)
            statement.reference = None
            statement.save()
            new_statements[old_id] = statement
            # if symbol definition, copy symbol and symbol content
            if symbol is None:
                continue
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
            symbol.definition = new_statements[symbol.definition_id]
            symbol.project_version = target
            symbol.save()
            new_symbols[old_id] = symbol
        refs: dict[UUID, SymbolContent | Symbol | Statement] = {
            **new_symbols,
            **new_contents,
            **new_statements,
        }
        # 3 re-assign references and deep copy other relations
        for old_statement in source.statements.filter(deleted_at=None):
            new_statement = new_statements[old_statement.id]
            old_statement.deepcopy(new_statement, refs)
            new_statement.save()

    def __str__(self) -> str:
        return f"{self.organization.slug}/{self.project.slug}@{self.id.hex}"

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

    def available_statements(self, include_dependencies: bool = True) -> QuerySet[Statement]:
        # get own and dependencies symbols (non-recursive for now)
        if include_dependencies:
            return Statement.objects.filter(
                Q(project_version_id__in=(self.id, *self.dependencies.values_list("id", flat=True)))
            )
        else:
            return self.statements.all()

    def define_symbol(
        self,
        name: str,
        content: SymbolContent,
        file: File,
        parent: Optional[Statement] = None,
        index: Optional[int] = None,
        **kwargs,
    ) -> Symbol:
        """Define a symbol in this project version in the given file."""
        statement = Statement.objects.create_definition(
            content=content,
            project_version=self,
            name=name,
            file=file,
            parent=parent,
            index=index,
            **kwargs,
        )
        return statement.symbol_

    def get_statements(
        self, file: Optional[File], name: str, type: Optional[SymbolType] = None
    ) -> QuerySet[Statement]:
        qs = self.available_statements().filter(name=name)
        if type is not None:
            qs = qs.filter(Q(symbol__type=type) | Q(reference__symbol__type=type))
        if file is not None:
            qs = qs.filter(file=file)
        return qs

    def get_statement(
        self, file: Optional[File], name: str, type: Optional[SymbolType] = None
    ) -> Optional[Statement]:
        try:
            return self.get_statements(file, name, type).get()
        except Statement.MultipleObjectsReturned as e:
            type_name_declr = f"{type} {name}" if type else name
            raise Statement.MultipleObjectsReturned(
                f"multiple statements like {type_name_declr} in {self}"
            ) from e
        except Statement.DoesNotExist:
            return None

    def statement(
        self, file: Optional[File], name: str, type: Optional[SymbolType] = None
    ) -> Statement:
        statement = self.get_statement(file, name, type)
        if statement is None:
            available_symbols_str = self._get_available_symbols_str()
            raise ValueError(
                f"symbol {name}{'.' + type if type else ''} is not defined in {self}:\n{available_symbols_str}"
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

    def reset(self):
        """Hard deletes all files (cascades to statements and their symbols)"""
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


class FileType(models.TextChoices):
    """The type of file determines the type of statements it can contain."""

    INSTRUCT = "instruct", "Instructions"
    # non-instruct files are "virtual" until we figure out how they should work (no actual statements)
    COMPILE = "compile", "Compilations"  # how instructions are compiled
    PROJECT = "project", "Project metadata"  # dependencies, etc.


class FileManager(models.Manager):
    def get_queryset(self) -> models.QuerySet[Statement]:
        # soft-deleted statements are not returned by default
        return super().get_queryset().filter(deleted_at__isnull=True)


class File(UUIDModel):
    """
    A file containing statements, potentially containing other files if it's a folder.
    A file - and the statements it contains - may be soft-deleted.
    Nothing is actually deleted, but soft deleted objects are not visible and not copied on commit.
    """

    project_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="files"
    )
    type = TextChoicesField(FileType, default=FileType.INSTRUCT)  # irrelevant if is_folder
    name: models.CharField = models.CharField(max_length=MAX_NAME_LENGTH)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    deleted_at = models.DateTimeField(null=True, blank=True)
    is_folder = models.BooleanField(default=False)
    parent = models.ForeignKey(
        "File", on_delete=models.CASCADE, null=True, blank=True, related_name="files"
    )

    files: models.QuerySet["File"]  # noqa via File.parent (if is_folder)
    statements: models.QuerySet["Statement"]  # noqa via Statement.file
    symbols: models.QuerySet["Symbol"]  # noqa via Symbol.file

    def __str__(self):
        if self.parent:
            return f"{self.parent}/{self.name}.{self.type}"
        else:
            return f"{self.project_version}/{self.name}.{self.type}"

    @gql.model_property(only=["name", "parent"], select_related=["parent"])
    def path(self) -> str:
        return f"{self.parent.path}/{self.name}.{self.type}" if self.parent else self.name

    @property
    def is_root(self) -> bool:
        return self.parent is None

    @property
    def root_statements(self) -> models.QuerySet["Statement"]:
        return self.statements.filter(parent=None)

    def define_symbol(
        self, name: str, content: SymbolContent, parent: Optional[Statement] = None, **kwargs
    ) -> Symbol:
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
