from typing import TYPE_CHECKING, Annotated, Optional, Union
from uuid import UUID

import strawberry
import strawberry_django
from django.core.exceptions import ValidationError
from django.db.models import Sum
from strawberry import UNSET, auto, lazy, relay
from strawberry.relay import GlobalID
from strawberry.types import Info
from strawberry_django.fields.types import OperationInfo

from bench import models
from bench.api.auth import check_module_access, has_module_access, is_owner_or_member
from bench.api.utils import HasCrud, ModuleNode, get_user_from_info, safe_mutation
from bench.language import const
from bench.language.cache import _get_usage_key
from bench.models import ModuleAccessLevel
from bench.utils.cache import redis_sync

if TYPE_CHECKING:
    from bench.api.organization import Organization
    from bench.api.user import User

StatementType = strawberry.enum(const.StatementType)


@strawberry_django.filter(models.BenchVersion)
class BenchVersionFilter:
    from_id: GlobalID = UNSET
    to_id: GlobalID = UNSET

    def filter(self, queryset):
        if self.from_id is not None and self.to_id is not None:
            from_v = models.BenchVersion.objects.get(id=self.from_id.node_id)
            to_v = models.BenchVersion.objects.get(id=self.to_id.node_id)
            # we can go both directions
            if from_v.created_at < to_v.created_at:  # migrate forwards
                queryset = queryset.filter(
                    created_at__gte=from_v.created_at, created_at__lte=to_v.created_at
                )
            else:  # migrate backwards (reversing source/target in the client)
                queryset = queryset.filter(
                    created_at__lte=from_v.created_at, created_at__gte=to_v.created_at
                )
        elif self.from_id is not None or self.to_id is not None:
            raise ValidationError("from_id and to_id must be set together")
        return queryset.order_by("created_at")


@strawberry_django.filter(models.File)
class FileFilter:
    is_visible: Optional[bool] = True

    def filter(self, queryset):
        if self.is_visible is not None:
            queryset = queryset.filter(deleted_at__isnull=self.is_visible)
        return queryset


BenchVisibility = strawberry.enum(models.BenchVisibility)


@strawberry.type
class BenchUsage:
    records_active: int
    objects_bytes_total: int
    cache_bytes_total: int


def get_bench_usage(info: Info) -> BenchUsage:
    bench_id = UUID(info.variable_values.get("benchId").node_id)

    # count total object bytes
    object_bytes_total = (
        models.Blob.objects.filter(bench_id=bench_id)
        .values("content_length")
        .aggregate(Sum("content_length"))["content_length__sum"]
    )

    # get cache bytes total
    cache_bytes_total = redis_sync.get(_get_usage_key(bench_id))
    return BenchUsage(
        records_active=0,  # doesn't matter anymore, will remove later in favor of bytes
        objects_bytes_total=object_bytes_total or 0,
        cache_bytes_total=cache_bytes_total or 0,
    )


@strawberry_django.type(models.Bench)
class Bench(relay.Node):
    created_at: auto
    updated_at: auto
    name: auto
    slug: auto
    visibility: BenchVisibility
    path: auto
    description: auto

    owner: Union[Annotated["User", lazy(".user")], Annotated["Organization", lazy(".organization")]]
    base_level: int
    sharing_enabled: bool
    sharing_token: Optional[UUID]
    sharing_level: int

    head: "BenchVersion"
    versions: strawberry_django.relay.ListConnectionWithTotalCount[
        "BenchVersion"
    ] = strawberry_django.connection(filters=BenchVersionFilter)

    usage: BenchUsage = strawberry_django.field(resolver=get_bench_usage)

    @strawberry_django.field
    def access_level(self, info: OperationInfo) -> int:
        access = has_module_access(info, self, models.ModuleAccessLevel.Read)
        return access.level if access else models.ModuleAccessLevel.Zero


@strawberry_django.type(models.BenchMembership)
class BenchMembership(relay.Node):
    bench: Bench
    user: Annotated["User", lazy(".user")]
    level: int
    created_at: auto
    updated_at: auto


@strawberry_django.type(models.BenchInvite)
class BenchInvite(relay.Node):
    bench: Bench
    user: Optional[Annotated["User", lazy(".user")]]
    email: auto
    level: int
    created_at: auto
    updated_at: auto
    email_sent_at: auto


@strawberry_django.type(models.BenchVersion)
class BenchVersion(HasCrud, ModuleNode, relay.Node):
    bench: Bench
    parent: Optional[ModuleNode]
    name: auto
    tag: auto  # :BenchVersionTags
    description: auto
    parents: list["BenchVersion"]
    children: list["BenchVersion"]
    is_snapshot: bool


@strawberry.input
class BenchCreateInput:
    owner_id: GlobalID
    name: str
    slug: str
    visibility: BenchVisibility


