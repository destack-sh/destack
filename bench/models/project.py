from __future__ import annotations

import collections
import os
import uuid
from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID, uuid4

import structlog
from django.core.validators import validate_slug
from django.db import models, transaction
from django.db.models import Q
from django.db.models.expressions import RawSQL
from pgcrypto.fields import TextPGPSymmetricKeyField
from strawberry_django.descriptors import model_property

from bench.language import wire
from bench.language.const import StatementType, new_dynamic_node_key
from bench.language.validation import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH
from bench.models.object import get_s3_client
from bench.models.statement import Statement
from bench.models.utils import CrudModel, CrudNode, ModuleNode, UUIDModel, create_models_bfs
from bench.search.core import IndexType
from bench.settings import GLOBAL_PROJECT_BUCKET_NAME, LOCAL
from bench.utils.dt import utcnow_with_tz
from bench.utils.func import generate_random_name, generate_secret_password
from bench.utils.utils import DEBUG

if TYPE_CHECKING:
    from bench.models.organization import Organization
    from bench.models.packer import PackFilter, _PackedCopy
    from bench.models.user import User

logger = structlog.get_logger(__name__)


class ProjectVisibility(models.TextChoices):
    PUBLIC = "public", "Public"
    SOURCE_PRIVATE = "source_private", "Source Private"
    PRIVATE = "private", "Private"


class ModuleAccessLevel(models.IntegerChoices):  # :ModuleAccessLevel
    Zero = 0  # no access
    Read = 1  # can view and comment
    Use = 4  # can run
    Edit = 8  # can edit, view secrets
    Manage = 12  # can manage members
    Admin = 16  # deletion-protection, destructive actions, manage admins


_PROJECT_AUTH_COLUMNS = ("db_username", "db_password", "os_username", "os_password")


class ProjectManager(models.Manager["Project"]):
    def get_queryset(self):
        return super().get_queryset().defer(*_PROJECT_AUTH_COLUMNS)

    @transaction.atomic
    def create_project(
        self,
        owner: User | Organization,
        name: str,
        slug: str,
        id: UUID = None,
        visibility: ProjectVisibility = ProjectVisibility.PRIVATE,
        create_onboarding_files: bool = False,
        create_infra: bool = True,
        head_version_id: Optional[UUID] = None,
    ):
        if owner.__class__.__name__ == "Organization":
            user = None
            organization = owner
        else:
            user = owner
            organization = None
        id = id or uuid4()
        project = super().create(
            id=id,
            organization=organization,
            user=user,
            name=name,
            slug=slug,
            visibility=visibility,
            os_name=f"bench-user-{id}-local",
        )
        # initial version head
        project.head = ProjectVersion.objects.create(
            id=head_version_id or uuid4(), ck=project.id, project=project
        )
        project.save()

        # onboarding
        if create_onboarding_files:
            # TODO @UX: re-implement create onboarding files
            pass

        # infra
        if create_infra:
            from bench.server.search import create_local_search_index

            create_local_worker_set(project, upsert=False)
            create_local_search_index(project.id, project.os_name, upsert=False)

        return project

    def get_by_slug(self, owner: str, project: str):
        return (
            self.filter(slug=project)
            .filter(Q(organization__owner_slug_id=owner) | Q(user__owner_slug_id=owner))
            .get()
        )


