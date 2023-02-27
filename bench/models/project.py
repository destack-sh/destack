from __future__ import annotations

from collections import deque
from datetime import datetime
from itertools import groupby
from typing import TYPE_CHECKING, Deque, Iterator, Optional, TypedDict, TypeVar
from uuid import UUID, uuid4

import pytz
from django.core.validators import validate_slug
from django.db import models, transaction
from django.db.models import Q
from django_choices_field import TextChoicesField
from strawberry_django_plus import gql

from bench.language import SymbolType
from bench.models.data import DatasetRecord
from bench.models.deployment import Deployment, DeploymentStatus, DeploymentType
from bench.models.generated import SourceMapping
from bench.models.statement import SimpleTypeNode, Statement, StatementType
from bench.models.utils import UUIDModel
from bench.utils.uuidt import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH

if TYPE_CHECKING:
    from bench.models.organization import Organization
    from bench.models.user import User


class ProjectType(models.TextChoices):
    EXECUTABLE = "executable", "Executable"
    LIBRARY = "library", "Library"


class ProjectVisibility(models.TextChoices):
    PUBLIC = "public", "Public"
    SOURCE_PRIVATE = "source_private", "Source Private"
    PRIVATE = "private", "Private"


class ProjectManager(models.Manager["Project"]):
    @transaction.atomic
    def create_project(
        self,
        owner: User | Organization,
        name: str,
        slug: str,
        type: ProjectType = ProjectType.EXECUTABLE,
        visibility: ProjectVisibility = ProjectVisibility.PRIVATE,
        create_adhoc_deployment: bool = True,
    ):
        if owner.__class__.__name__ == "Organization":
            user = None
            organization = owner
        else:
            user = owner
            organization = None
        project = super().create(
            organization=organization,
            user=user,
            name=name,
            slug=slug,
            type=type,
            visibility=visibility,
        )
        project.head = ProjectVersion.objects.create(project=project)
        project.save()
        if create_adhoc_deployment:
            Deployment.objects.create_deployment(
                project_version=project.head, owner=owner, type=DeploymentType.ADHOC
            )
        return project

    def get_by_slug(self, owner: str, project: str):
        return (
            self.filter(slug=project)
            .filter(Q(organization__owner_slug_id=owner) | Q(user__owner_slug_id=owner))
            .get()
        )


RefDict = TypedDict("RefDict", {"source": str, "target": str, "type": str})


