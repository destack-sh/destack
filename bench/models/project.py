from __future__ import annotations

from collections import deque
from datetime import datetime
from itertools import groupby
from typing import TYPE_CHECKING, Deque, Iterator, Optional, TypedDict, TypeVar
from uuid import UUID, uuid4

import pytz
from django import db
from django.core.validators import validate_slug
from django.db import models, transaction
from django_choices_field import TextChoicesField
from strawberry_django_plus import gql

from bench.language import SymbolType
from bench.models.symbol import Statement, StatementType
from bench.models.tag import TaggableMixin
from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH, UUIDModel

if TYPE_CHECKING:
    from bench.models.organization import Organization


class ProjectType(models.TextChoices):
    EXECUTABLE = "executable", "Executable"
    LIBRARY = "library", "Library"


class ProjectVisibility(models.TextChoices):
    PUBLIC = "public", "Public"
    PRIVATE = "private", "Private"


class ProjectManager(models.Manager["Project"]):
    @transaction.atomic
    def create_project(
        self,
        organization: Organization,
        name: str,
        slug: str,
        type: ProjectType = ProjectType.EXECUTABLE,
        visibility: ProjectVisibility = ProjectVisibility.PRIVATE,
    ):
        project = super().create(
            organization=organization, name=name, slug=slug, type=type, visibility=visibility
        )
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
    visibility = TextChoicesField(choices_enum=ProjectVisibility, default=ProjectVisibility.PRIVATE)
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

    def copy(self, source: ProjectVersion, target: ProjectVersion) -> dict[UUID, File | Statement]:
        if not source.committed:
            raise ValueError(f"source version must be committed: {source}")
        # TODO @Performance: copy project version on commit server-side (in SQL)
        #  (generally good, but also especially for dataset records, mappings and other relations)
        # TODO @Cleanup: created_at/updated_at are not copied correctly (they are set to now)
        #  (could control them manually in project mutation wrapper)
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
        # (pre-determine new statement ids to re-create source mappings in one go)
        statements_bfs = list(
            walk_children_bfs(source.statements.filter(deleted_at=None, parent=None), "children")
        )
        new_statements_ids: dict[UUID, UUID] = {
            statement.id: uuid4() for statement in statements_bfs
        }
        new_statements: dict[UUID, Statement] = {}
        new_contents: list[db.models.Model] = []
        for statement in statements_bfs:
            # copy statement
            old_id = statement.id
            statement.pk = new_statements_ids[old_id]
            statement._state.adding = True
            statement.revision = 0  # reset revision
            statement.file = new_files[statement.file_id]
            statement.project_version = target
            statement.reference = None
            statement.parent = new_statements.get(statement.parent_id)
            # if statement is a definition, add relations to save in batch later
            # all other symbol contents are value fields (copied automatically above)
            if statement.type == StatementType.DEFINITION:
                # the relations are saved below in step 4 (after statement creation)
                if statement.symbol_type == SymbolType.DATASET:
                    for record in statement.records.all():
                        record.pk = None
                        record.dataset = statement
                        new_contents.append(record)
                elif statement.symbol_type == SymbolType.COMPILATION:
                    for mapping in statement.generated_mappings.all():
                        mapping.pk = None
                        mapping.compilation = statement
                        mapping.source_id = new_statements_ids[mapping.source_id]
                        mapping.target_id = new_statements_ids[mapping.target_id]
                        mapping.source_revision = 0
                        mapping.target_revision = 0
                        new_contents.append(mapping)
            statement.save()
            new_statements[old_id] = statement

        # 3. re-assign references
        for old in source.statements.filter(deleted_at=None):
            if old.id not in new_statements:
                # skip ghost statement whose parent was deleted or lost somehow
                # TODO @Cleanup: fix/prevent ghost orphan statements on insert
                continue
            new = new_statements[old.id]
            new.parent = new_statements.get(old.parent_id)  # may be null
            # replace ref (default to same ref if not in refs since library refs are not copied)
            new.reference = new_statements.get(old.reference_id, old.reference)
        Statement.objects.bulk_update(new_statements.values(), ["parent", "reference"])

        # 4. save content relations
        for relation_cls, relations in groupby(new_contents, key=type):
            relation_cls.objects.bulk_create(relations)

        return {**new_statements, **new_files}


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
    files: models.QuerySet["File"]  # noqa via File
    symbols: models.QuerySet["Symbol"]  # noqa via Symbol
    statements: models.QuerySet["Statement"]  # noqa via Statement

    def __str__(self) -> str:
        return f"{self.organization.slug}/{self.project.slug}@{self.id.hex}"

    def reset(self):
        """Hard deletes all files (cascades to statements and their contents)."""
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

    @transaction.atomic
    def create_file(self, name: str, parent: Optional[File] = None) -> "File":
        file = File.objects.create(project_version=self, parent=parent, name=name)
        return file

    @transaction.atomic
    def create_path(self, path: str, exists_ok: bool = False, id: Optional[UUID] = None) -> "File":
        """
        Create a file or directory at the given path, automatically creating parent directories.
        """
        file_parts = path.split("/")
        # create parent directories
        parent = None
        for directory in file_parts[:-1]:
            parent, _ = File.objects.get_or_create(
                project_version=self, parent=parent, name=directory
            )
        # create file
        file, created = File.objects.get_or_create(
            project_version=self,
            parent=parent,
            name=file_parts[-1],
            defaults={"id": id} if id is not None else {},
        )
        if not created and not exists_ok:
            raise ValueError(f"file already exists: {file}")
        return file

    def create_file_from_path(
        self, path: str, exists_ok: bool = False, id: Optional[UUID] = None
    ) -> "File":
        return self.create_path(path, exists_ok=exists_ok, id=id)

    def get_file(self, path: str) -> "File":
        try:
            file_parts = path.split("/")
            parent = None
            for directory in file_parts[:-1]:
                parent = File.objects.get(project_version=self, parent=parent, name=directory)
            return File.objects.get(project_version=self, parent=parent, name=file_parts[-1])
        except File.DoesNotExist:
            raise ValueError(f"project {self} does not contain {path}.{type}")

    objects = ProjectVersionManager()

    class Meta:
        ordering = ["-created_at"]


class FileManager(models.Manager):
    def get_queryset(self) -> models.QuerySet[Statement]:
        # soft-deleted statements are not returned by default
        return super().get_queryset().filter(deleted_at__isnull=True)


class File(UUIDModel):
    """
    A file containing statements, potentially containing other files if it's a directory.
    A file - and the statements it contains - may be soft-deleted.
    Nothing is actually deleted, but soft deleted objects are not visible and not copied across versions.
    """

    project_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="files"
    )
    revision = models.IntegerField(default=1)
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
            return f"{self.parent}/{self.name}"
        else:
            return f"{self.project_version}/{self.name}"

    @gql.model_property(only=["name", "parent"], select_related=["parent"])
    def path(self) -> str:
        return f"{self.parent.path}/{self.name}" if self.parent else f"{self.name}"

    def is_root(self) -> bool:
        return self.parent is None

    @property
    def root_statements(self) -> models.QuerySet["Statement"]:
        return self.statements.filter(parent=None)

    @property
    def active_root_statements(self):
        return self.root_statements.filter(deleted_at__isnull=True, commented=False)

    def soft_delete(self):
        self.deleted_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        self.statements.filter(deleted_at=None).update(deleted_at=self.deleted_at)

    def restore(self):
        self.deleted_at = None
        self.statements.filter(deleted_at=self.deleted_at).update(deleted_at=None)

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