class Project(UUIDModel, CrudModel):
    """
    A project == a Bench.

    Projects are the root of versioning, similar to repositories in Git.
    All versions are available in 'versions' and may not be linear (also like in Git).
    """

    name: models.CharField = models.CharField(max_length=MAX_NAME_LENGTH)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, null=True)
    slug: models.SlugField = models.SlugField(max_length=128, validators=[validate_slug])

    visibility = models.CharField(
        max_length=32, choices=ProjectVisibility.choices, default=ProjectVisibility.PRIVATE
    )
    base_level = models.IntegerField(default=ModuleAccessLevel.Read)
    sharing_enabled = models.BooleanField(default=True)
    sharing_token = models.UUIDField(default=uuid4)
    sharing_level = models.IntegerField(default=ModuleAccessLevel.Read)
    organization: models.ForeignKey = models.ForeignKey(
        "Organization", on_delete=models.CASCADE, related_name="projects", null=True
    )
    user: models.ForeignKey = models.ForeignKey(
        "User", on_delete=models.CASCADE, related_name="projects", null=True
    )
    members = models.ManyToManyField("User", through="ProjectMembership", related_name="projects+")
    memberships: models.QuerySet["ProjectMembership"]  # noqa via ProjectMembership.project

    head = models.ForeignKey(
        "ProjectVersion", on_delete=models.SET_NULL, null=True, related_name="project+"
    )
    blobs: models.QuerySet["Blob"]  # noqa via Blob
    worker_set = models.OneToOneField(  # only one worker set for now
        "WorkerSet", on_delete=models.SET_NULL, related_name="project+", null=True
    )
    worker_sets: models.QuerySet["WorkerSet"]  # noqa via WorkerSet
    db_name = models.CharField(max_length=64, default=generate_random_name)
    db_username = models.CharField(max_length=64, default=generate_random_name)
    db_password = TextPGPSymmetricKeyField(default=generate_secret_password)
    os_name = models.CharField(max_length=64)
    os_username = models.CharField(max_length=64, default=generate_random_name)
    os_password = TextPGPSymmetricKeyField(default=generate_secret_password)

    def __str__(self):
        return f"{self.owner.slug}/{self.slug}"

    @property
    def owner(self) -> Organization | User:
        return self.organization or self.user

    @property
    def owner_id(self) -> UUID:
        return self.organization_id or self.user_id

    @model_property(only=["user", "organization", "slug"], select_related=["user", "organization"])
    def path(self) -> str:
        return f"{self.owner.slug}.{self.slug}"

    @property
    def head_(self) -> ProjectVersion:
        if self.head is None:
            raise ValueError(f"project {self} has no head")
        return self.head

    @transaction.atomic(savepoint=False)
    def rename(self, name: str, slug: str):
        self.name = name
        self.slug = slug
        self.save()

    @transaction.atomic(savepoint=False)
    def create_new_blank_head(
        self,
        name: Optional[str] = None,
        tag: Optional[str] = None,
        description: Optional[str] = None,
        parent: Optional[ProjectVersion] = None,
    ) -> "ProjectVersion":
        if parent is None:
            if self.head is None:
                raise ValueError(f"project does not have a head: {self}")
            assigned_parent = self.head
        else:
            assigned_parent = parent
        del parent  # avoid accidental use
        if not assigned_parent.committed:
            raise ValueError(f"parent must be committed: {assigned_parent}")

        new_version = ProjectVersion.objects.create(
            project=self, name=name, tag=tag, description=description
        )
        new_version.parents.add(assigned_parent)
        self.head = new_version
        self.save()

        return new_version

    @transaction.atomic(savepoint=False)
    def create_invite(
        self, email: str, level: "ModuleAccessLevel", message: str = None, created_by: User = None
    ) -> "ProjectInvite":
        from bench.models.notification import Notification, NotificationType
        from bench.models.user import User

        user = User.objects.filter(email=email).first()
        if user is not None and self.members.filter(id=user.id).exists():
            raise ValueError("user already a member of organization")

        invite = ProjectInvite.objects.create(
            project=self,
            email=email,
            level=level,
            message=message,
            created_by=created_by,
            user=user,
        )

        # create notification if the user is signed up
        if user is not None:
            Notification.objects.create(
                type=NotificationType.ORGANIZATION_INVITE,
                user=user,
                invite=invite,
            )

        return invite

    @transaction.atomic(savepoint=False)
    def accept_invite(self, invite: "ProjectInvite") -> None:
        if invite.user is None:
            raise ValueError("cannot accept invite without registered user")
        invite.project.add_member(invite.user, invite.level)
        invite.delete()

    def add_member(self, user: User, level: "ModuleAccessLevel") -> None:
        if self.members.filter(id=user.id).exists():
            raise ValueError("user already a member of project")
        ProjectMembership.objects.create(project=self, user=user, level=level)

    objects: ProjectManager = ProjectManager()

    class Meta:
        base_manager_name = "objects"
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
            # unique sharing token
            models.UniqueConstraint(
                name="bench_project_sharing_token_ak", fields=["sharing_token"]
            ),
            # must have at least one owner (organization or user)
            models.CheckConstraint(
                name="bench_project_owner_ck",
                check=models.Q(organization__isnull=False) | models.Q(user__isnull=False),
            ),
        ]


