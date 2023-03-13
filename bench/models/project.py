from __future__ import annotations

from datetime import datetime
from typing import TYPE_CHECKING, Optional, TypedDict
from uuid import UUID, uuid4

import pytz
from django.core.validators import validate_slug
from django.db import models, transaction
from django.db.models import Q
from django_choices_field import TextChoicesField
from strawberry_django_plus import gql

from bench.models.deployment import Deployment, DeploymentStatus, DeploymentType
from bench.models.statement import Statement
from bench.models.utils import UUIDModel, walk_children_bfs_batched
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
        create_onboarding_files: bool = False,
        create_blank_file: bool = False,
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
            # :SingleOwnedDeployment
            Deployment.objects.create_deployment(
                project_version=project.head,
                owner=owner,
                type=DeploymentType.ADHOC,
                status=DeploymentStatus.ACTIVE,
            )
        if create_onboarding_files:
            docs_v = Project.objects.get_by_slug("symbolx", "docs").head
            # :GettingStarted
            if not docs_v.files.filter(name="Getting Started").exists():
                raise ValueError(f"{docs_v} is missing Getting Started file")
            ProjectVersion.objects.copy_files(
                docs_v,
                project.head,
                docs_v.files.filter(name="Getting Started"),
                copy_mappings=False,
            )
        if create_blank_file:
            # create empty file
            project.head.create_path("Untitled")
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
        tag: Optional[str] = None,
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
            project=self, name=name, tag=tag, description=description
        )
        new_version.parents.add(assigned_parent)

        # copy project content from parent
        ref_mappings = ProjectVersion.objects.copy_files(assigned_parent, new_version)
        for mapping in ref_mappings:
            mapping.kind = RefMappingKind.COMMIT
        RefMapping.objects.bulk_create(ref_mappings)

        # copy owned deployments from parent
        for source_deployment in assigned_parent.deployments.filter(owned=True):
            target_deployment = Deployment.objects.copy(
                source_deployment, new_version, ref_mappings
            )
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

    def get_by_tag(self, project_id: UUID, tag: str):
        return self.filter(project_id=project_id, tag=tag).get()

    def get_between(self, source_version_id: UUID, target_version_id: UUID) -> list[ProjectVersion]:
        """
        Get every version between the source and target version (including both)
        Follow ProjectVersion.parents (not timestamps).
        TODO @Performance: implement get_between as recursive CTE query
        """
        source_version = self.only("id", "project_id").get(id=source_version_id)
        target_version = self.only("id", "project_id").get(id=target_version_id)
        if source_version.project_id != target_version.project_id:
            raise ValueError(
                f"source and target version must be from the same project: {source_version} {target_version}"
            )

        # start at target version and walk up to source version
        versions = [target_version]
        while versions[-1] != source_version:
            # this only works if there is one parent (no branching) :ProjectBranching
            first_parent = versions[-1].parents.only("id").first()
            if first_parent is None:
                raise ValueError(f"{target_version} is unreachable from {source_version}")
            versions.append(first_parent)
        versions.reverse()
        return versions

    def get_ancestors(self, version_id: UUID, depth: int = None) -> list[ProjectVersion]:
        """
        Get every version that is an ancestor of the given version (including the version itself)
        Follow ProjectVersion.parents (not timestamps).
        TODO @Performance: implement get_ancestors as recursive CTE query
        """
        version = self.only("id", "project_id").get(id=version_id)
        versions = [version]
        while True:
            # this only works if there is one parent (no branching) :ProjectBranching
            first_parent = versions[-1].parents.only("id").first()
            if first_parent is None:
                break  # reached root
            versions.append(first_parent)
            if depth and len(versions) >= depth:
                break
        versions.reverse()
        return versions

    def get_migration_mappings(
        self, source_version_id: UUID, target_version_id: UUID
    ) -> tuple[list[RefMapping], bool]:
        """
        Gets the final ref mappings between the source and target version.
        Follows ProjectVersion.parents (not timestamps).
        """

        # As an illustrating example, consider versions A, B, C, D, E (A -> E).
        # Each version contains the ref mappings to its parent(s) (e.g. B: A->B).
        #
        # Forward migrating B -> D:
        #  - intermediate versions C, D
        #
        # Backward migrating D -> B:
        #  - intermediate versions C, D
        #  - reverse

        source_version = self.get(id=source_version_id)
        target_version = self.get(id=target_version_id)

        is_reverse = source_version.created_at > target_version.created_at  # :ProjectBranching
        if is_reverse:
            # swap, then reverse at the end
            source_version_id, target_version_id = target_version_id, source_version_id

        intermediate_versions = ProjectVersion.objects.get_between(
            source_version_id, target_version_id
        )
        # skip first version (source version)
        intermediate_versions = intermediate_versions[1:]

        ref_mappings = list(RefMapping.objects.filter(target_version__in=intermediate_versions))

        # init with first mappings
        refs: dict[UUID, UUID] = {}
        refs_types: dict[UUID, RefType] = {}
        for ref in ref_mappings:
            if ref.source_version_id == source_version_id:
                refs[ref.source_id] = ref.target_id
                refs_types[ref.source_id] = ref.type

        # iterate through intermediate versions, updating target_id to each new target_id
        for version in intermediate_versions[1:]:
            reverse_refs = {v: k for k, v in refs.items()}
            for ref in ref_mappings:
                if ref.target_version_id == version.id:
                    # this source id is a current target id
                    source_id = reverse_refs.get(ref.source_id)
                    if source_id is not None:
                        # update target id
                        refs[source_id] = ref.target_id

        if is_reverse:
            refs = {v: k for k, v in refs.items()}

        final_ref_mappings = [
            RefMapping(
                id=None,
                kind=RefMappingKind.COMMIT,
                type=refs_types.get(source_id, refs_types.get(target_id)),
                source_version=source_version,
                target_version=target_version,
                source_id=source_id,
                source_revision=0,  # not tracked
                target_id=target_id,
                target_revision=0,  # not tracked
            )
            for source_id, target_id in refs.items()
        ]

        return final_ref_mappings, is_reverse

    def copy_files(
        self,
        source: ProjectVersion,
        target: ProjectVersion,
        files: Optional[models.QuerySet[File]] = None,
        copy_mappings: bool = True,
        target_files_ids: dict[UUID, UUID] = None,
    ) -> list["RefMapping"]:
        """Copies the given files from a source version to a target version (by default everything)"""

        # 0. select files & statements to copy
        if files is None:  # default to all files
            files = source.files.filter(deleted_at=None)
            statements = source.statements.filter(deleted_at=None)
        else:
            statements = Statement.objects.filter(file__in=files).filter(deleted_at=None)

        # TODO @Performance: copy project version server-side (in SQL)
        # TODO @Cleanup: created_at/updated_at are not copied correctly (auto-reset to now)
        #  (could set them manually in project mutation wrapper)
        # copy files
        new_files: dict[UUID, File] = {}
        file_mappings: list[RefMapping] = []
        target_files_ids = target_files_ids or {file.id: uuid4() for file in files}
        for files in walk_children_bfs_batched(files.filter(parent=None), "parent_id"):
            for file in files:
                old_id = file.id
                old_revision = file.revision
                file.id = target_files_ids[old_id]
                file._state.adding = True
                file.project_version = target
                file.parent = new_files.get(file.parent_id)
                new_files[old_id] = file
                file_mapping = RefMapping(
                    source_version=source,
                    target_version=target,
                    type=RefType.FILE,
                    source_id=old_id,
                    target_id=file.id,
                    source_revision=old_revision,
                    target_revision=file.revision,
                )
                file_mappings.append(file_mapping)
            File.objects.bulk_create(files)

        # copy statements
        statement_mappings = Statement.objects.copy_statements(
            statements, new_files, source, target, copy_mappings
        )
        return [*file_mappings, *statement_mappings]


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
    parent_refs: models.QuerySet["RefMapping"]  # noqa via RefMapping.source_version
    child_refs: models.QuerySet["RefMapping"]  # noqa via RefMapping.target_version
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


