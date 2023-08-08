from __future__ import annotations

import os
import uuid
from datetime import datetime
from typing import TYPE_CHECKING, Optional, TypedDict, Union
from uuid import UUID, uuid4

import pytz
import structlog
from django.core.validators import validate_slug
from django.db import models, transaction
from django.db.models import Q
from django.db.models.expressions import RawSQL
from strawberry_django_plus import gql

from bench.language import wire
from bench.language.wire import MOT_BY_DATA_CLASS
from bench.models.object import get_s3_client
from bench.models.statement import Statement
from bench.models.utils import CrudModel, ModuleNode, Revisioned, UUIDModel, create_models_bfs
from bench.settings import LOCAL, PROJECT_BUCKET_NAME
from bench.utils.uuidt import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH

if TYPE_CHECKING:
    from bench.models.organization import Organization
    from bench.models.packer import Packed, PackFilter
    from bench.models.user import User

logger = structlog.get_logger(__name__)


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
        visibility: ProjectVisibility = ProjectVisibility.PRIVATE,
        create_onboarding_files: bool = False,
        create_worker_set: bool = True,
        create_os_index: bool = True,
        head_version_id: Optional[UUID] = None,
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
            visibility=visibility,
        )
        project.head = ProjectVersion.objects.create(id=head_version_id, project=project)
        if create_onboarding_files:
            # TODO @Broken: re-implement create onboarding files
            pass
        if create_worker_set:
            create_default_worker_set(project)
        if create_os_index:
            create_per_project_os_index(project)
        project.save()
        return project

    def get_by_slug(self, owner: str, project: str):
        return (
            self.filter(slug=project)
            .filter(Q(organization__owner_slug_id=owner) | Q(user__owner_slug_id=owner))
            .get()
        )


RefDict = TypedDict("RefDict", {"source": str, "target": str, "type": str})


class Project(UUIDModel, CrudModel):
    """
    A project to instruct beautiful bots..

    Projects are the root of versioning, similar to repositories in Git.
    All versions are available in 'versions' and may not be linear (also like in Git).
    """

    name: models.CharField = models.CharField(max_length=MAX_NAME_LENGTH)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, null=True)
    slug: models.SlugField = models.SlugField(max_length=128, validators=[validate_slug])
    visibility = models.CharField(
        max_length=32, choices=ProjectVisibility.choices, default=ProjectVisibility.PRIVATE
    )

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
    remote_objects: models.QuerySet["RemoteObject"]  # noqa via RemoteObject
    worker_set = models.OneToOneField(  # only one worker set for now
        "WorkerSet", on_delete=models.CASCADE, related_name="project+", null=True
    )
    worker_sets: models.QuerySet["WorkerSet"]  # noqa via WorkerSet

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