class ProjectMembership(UUIDModel):
    project: models.ForeignKey = models.ForeignKey(
        Project, on_delete=models.CASCADE, related_name="memberships"
    )
    user: models.ForeignKey = models.ForeignKey(
        "User", on_delete=models.CASCADE, related_name="project_memberships"
    )
    level = models.IntegerField(choices=ModuleAccessLevel.choices)

    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)

    def __str__(self):
        return f"{self.project} -> {self.user} ({self.level})"

    def __repr__(self):
        return f"<ProjectMembership {self}>"

    class Meta:
        ordering = ["-created_at"]
        constraints = [
            models.UniqueConstraint(name="bench_project_membership_ak", fields=["project", "user"])
        ]


class ProjectInvite(UUIDModel):
    """
    An invitation to join a project (for existing or not yet existing users).
    """

    project = models.ForeignKey("Project", on_delete=models.CASCADE, related_name="invites")
    email = models.EmailField()
    user = models.ForeignKey(
        "User", on_delete=models.CASCADE, related_name="project_invites", null=True
    )
    level = models.IntegerField(choices=ModuleAccessLevel.choices)
    message = models.TextField(blank=True, null=True)
    email_sent_at = models.DateTimeField(blank=True, null=True)

    created_by = models.ForeignKey("User", on_delete=models.CASCADE, related_name="+")
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)

    def __str__(self):
        return f"{self.project} -> {self.email} ({self.level})"

    def __repr__(self):
        return f"<ProjectInvite {self}>"

    def accept(self):
        self.project.accept_invite(self)

    class Meta:
        ordering = ["-created_at"]
        constraints = [
            models.UniqueConstraint(name="bench_project_invite_ak", fields=["project", "email"])
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
                f"source and target version must be from the same project: {source_version} </> {target_version}"
            )

        # try both directions
        versions = self.get_between_unidirectional(source_version, target_version)
        if versions is not None:
            return versions
        versions = self.get_between_unidirectional(target_version, source_version)
        if versions is not None:
            return versions
        raise ValueError(
            f"source and target version are not connected: {source_version} </> {target_version}"
        )

    def get_between_unidirectional(
        self, source_version: ProjectVersion, target_version: ProjectVersion
    ) -> list[ProjectVersion] | None:
        # start at target version and walk up to source version
        versions = [target_version]
        while versions[-1] != source_version:
            # this only works if there is one parent (no branching) :ProjectBranching
            first_parent = versions[-1].parents.only("id").first()
            if first_parent is None:
                return None
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

    def pack_copy(
        self,
        source: ProjectVersion,
        target: ProjectVersion,
        nodes: list[models.Model],
        keep_cks: bool,
        excluded: set[type[ModuleNode]],
        target_ids: dict[UUID, UUID] = None,
        target_cks: dict[UUID, UUID] = None,
        copy_revisions: bool = True,
        filter: PackFilter = None,
    ) -> _PackedCopy:
        """Packs a copy of the module tree starting at the given nodes."""
        from bench.models import packer

        if target_ids and len(set(target_ids.values())) != len(target_ids):
            raise ValueError(f"target ids must be unique: {target_ids}")
        if target_cks and len(set(target_cks.values())) != len(target_cks):
            raise ValueError(f"target cks must be unique: {target_cks}")

        target_ids = {**(target_ids or {}), source.id: target.id}
        target_ids_reversed = {target.id: source.id}
        target_cks = {**(target_cks or {}), source.ck: target.ck}
        target_cks_reversed = {target.ck: source.ck}
        target_keys = {}
        packed = packer.pack_node(
            *nodes, filter=filter or packer.DEFAULT_PACK_FILTER, excluded=excluded
        )

        # map all identities to new identities (id, ck, key)
        for node in packed.nodes_by_id.values():
            if (node.id in target_ids) != (node.ck in target_cks):
                raise ValueError(
                    f"node id and ck must be set together: {repr(node)}"
                    f" (id:{node.id}:{node.id in target_ids}, ck:{node.ck}:{node.ck in target_cks})"
                )
            if node.id not in target_ids:
                target_cks[node.ck] = uuid4() if not keep_cks else node.ck
                target_ids[node.id] = uuid.uuid5(target.id, str(target_cks[node.ck]))
            target_ids_reversed[target_ids[node.id]] = node.id
            target_cks_reversed[target_cks[node.ck]] = node.ck
            node.id = target_ids[node.id]
            node.ck = target_cks[node.ck]
            if isinstance(node, wire.HasCrud) and not copy_revisions:
                node.revision = 0
            # dynamic key is used to attach records to databases
            # so if it's copied and the database is versioned, we need to update the key
            if (
                isinstance(node, wire.StatementData)
                and node.type == StatementType.DATABASE
                and node.versioned
            ):
                source_key = node.key
                node.key = new_dynamic_node_key(node.id)
                target_keys[source_key] = node.key

        # patch parents & references
        for node in packed.nodes_by_id.values():
            node.parent_id = target_ids.get(node.parent_id, node.parent_id)
            wire.patch_node_flat(node, target_cks, target_keys)

        # sanity check target cks
        if DEBUG or LOCAL:
            nodes_by_ck = collections.defaultdict(list)
            for node in packed.nodes_by_id.values():
                nodes_by_ck[node.ck].append(node)
            if len(nodes_by_ck) != len(packed.nodes_by_id):
                duplicates = {ck: nodes for ck, nodes in nodes_by_ck.items() if len(nodes) > 1}
                raise ValueError(
                    f"target cks are not unique: {len(packed.nodes_by_id)} != {len(nodes_by_ck)}:\n{duplicates}"
                )

        return packer._PackedCopy(
            roots=packed.roots,
            nodes_by_id=packed.nodes_by_id,
            target_ids=target_ids,
            target_ids_reversed=target_ids_reversed,
            target_cks=target_cks,
            target_cks_reversed=target_cks_reversed,
        )

    def copy(
        self,
        source: ProjectVersion,
        target: ProjectVersion,
        files: Optional[models.QuerySet[File] | list[File]] = None,
        target_ids: dict[UUID, UUID] = None,
        keep_cks: bool = True,
        include_interp: bool = True,
        copy_revisions: bool = True,
    ) -> None:
        """Copies the given files from a source version to a target version (by default everything)"""

        from bench.models import packer
        from bench.server.search import write_module_to_os

        # pack relevant nodes
        filter = packer.DEFAULT_PACK_FILTER.extend()
        if files is not None:
            if isinstance(files, list):
                filter.filter(File, lambda qs: qs.filter(id__in=[f.id for f in files]))
            else:
                filter.filter(File, lambda qs: qs.filter(id__in=files.values_list("id", flat=True)))
        copy = self.pack_copy(
            source=source,
            target=target,
            nodes=[source],
            target_ids=target_ids,
            keep_cks=keep_cks,
            excluded=packer.INTERP_MODEL_TYPES if not include_interp else set(),
            copy_revisions=copy_revisions,
            filter=filter,
        )

        # unpack and save
        unpacked = packer.unpack_nodes_tree(copy.nodes_list(), pre_unpacked={target.id: target})
        create_models_bfs(unpacked.walk_bfs_batched(), exclude={target.id})
        write_module_to_os(target, unpacked.walk_bfs(), wipe=True)