class RefType(models.TextChoices):
    FILE = "file", "File"
    STATEMENT = "statement", "Statement"
    RECORD = "record", "Record"
    TYPE_NODE = "type_node", "TypeNode"


class RefMappingKind(models.TextChoices):
    COMMIT = "commit", "Commit"
    PASTE = "paste", "Paste"


class RefMappingManager(models.Manager["RefMapping"]):
    def expand_target_ids(self, target_ids: list[UUID], depth: Optional[int] = None) -> list[UUID]:
        # TODO @Performance: implement symbol version id expansion in SQL
        expanded_ids = list(target_ids)
        last_symbol_ids = expanded_ids
        remaining_depth = depth
        while last_symbol_ids and (depth is None or remaining_depth > 0):
            remaining_depth -= 1
            last_symbol_ids = self.filter(target_id__in=last_symbol_ids).values_list(
                "source_id", flat=True
            )
            expanded_ids.extend(last_symbol_ids)
        return expanded_ids


class RefMapping(UUIDModel):
    """
    The mapping of a project content object's identity between locations/versions.
    There is no benefit to foreign constraints on the object ids here (?), so they're just UUIDs.
    Used to track lineage for versioning, forking, copy/paste, etc.

    This is similar to GeneratedMapping on the surface, but here we track object identities
    rather than statement-generated arbitrary mappings (different uses, constraints, etc.).
    """

    kind = TextChoicesField(choices_enum=RefMappingKind)
    source_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="child_refs"
    )
    target_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="parent_refs"
    )
    type = TextChoicesField(choices_enum=RefType)
    source_id = models.UUIDField()
    source_revision = models.IntegerField()
    target_id = models.UUIDField()
    target_revision = models.IntegerField()

    objects = RefMappingManager()

    class Meta:
        pass


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