def create_global_project_s3_bucket():
    """
    Creates a public S3 bucket for all projects.
    """
    s3_client = get_s3_client()
    response = s3_client.create_bucket(
        Bucket=PROJECT_BUCKET_NAME,
        CreateBucketConfiguration={"LocationConstraint": os.environ["AWS_REGION"]},
    )
    if response["ResponseMetadata"]["HTTPStatusCode"] != 200:
        raise RuntimeError(f"failed to create s3 bucket: {response}")
    if not LOCAL:
        # enable cors
        response = s3_client.put_bucket_cors(
            Bucket=PROJECT_BUCKET_NAME,
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
            Bucket=PROJECT_BUCKET_NAME,
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


def create_per_project_os_index(project: Project):
    """Creates OpenSearch indices for the project."""
    from bench.opensearch.index import create_bench_index

    create_bench_index(project.id)


def create_default_worker_set(project: Project):
    from bench.models import WorkerProfile, WorkerRegion, WorkerSet, WorkerSetStatus

    worker_set = WorkerSet.objects.create(
        project_id=project.id,
        region=WorkerRegion.EU_CENTRAL,
        profile=WorkerProfile.TINY,
        sleeping=True,
        desired_replicas=1,
        target_replicas=1,
        status=WorkerSetStatus.SLEEPING,
    )
    project.worker_set = worker_set
    project.save()


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

        intermediate_versions = ProjectVersion.objects.get_between(
            source_version_id, target_version_id
        )
        is_reverse = intermediate_versions[0].id != source_version_id
        # skip first version (source version)
        intermediate_versions = intermediate_versions[1:]

        ref_mappings = list(RefMapping.objects.filter(target_version__in=intermediate_versions))

        # init with first mappings
        refs: dict[UUID, UUID] = {}
        refs_types: dict[UUID, str] = {}
        for ref in ref_mappings:
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

    def pack_copy(
        self,
        source: ProjectVersion,
        target: ProjectVersion,
        nodes: list[models.Model],
        target_ids: dict[UUID, UUID] = None,
        copy_revisions: bool = True,
        kind: RefMappingKind = None,
        filter: PackFilter = None,
    ) -> tuple[Packed, list["RefMapping"], dict[UUID, UUID]]:
        """Packs a copy of the module tree at the given nodes."""
        from bench.models import packer

        target_ids = {**(target_ids or {}), source.id: target.id}
        kind = kind or RefMappingKind.COMMIT
        packed = packer.pack_node(*nodes, filter=filter or packer.DEFAULT_PACK_FILTER)

        # map all ids to new ids
        ref_mappings: dict[UUID, RefMapping] = {}
        for node in packed.nodes.values():
            source_id = node.id
            if node.id not in target_ids:
                target_ids[node.id] = uuid4()
            node.id = target_ids[node.id]
            if not isinstance(node, wire.HasCrud):
                continue
            source_revision = node.revision
            if not copy_revisions:
                node.revision = 0
            ref_mappings[node.id] = RefMapping(
                source_version=source,
                target_version=target,
                source_id=source_id,
                source_revision=source_revision,
                target_id=node.id,
                target_revision=node.revision,
                type=MOT_BY_DATA_CLASS[type(node)],
                kind=kind,
            )
        for node in packed.nodes.values():  # patch parent ids
            node.parent_id = target_ids.get(node.parent_id, node.parent_id)
            wire.patch_node_flat(node, target_ids)
        return packed, list(ref_mappings.values()), target_ids

    def copy(
        self,
        source: ProjectVersion,
        target: ProjectVersion,
        files: Optional[models.QuerySet[File]] = None,
        target_ids: dict[UUID, UUID] = None,
        invert_mappings: bool = False,
        copy_revisions: bool = True,
        kind: RefMappingKind = None,
    ) -> list[RefMapping]:
        """Copies the given files from a source version to a target version (by default everything)"""

        from bench.models import Dataset, packer

        # pack  relevant nodes
        filter = packer.DEFAULT_PACK_FILTER.extend()
        if files is not None:
            filter.filter(File, lambda qs: qs.filter(id__in=files))
        packed, mappings, target_ids = self.pack_copy(
            source=source,
            target=target,
            nodes=[source],
            target_ids=target_ids,
            copy_revisions=copy_revisions,
            kind=kind or RefMappingKind.COMMIT,
            filter=filter,
        )

        # unpack and save
        unpacked = packer.unpack_nodes_tree(packed.nodes_list(), pre_unpacked={target.id: target})
        create_models_bfs(unpacked.walk_bfs_batched(), exclude={target.id})
        # duplicate versioned datasets
        versioned_datasets = [
            n for n in unpacked.nodes.values() if isinstance(n, Dataset) and n.versioned
        ]
        Statement.objects.duplicate_datasets_inplace(source, target, versioned_datasets)
        # save ref mappings
        if invert_mappings:
            for mapping in mappings:
                mapping.source_id, mapping.target_id = mapping.target_id, mapping.source_id
                mapping.source_version, mapping.target_version = (
                    mapping.target_version,
                    mapping.source_version,
                )
        RefMapping.objects.bulk_create(mappings)

        return mappings


class ProjectVersion(UUIDModel, CrudModel, ModuleNode):
    """
    A project version records the state of a project at a specific point in time.
    """

    project = models.ForeignKey(Project, on_delete=models.CASCADE, related_name="versions")
    name = models.CharField(max_length=MAX_NAME_LENGTH, null=True)
    # single unique tag should be a ProjectVersionTag list later :ProjectVersionTags
    tag = models.CharField(max_length=MAX_NAME_LENGTH, null=True)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, null=True)
    committed_at = models.DateTimeField(null=True)

    parents = models.ManyToManyField(
        "ProjectVersion", related_name="children", symmetrical=False, blank=True
    )
    parent_refs: models.QuerySet["RefMapping"]  # noqa via RefMapping.source_version
    child_refs: models.QuerySet["RefMapping"]  # noqa via RefMapping.target_version
    files: models.QuerySet["File"]  # noqa via File
    statements: models.QuerySet["Statement"]  # noqa via Statement

    def __str__(self) -> str:
        return f"{self.project.path}@{self.tag or self.id.hex}"

    @property
    def parent_id(self) -> Optional[uuid.UUID]:
        return None

    @property
    def parent(self) -> Optional["ModuleNode"]:
        return None

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
            if remaining_depth is not None:
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

    kind = models.CharField(max_length=32, choices=RefMappingKind.choices)
    source_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="child_refs"
    )
    target_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="parent_refs"
    )
    type = models.CharField(max_length=32)
    source_id = models.UUIDField()
    source_revision = models.IntegerField()
    target_id = models.UUIDField()
    target_revision = models.IntegerField()

    objects = RefMappingManager()

    def __str__(self):
        return f"{self.source_version} {self.source_id} -> {self.target_version} {self.target_id}"

    def __repr__(self):
        return f"<RefMapping {self}>"


