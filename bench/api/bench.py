from typing import TYPE_CHECKING, Annotated, Optional, Union
from uuid import UUID

import strawberry
import strawberry_django
from django.db.models import Sum
from strawberry import auto, lazy, relay
from strawberry.relay import GlobalID
from strawberry.types import Info
from strawberry_django.fields.types import OperationInfo

from bench import models
from bench.api.auth import check_module_access, has_module_access, is_owner_or_member
from bench.api.utils import get_user_from_info, safe_mutation
from bench.language import const
from bench.language.cache import _get_usage_key
from bench.models import ModuleAccessLevel
from bench.utils.cache import redis_sync

if TYPE_CHECKING:
    from bench.api.organization import Organization
    from bench.api.user import User

StatementType = strawberry.enum(const.StatementType)


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


@strawberry.input
class BenchCreateInput:
    owner_id: GlobalID
    name: str
    slug: str
    visibility: BenchVisibility


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