class ProjectVersion(CrudNode):
    """
    A project version records the state of a project at a specific point in time.
    """

    project = models.ForeignKey(Project, on_delete=models.CASCADE, related_name="versions")
    name = models.CharField(max_length=MAX_NAME_LENGTH, null=True)
    # single unique tag should be a ProjectVersionTag list later :ProjectVersionTags
    tag = models.CharField(max_length=MAX_NAME_LENGTH, null=True)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, null=True)
    committed_at = models.DateTimeField(null=True)

    parents = models.ManyToManyField("ProjectVersion", related_name="children", symmetrical=False)
    files: models.QuerySet["File"]  # noqa via File
    statements: models.QuerySet["Statement"]  # noqa via Statement
    sessions: models.QuerySet["Session"]  # noqa via Session
    runs: models.QuerySet["Run"]  # noqa via Run

    def __str__(self) -> str:
        return f"{self.project.path}@{self.tag or self.id.hex}"

    @property
    def parent_id(self) -> Optional[uuid.UUID]:
        return None

    @property
    def parent(self) -> Optional["ModuleNode"]:
        return None

    def commit(self):
        if self.committed:
            raise ValueError(f"already committed: {self}")
        self.committed_at = utcnow_with_tz()
        self.save()

    @model_property(only=["committed_at"])
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
                project_version=self, parent=parent, name=directory, directory=True
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

    def copy(
        self,
        file: "File",
        source: ProjectVersion,
        target: ProjectVersion,
        target_id: UUID,
        target_ck: UUID,
        keep_cks: bool,
        include_interp: bool = True,
        target_parent: Optional["File"] = None,
        copy_revisions: bool = False,
    ) -> "File":
        """Copies a file from one module to another (may be the same)."""
        from bench.models import packer
        from bench.server.search import write_module_to_os

        target_id = target_id or uuid.uuid4()
        # pack relevant nodes
        copy = ProjectVersion.objects.pack_copy(
            source=source,
            target=target,
            nodes=[file],
            keep_cks=keep_cks,
            excluded=packer.INTERP_MODEL_TYPES if not include_interp else set(),
            copy_revisions=copy_revisions,
            target_ids={file.id: target_id},
            target_cks={file.ck: target_ck},
        )
        assert len(copy.roots) == 1, "expected exactly one root in packed nodes"
        copy.roots[0].parent_id = target_parent.id if target_parent else target.id

        # unpack and save
        unpacked = packer.unpack_nodes_tree(copy.nodes_list(), pre_unpacked={target.id: target})
        create_models_bfs(unpacked.walk_bfs_batched())
        write_module_to_os(target, unpacked.walk_bfs(), wipe=False)

        target_file = unpacked.nodes_by_id[target_id]
        return target_file

    def get_descendants(
        self, file_ids: list[UUID], deleted_at: Optional[datetime] = None
    ) -> models.QuerySet[File]:
        """Gets descendants of files with given ids (including the files themselves)."""
        query = """
           WITH RECURSIVE descendants(id, parent_file_id) AS (
               SELECT id, parent_file_id
               FROM bench_file
               WHERE id = ANY(%s)
               UNION ALL
               SELECT bench_file.id, bench_file.parent_file_id
               FROM bench_file
               INNER JOIN descendants ON descendants.id = bench_file.parent_file_id
           )
           SELECT DISTINCT id
           FROM descendants
        """
        return File._base_manager.filter(id__in=RawSQL(query, (file_ids,)), deleted_at=deleted_at)


