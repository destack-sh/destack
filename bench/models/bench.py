from __future__ import annotations

import os
import uuid
from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID, uuid4

import structlog
from asgiref.sync import async_to_sync
from django.core.validators import validate_slug
from django.db import models, transaction
from django.db.models import Q
from pgcrypto.fields import TextPGPSymmetricKeyField
from strawberry_django.descriptors import model_property

from bench.language.validation import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH
from bench.models.blob import get_s3_client
from bench.models.statement import Statement
from bench.models.utils import CrudModel, CrudNode, Node, UUIDModel, create_models_bfs
from bench.settings import GLOBAL_PROJECT_BUCKET_NAME, LOCAL
from bench.utils.func import (
    generate_random_lowercase_name,
    generate_random_name,
    generate_secret_password,
)

if TYPE_CHECKING:
    from bench.models.organization import Organization
    from bench.models.user import User

logger = structlog.get_logger(__name__)


class BenchVisibility(models.TextChoices):
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


_PROJECT_AUTH_COLUMNS = ("pg_username", "pg_password", "os_username", "os_password")


class BenchManager(models.Manager["Bench"]):
    def get_queryset(self):
        return super().get_queryset().defer(*_PROJECT_AUTH_COLUMNS)

    @transaction.atomic
    def create_bench(
        self,
        owner: User | Organization,
        name: str,
        slug: str,
        id: UUID = None,
        visibility: BenchVisibility = BenchVisibility.PRIVATE,
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
        bench = super().create(
            id=id, organization=organization, user=user, name=name, slug=slug, visibility=visibility
        )
        # initial version head
        bench.head = Module.objects.create(id=head_version_id or uuid4(), ck=bench.id, bench=bench)
        bench.save()

        # onboarding
        if create_onboarding_files:
            # TODO @UX: re-implement create onboarding files
            pass

        # infra
        if create_infra:
            from bench.search.engine import create_local_os_index
            from bench.sql.engine import create_local_pg_database

            logger.info("bench.create_infra", bench=bench)
            start_time = datetime.now()
            create_local_worker_set(bench)
            create_local_os_index(
                os_name=bench.os_name,
                os_username=bench.os_username,
                os_password=bench.os_password,
                is_public=bench.visibility == BenchVisibility.PUBLIC,
                upsert=False,
            )
            async_to_sync(create_local_pg_database)(
                pg_name=bench.pg_name,
                pg_username=bench.pg_username,
                pg_password=bench.pg_password,
                is_public=bench.visibility == BenchVisibility.PUBLIC,
                upsert=False,
            )

            duration = datetime.now() - start_time
            logger.info("bench.create_infra.done", bench=bench, duration=duration.total_seconds())

        return bench

    def get_by_slug(self, owner: str, bench: str):
        return (
            self.filter(slug=bench)
            .filter(Q(organization__owner_slug_id=owner) | Q(user__owner_slug_id=owner))
            .get()
        )


class Bench(UUIDModel, CrudModel):
    """
    A Bench contains all of its versions, similar to repositories in Git.
    """

    name: models.CharField = models.CharField(max_length=MAX_NAME_LENGTH)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, null=True)
    slug: models.SlugField = models.SlugField(max_length=128, validators=[validate_slug])

    visibility = models.CharField(
        max_length=32, choices=BenchVisibility.choices, default=BenchVisibility.PRIVATE
    )
    sharing_enabled = models.BooleanField(default=True)
    sharing_token = models.UUIDField(default=uuid4)
    sharing_level = models.IntegerField(default=ModuleAccessLevel.Read)
    organization: models.ForeignKey = models.ForeignKey(
        "Organization", on_delete=models.CASCADE, related_name="benches", null=True
    )
    user: models.ForeignKey = models.ForeignKey(
        "User", on_delete=models.CASCADE, related_name="benches", null=True
    )
    members = models.ManyToManyField("User", through="BenchMembership", related_name="benches+")
    memberships: models.QuerySet["BenchMembership"]  # noqa via BenchMembership.bench

    head = models.ForeignKey("Module", on_delete=models.SET_NULL, null=True, related_name="bench+")
    blobs: models.QuerySet["Blob"]  # noqa via Blob
    worker_set = models.OneToOneField(  # only one worker set for now
        "WorkerSet", on_delete=models.SET_NULL, related_name="bench+", null=True
    )
    worker_sets: models.QuerySet["WorkerSet"]  # noqa via WorkerSet
    pg_name = models.CharField(max_length=64, default=generate_random_lowercase_name)
    pg_username = models.CharField(max_length=64, default=generate_random_name)
    pg_password = TextPGPSymmetricKeyField(default=generate_secret_password)
    os_name = models.CharField(max_length=64, default=generate_random_lowercase_name)
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
    def head_(self) -> Module:
        if self.head is None:
            raise ValueError(f"bench {self} has no head")
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
        parent: Optional[Module] = None,
    ) -> "Module":
        if parent is None:
            if self.head is None:
                raise ValueError(f"bench does not have a head: {self}")
            assigned_parent = self.head
        else:
            assigned_parent = parent
        del parent  # avoid accidental use
        if not assigned_parent.committed:
            raise ValueError(f"parent must be committed: {assigned_parent}")

        new_version = Module.objects.create(bench=self, name=name, tag=tag, description=description)
        new_version.parents.add(assigned_parent)
        self.head = new_version
        self.save()

        return new_version

    @transaction.atomic(savepoint=False)
    def create_invite(
        self, email: str, level: "ModuleAccessLevel", message: str = None, created_by: User = None
    ) -> "BenchInvite":
        from bench.models.notification import Notification, NotificationType
        from bench.models.user import User

        user = User.objects.filter(email=email).first()
        if user is not None and self.members.filter(id=user.id).exists():
            raise ValueError("user already a member of organization")

        invite = BenchInvite.objects.create(
            bench=self,
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
    def accept_invite(self, invite: "BenchInvite") -> None:
        if invite.user is None:
            raise ValueError("cannot accept invite without registered user")
        invite.bench.add_member(invite.user, invite.level)
        invite.delete()

    def add_member(self, user: User, level: "ModuleAccessLevel") -> None:
        if self.members.filter(id=user.id).exists():
            raise ValueError("user already a member of bench")
        BenchMembership.objects.create(bench=self, user=user, level=level)

    objects: BenchManager = BenchManager()

    class Meta:
        base_manager_name = "objects"
        default_related_name = "benches"
        constraints = [
            # unique slug per owner
            models.UniqueConstraint(
                name="bench_bench_organization_slug_ak",
                fields=["organization", "slug"],
                condition=models.Q(organization__isnull=False),
            ),
            models.UniqueConstraint(
                name="bench_bench_user_slug_ak",
                fields=["user", "slug"],
                condition=models.Q(user__isnull=False),
            ),
            # unique sharing token
            models.UniqueConstraint(name="bench_bench_sharing_token_ak", fields=["sharing_token"]),
            # must have at least one owner (organization or user)
            models.CheckConstraint(
                name="bench_bench_owner_ck",
                check=models.Q(organization__isnull=False) | models.Q(user__isnull=False),
            ),
        ]


class BenchMembership(UUIDModel):
    bench: models.ForeignKey = models.ForeignKey(
        Bench, on_delete=models.CASCADE, related_name="memberships"
    )
    user: models.ForeignKey = models.ForeignKey(
        "User", on_delete=models.CASCADE, related_name="bench_memberships"
    )
    level = models.IntegerField(choices=ModuleAccessLevel.choices)

    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)

    def __str__(self):
        return f"{self.bench} -> {self.user} ({self.level})"

    def __repr__(self):
        return f"<BenchMembership {self}>"

    class Meta:
        ordering = ["-created_at"]
        constraints = [
            models.UniqueConstraint(name="bench_bench_membership_ak", fields=["bench", "user"])
        ]