class Project(UUIDModel):
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
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, null=True)
    slug: models.SlugField = models.SlugField(max_length=128, validators=[validate_slug])
    visibility = TextChoicesField(choices_enum=ProjectVisibility, default=ProjectVisibility.PRIVATE)
    created_at: models.DateTimeField = models.DateTimeField(auto_now_add=True)
    updated_at: models.DateTimeField = models.DateTimeField(auto_now=True)

    # TODO @Feature: basic branching (per-head branch with head pointing to main head)
    head = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, null=True, related_name="project+"
    )
    organization: models.ForeignKey = models.ForeignKey(
        "Organization", on_delete=models.CASCADE, related_name="projects", null=True
    )
    user: models.ForeignKey = models.ForeignKey(
        "User", on_delete=models.CASCADE, related_name="projects", null=True
    )
    deployments: models.QuerySet["Deployment"]  # noqa via Deployment

    def __str__(self):
        return f"{self.owner.slug}/{self.slug}"

    @property
    def owner(self) -> Organization | User:
        return self.organization or self.user

    @gql.model_property(
        only=["user", "organization", "slug"], select_related=["user", "organization"]
    )
    def path(self) -> str:
        return f"{self.owner.slug}.{self.slug}"

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
        commit_tag: Optional[str] = None,
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
                assigned_parent.commit(commit_name, commit_tag, commit_description)
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

        # copy owned deployments from parent
        for source_deployment in assigned_parent.deployments.filter(owned=True):
            target_deployment = Deployment.objects.copy(source_deployment, new_version, refs)
            target_deployment.type = DeploymentType.ADHOC
            target_deployment.status = DeploymentStatus.INACTIVE  # reset status
            target_deployment.save()

        # advance head if it moved
        if assigned_parent == self.head:
            self.head = new_version
            self.save()

        return new_version

    objects: ProjectManager = ProjectManager()

    class Meta:
        default_related_name = "projects"
        constraints = [
            # unique slug per owner
            models.UniqueConstraint(
                name="bench_project_organization_slug_ak",
                fields=["organization", "slug"],
                condition=models.Q(organization__isnull=False),
            ),
            models.UniqueConstraint(
                name="bench_project_user_slug_ak",
                fields=["user", "slug"],
                condition=models.Q(user__isnull=False),
            ),
            # must have at least one owner (organization or user)
            models.CheckConstraint(
                name="bench_project_owner_ck",
                check=models.Q(organization__isnull=False) | models.Q(user__isnull=False),
            ),
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
    def get_by_slug(self, owner: str, project: str, tag: str):
        return (
            self.filter(tag=tag)
            .filter(project__slug=project)
            .filter(
                Q(project__organization__owner_slug_id=owner)
                | Q(project__user__owner_slug_id=owner)
            )
            .get()
        )

    def copy(
        self, source: ProjectVersion, target: ProjectVersion
    ) -> dict[UUID, File | Statement | DatasetRecord | SimpleTypeNode]:
        """Copies all project contents from a source version to a target version."""
        if not source.committed:
            raise ValueError(f"source version must be committed: {source}")
        # TODO @Performance: copy project version on commit server-side (in SQL)
        #  (generally good, but also especially for dataset records, mappings and other relations)
        # TODO @Cleanup: created_at/updated_at are not copied correctly (auto-reset to now)
        #  (could set them manually in project mutation wrapper)
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
        new_contents: dict[UUID, DatasetRecord | SimpleTypeNode] = {}
        new_mappings: list[SourceMapping] = []
        for statement in statements_bfs:
            # copy statement contents/relations
            if statement.type == StatementType.DEFINITION:
                # the relations are saved below after statement creation
                if statement.root_type_tag is not None:
                    for type_node in statement.type_nodes.all():
                        old_id = type_node.id
                        type_node.pk = None
                        type_node.statement_id = new_statements_ids[statement.id]
                        if type_node.reference_id is not None:
                            # replace type node reference if it was copied (default to same for externals)
                            type_node.reference_id = new_statements_ids.get(
                                type_node.reference_id, type_node.reference_id
                            )
                        new_contents[old_id] = type_node
                if statement.symbol_type == SymbolType.DATA:
                    for record in statement.records.all():
                        old_id = record.id
                        record.pk = None
                        record.statement_id = new_statements_ids[statement.id]
                        new_contents[old_id] = record
                elif statement.symbol_type == SymbolType.BUILD:
                    for mapping in statement.generated_mappings.all():
                        mapping.pk = None
                        mapping.statement_id = new_statements_ids[mapping.statement_id]
                        mapping.source_id = new_statements_ids[mapping.source_id]
                        mapping.target_id = new_statements_ids[mapping.target_id]
                        mapping.source_revision = 0
                        mapping.target_revision = 0
                        new_mappings.append(mapping)

            # copy statement
            # automatically copies all non-relational columns
            old_id = statement.id
            statement.pk = new_statements_ids[old_id]
            statement._state.adding = True
            statement.revision = 0  # reset revision
            statement.file = new_files[statement.file_id]
            statement.project_version = target
            statement.reference = None
            statement.parent = new_statements.get(statement.parent_id)
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

        # 4. save statement's relations
        for relation_cls, relations in groupby(new_contents.values(), key=type):
            relation_cls.objects.bulk_create(relations)
        SourceMapping.objects.bulk_create(new_mappings)

        return {**new_statements, **new_files, **new_contents}


class ProjectVersion(UUIDModel):
    """
    A project version records the state of a project at a specific point in time.
    """

    project = models.ForeignKey(Project, on_delete=models.CASCADE, related_name="versions")
    name = models.CharField(max_length=MAX_NAME_LENGTH, null=True)
    # single unique tag should be a ProjectVersionTag list later :ProjectVersionTags
    tag = models.CharField(max_length=MAX_NAME_LENGTH, null=True)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, null=True)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    committed_at = models.DateTimeField(null=True)

    parents = models.ManyToManyField(
        "ProjectVersion", related_name="children", symmetrical=False, blank=True
    )
    parents_refs = models.JSONField(default=dict)
    files: models.QuerySet["File"]  # noqa via File
    statements: models.QuerySet["Statement"]  # noqa via Statement
    deployments: models.QuerySet["Deployment"]  # noqa via Deployment

    def __str__(self) -> str:
        return f"{self.project.path}@{self.id.hex}"

    def reset(self):
        """Hard deletes all files (cascades to statements and their contents)."""
        self.files.all().delete()

    @transaction.atomic
    def commit(
        self,
        name: Optional[str] = None,
        tag: Optional[str] = None,
        description: Optional[str] = None,
    ):
        if self.committed:
            raise ValueError(f"already committed: {self}")
        self.committed_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        if name is not None:
            self.name = name
        if tag is not None:
            self.tag = tag  # :ProjectVersionTags
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
        constraints = [
            # tag is unique per project
            models.UniqueConstraint(
                fields=["project", "tag"],
                name="bench_project_version_tag_ak",
            ),
        ]


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
    directory = models.BooleanField(default=False)
    generated = models.BooleanField(default=False)
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
        Statement._base_manager.filter(file=self, deleted_at=self.deleted_at).update(
            deleted_at=None
        )
        self.deleted_at = None

    objects = FileManager()

    class Meta:
        ordering = ["name"]
        constraints = [
            # ensure that the path is unique per project version (includes parent directory)
            models.UniqueConstraint(
                name="bench_project_file_project_name_ak",
                fields=["project_version_id", "name"],
                condition=models.Q(parent_id__isnull=True, deleted_at__isnull=True),
            ),
            models.UniqueConstraint(
                name="bench_project_file_project_parent_name_ak",
                fields=["project_version_id", "parent_id", "name"],
                condition=models.Q(parent_id__isnull=False, deleted_at__isnull=True),
            ),
        ]