class File(CrudNode):
    """
    A file containing statements, potentially containing other files if it's a directory.
    A file - and the statements it contains - may be soft-deleted.
    Nothing is actually deleted, but soft deleted objects are not visible and not copied across versions.
    """

    project_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="files"
    )
    name: models.CharField = models.CharField(max_length=MAX_NAME_LENGTH, blank=True)
    parent_file = models.ForeignKey(
        "File", on_delete=models.CASCADE, null=True, blank=True, related_name="files"
    )

    files: models.QuerySet["File"]  # noqa via File.parent (if it's a directory)
    statements: models.QuerySet["Statement"]  # noqa via Statement.file
    symbols: models.QuerySet["Symbol"]  # noqa via Symbol.file

    def __str__(self):
        if self.parent_file:
            return f"{self.parent_file}/{self.name}"
        else:
            return f"{self.project_version}/{self.name}"

    @model_property(only=["name", "parent"], select_related=["parent"])
    def path(self) -> str:
        return f"{self.parent_file.path}/{self.name}" if self.parent_file else f"{self.name}"

    @property
    def parent_id(self) -> Optional[uuid.UUID]:
        return self.parent_file_id or self.project_version_id

    @property
    def parent(self) -> Union["File", "ProjectVersion"]:
        if self.parent_file_id:
            return self.parent_file
        else:
            return self.project_version

    def is_root(self) -> bool:
        return self.parent_file is None

    @property
    def root_statements(self) -> models.QuerySet["Statement"]:
        return self.statements.filter(parent=None)

    def soft_delete(self):
        self.deleted_at = utcnow_with_tz()
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
            # ck is unique per project version
            models.UniqueConstraint(
                fields=["project_version", "ck"],
                name="bench_file_project_version_ck",
                condition=models.Q(deleted_at__isnull=True),
            ),
        ]