class BenchInvite(UUIDModel):
    """
    An invitation to join a bench (for existing or not yet existing users).
    """

    bench = models.ForeignKey("Bench", on_delete=models.CASCADE, related_name="invites")
    email = models.EmailField()
    user = models.ForeignKey(
        "User", on_delete=models.CASCADE, related_name="bench_invites", null=True
    )
    level = models.IntegerField(choices=ModuleAccessLevel.choices)
    message = models.TextField(blank=True, null=True)
    email_sent_at = models.DateTimeField(blank=True, null=True)

    created_by = models.ForeignKey("User", on_delete=models.CASCADE, related_name="+")
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)

    def __str__(self):
        return f"{self.bench} -> {self.email} ({self.level})"

    def __repr__(self):
        return f"<BenchInvite {self}>"

    def accept(self):
        self.bench.accept_invite(self)

    class Meta:
        ordering = ["-created_at"]
        constraints = [
            models.UniqueConstraint(name="bench_bench_invite_ak", fields=["bench", "email"])
        ]


class ModuleManager(models.Manager["BenchVersion"]):
    def get_by_slug(self, owner: str, bench: str, tag: str):
        return (
            self.filter(tag=tag)
            .filter(bench__slug=bench)
            .filter(
                Q(bench__organization__owner_slug_id=owner) | Q(bench__user__owner_slug_id=owner)
            )
            .get()
        )

    def get_by_tag(self, bench_id: UUID, tag: str):
        return self.filter(bench_id=bench_id, tag=tag).get()