@strawberry.input
class BenchUpdateVisibilityInput(strawberry_django.NodeInput):
    visibility: BenchVisibility


@strawberry.input
class BenchUpdateSharingInput(strawberry_django.NodeInput):
    sharing_enabled: bool
    sharing_token: UUID
    sharing_level: int


@strawberry.input
class BenchUpdateNameInput(strawberry_django.NodeInput):
    name: str


@strawberry.input
class BenchInviteInput(strawberry_django.NodeInput):
    emails: list[str]
    level: int
    message: Optional[str] = None


@strawberry.input
class BenchUpdateMembershipInput(strawberry_django.NodeInput):
    user_id: GlobalID
    level: int


@strawberry.input
class BenchRemoveMembershipInput(strawberry_django.NodeInput):
    user_id: GlobalID


@strawberry.type
class BenchMutation:
    @safe_mutation
    def create_bench(self, info: Info, input: "BenchCreateInput") -> Bench | OperationInfo:
        requesting_user = get_user_from_info(info)
        owner_model = models.User if input.owner_id.type_name == "User" else models.Organization
        owner = owner_model.objects.get(id=input.owner_id.node_id)
        if not is_owner_or_member(requesting_user, owner):
            raise PermissionError("cannot create bench for this owner")
        bench = models.Bench.objects.create_bench(
            owner=owner,
            name=input.name,
            slug=input.slug,
            visibility=input.visibility,
            create_onboarding_files=True,
        )
        return bench

    @safe_mutation
    def update_bench_visibility(
        self, info: Info, input: "BenchUpdateVisibilityInput"
    ) -> Bench | OperationInfo:
        bench = models.Bench.objects.get(id=input.id.node_id)
        check_module_access(info, bench, ModuleAccessLevel.Manage)
        bench.visibility = input.visibility
        # TODO @Broken: update bench infra permissions on visibility change
        bench.save()
        return bench

    @safe_mutation
    def update_bench_sharing(
        self, info: Info, input: "BenchUpdateSharingInput"
    ) -> Bench | OperationInfo:
        bench = models.Bench.objects.get(id=input.id.node_id)
        check_module_access(info, bench, ModuleAccessLevel.Manage)
        bench.sharing_enabled = input.sharing_enabled
        bench.sharing_token = input.sharing_token
        bench.sharing_level = input.sharing_level
        bench.save()
        return bench

    @safe_mutation
    def update_bench_name(self, info: Info, input: "BenchUpdateNameInput") -> Bench | OperationInfo:
        bench = models.Bench.objects.get(id=input.id.node_id)
        check_module_access(info, bench, ModuleAccessLevel.Manage)
        bench.name = input.name
        bench.save()
        return bench

    @safe_mutation(atomic=True)
    def create_bench_invites(self, info, input: BenchInviteInput) -> Bench | OperationInfo:
        bench = models.Bench.objects.get(id=input.id.node_id)
        user = get_user_from_info(info)
        check_module_access(info, bench, ModuleAccessLevel.Manage)
        for email in input.emails:
            invite = bench.create_invite(
                email=email, level=input.level, message=input.message, created_by=user
            )
            invite.full_clean()
        return bench

    @safe_mutation
    def cancel_bench_invite(self, info: Info, id: GlobalID) -> Bench | OperationInfo:
        invite = models.BenchInvite.objects.get(id=id.node_id)
        check_module_access(info, invite.bench, ModuleAccessLevel.Manage)
        invite.delete()
        return invite.bench

    @safe_mutation
    def remove_bench_membership(
        self, info: Info, input: "BenchRemoveMembershipInput"
    ) -> Bench | OperationInfo:
        bench = models.Bench.objects.get(id=input.id.node_id)
        check_module_access(info, bench, ModuleAccessLevel.Manage)
        bench.memberships.filter(user_id=input.user_id.node_id).delete()
        return bench


@strawberry.input
class UpdateBenchVersion(strawberry_django.NodeInput):
    name: str
    tag: Optional[str] = None  # :BenchVersionTags
    description: Optional[str] = None


@strawberry.input
class SnapshotInput:
    bench_version_id: GlobalID
    name: Optional[str] = None
    tag: Optional[str] = None  # :BenchVersionTags
    description: Optional[str] = None


@strawberry.type
class SnapshotPayload:
    bench: Bench


@strawberry.type
class BenchVersionMutation:
    @safe_mutation
    def update_bench_version(
        self, info, input: "UpdateBenchVersion"
    ) -> BenchVersion | OperationInfo:
        bench_v = models.BenchVersion.objects.select_related("bench").get(id=input.id.node_id)
        check_module_access(info, bench_v.bench, ModuleAccessLevel.Edit)
        bench_v.name = input.name
        bench_v.description = input.description
        bench_v.tag = input.tag
        bench_v.save()
        return bench_v