def create_global_user_bucket(ignore_exists: bool):
    """
    Creates a public S3 bucket for all projects.
    """
    s3_client = get_s3_client()
    try:
        response = s3_client.create_bucket(
            Bucket=GLOBAL_PROJECT_BUCKET_NAME,
            CreateBucketConfiguration={"LocationConstraint": os.environ["AWS_REGION"]},
        )
    except s3_client.exceptions.BucketAlreadyOwnedByYou:
        if ignore_exists:
            return
        raise
    code = response["ResponseMetadata"]["HTTPStatusCode"]
    if code != 200:
        raise RuntimeError(f"failed to create s3 bucket: {response}")
    if not LOCAL:
        # enable cors
        response = s3_client.put_bucket_cors(
            Bucket=GLOBAL_PROJECT_BUCKET_NAME,
            CORSConfiguration={
                "CORSRules": [
                    {
                        "AllowedHeaders": ["*"],
                        "AllowedMethods": ["GET", "PUT", "POST", "DELETE"],
                        "AllowedOrigins": ["*"],
                        "ExposeHeaders": ["ETag"],
                        "MaxAgeSeconds": 3000,
                    }
                ]
            },
        )
        if response["ResponseMetadata"]["HTTPStatusCode"] != 200:
            raise RuntimeError(f"failed to set cors on s3 bucket: {response}")
        # set encryption
        response = s3_client.put_bucket_encryption(
            Bucket=GLOBAL_PROJECT_BUCKET_NAME,
            ServerSideEncryptionConfiguration={
                "Rules": [
                    {
                        "ApplyServerSideEncryptionByDefault": {
                            "SSEAlgorithm": "AES256"  # Use AES256 encryption
                        }
                    }
                ]
            },
        )
        if response["ResponseMetadata"]["HTTPStatusCode"] != 200:
            raise RuntimeError(f"failed to set encryption on s3 bucket: {response}")


def create_local_worker_set(project: Project, *, upsert: bool):
    from bench.language.const import ProjectRegion, WorkerProfile, WorkerSetStatus
    from bench.models import WorkerSet

    # check if worker set already exists
    if WorkerSet.objects.filter(project_id=project.id).exists():
        if not upsert:
            raise ValueError(f"worker set already exists for project: {project}")
        return

    worker_set = WorkerSet.objects.create(
        project_id=project.id,
        region=ProjectRegion.EU_CENTRAL,
        profile=WorkerProfile.TINY,
        sleeping=True,
        desired_replicas=1,
        target_replicas=1,
        status=WorkerSetStatus.SLEEPING,
    )
    project.worker_set = worker_set
    project.save()