class FileManager(models.Manager):
    def get_queryset(self) -> models.QuerySet[Statement]:
        # soft-deleted statements are not returned by default
        return super().get_queryset().filter(deleted_at__isnull=True)

    def copy(
        self,
        file: "File",
        source: ProjectVersion,
        target: ProjectVersion,
        kind: "RefMappingKind",
        target_id: Optional[UUID] = None,
        target_parent: Optional["File"] = None,
        copy_revisions: bool = False,
    ) -> "File":
        """Copies a file from one module to another (may be the same)."""
        from bench.models import Dataset, Statement, packer

        target_id = target_id or uuid.uuid4()
        # pack relevant nodes
        packed, mappings, target_ids = ProjectVersion.objects.pack_copy(
            source=source,
            target=target,
            nodes=[file],
            copy_revisions=copy_revisions,
            target_ids={file.id: target_id},
            kind=kind,
        )
        packed.roots[0].parent_id = target_parent.id if target_parent else target.id

        # unpack and save
        unpacked = packer.unpack_nodes_tree(packed.nodes_list(), pre_unpacked={target.id: target})
        create_models_bfs(unpacked.walk_bfs_batched())
        # duplicate versioned datasets
        versioned_datasets = [
            n for n in unpacked.nodes.values() if isinstance(n, Dataset) and n.versioned
        ]
        Statement.objects.duplicate_datasets_inplace(source, target, versioned_datasets)
        # save mappings
        RefMapping.objects.bulk_create(mappings)

        target_file = unpacked.nodes[target_id]
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


class File(UUIDModel, CrudModel, ModuleNode, Revisioned):
    """
    A file containing statements, potentially containing other files if it's a directory.
    A file - and the statements it contains - may be soft-deleted.
    Nothing is actually deleted, but soft deleted objects are not visible and not copied across versions.
    """

    project_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="files"
    )
    revision = models.IntegerField(default=0)
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

    @gql.model_property(only=["name", "parent"], select_related=["parent"])
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
        # path doesn't have to be unique
        constraints = []