class Module(CrudNode):
    """
    A bench version records the state of a bench at a specific point in time.
    """

    parent_bench = models.ForeignKey(Bench, on_delete=models.CASCADE, related_name="versions")
    name = models.CharField(max_length=MAX_NAME_LENGTH, null=True)
    # single unique tag should be a BenchVersionTag list later :BenchVersionTags
    tag = models.CharField(max_length=MAX_NAME_LENGTH, null=True)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, null=True)
    is_snapshot = models.BooleanField(default=False)

    parents = models.ManyToManyField("Module", related_name="children", symmetrical=False)
    files: models.QuerySet["File"]  # noqa via File
    statements: models.QuerySet["Statement"]  # noqa via Statement
    sessions: models.QuerySet["Session"]  # noqa via Session
    runs: models.QuerySet["Run"]  # noqa via Run

    def __str__(self) -> str:
        return f"{self.parent_bench.path}@{self.tag or self.id.hex}"

    @property
    def parent_id(self) -> Optional[uuid.UUID]:
        return None

    @property
    def parent(self) -> Optional["Node"]:
        return None

    @property
    def organization(self):
        return self.parent_bench.organization

    @transaction.atomic
    def create_file(self, name: str, parent: Optional[File] = None) -> "File":
        file = File.objects.create(module=self, parent=parent, name=name)
        return file

    objects = ModuleManager()

    class Meta:
        ordering = ["-created_at"]
        constraints = [
            # tag is unique per bench
            models.UniqueConstraint(
                fields=["parent_bench", "tag"],
                name="bench_module_tag_ak",
            ),
        ]


class FileManager(models.Manager):
    def get_queryset(self) -> models.QuerySet[Statement]:
        # soft-deleted statements are not returned by default
        return super().get_queryset().filter(deleted_at__isnull=True)

    def copy(
        self,
        file: "File",
        source: Module,
        target: Module,
        target_id: UUID,
        target_ck: UUID,
        keep_cks: bool,
        include_interp: bool = True,
        target_parent: Optional["File"] = None,
        copy_revisions: bool = False,
    ) -> "File":
        """Copies a file from one module to another (may be the same)."""
        from bench.models import packer

        target_id = target_id or uuid.uuid4()
        # pack relevant nodes
        copy = Module.objects.pack_copy(
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

        target_file = unpacked.nodes_by_id[target_id]
        return target_file


class File(CrudNode):
    """
    A file containing statements, potentially containing other files if it's a directory.
    A file - and the statements it contains - may be soft-deleted.
    Nothing is actually deleted, but soft deleted objects are not visible and not copied across versions.
    """

    module = models.ForeignKey("Module", on_delete=models.CASCADE, related_name="files")
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
            return f"{self.module}/{self.name}"

    @model_property(only=["name", "parent"], select_related=["parent"])
    def path(self) -> str:
        return f"{self.parent_file.path}/{self.name}" if self.parent_file else f"{self.name}"

    @property
    def parent_id(self) -> Optional[uuid.UUID]:
        return self.parent_file_id or self.module_id

    @property
    def parent(self) -> Union["File", "Module"]:
        if self.parent_file_id:
            return self.parent_file
        else:
            return self.module

    def is_root(self) -> bool:
        return self.parent_file is None

    @property
    def root_statements(self) -> models.QuerySet["Statement"]:
        return self.statements.filter(parent=None)

    objects = FileManager()

    class Meta:
        ordering = ["name"]
        constraints = [
            # ck is unique per bench version
            models.UniqueConstraint(
                fields=["module", "ck"],
                name="bench_file_module_ck",
                condition=models.Q(deleted_at__isnull=True),
            ),
        ]


def create_global_user_bucket(ignore_exists: bool):
    """
    Creates a public S3 bucket for all benches.
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


def create_local_worker_set(bench: Bench, *, upsert: bool):
    from bench.language.const import ProjectRegion, WorkerProfile, WorkerSetStatus
    from bench.models import WorkerSet

    # check if worker set already exists
    if WorkerSet.objects.filter(bench_id=bench.id).exists():
        if not upsert:
            raise ValueError(f"worker set already exists for bench: {bench}")
        return

    worker_set = WorkerSet.objects.create(
        bench_id=bench.id,
        region=ProjectRegion.EU_CENTRAL,
        profile=WorkerProfile.TINY,
        sleeping=True,
        desired_replicas=1,
        target_replicas=1,
        status=WorkerSetStatus.SLEEPING,
    )
    bench.worker_set = worker_set
    bench.save()
