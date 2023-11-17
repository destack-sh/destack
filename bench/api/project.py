from typing import TYPE_CHECKING, Annotated, Optional, Union
from uuid import UUID

import strawberry
import strawberry_django
from asgiref.sync import sync_to_async
from django.core.exceptions import ValidationError
from django.db.models import Sum
from strawberry import UNSET, auto, lazy, relay
from strawberry.relay import GlobalID
from strawberry.types import Info
from strawberry_django.fields.types import OperationInfo

from bench import models
from bench.api.auth import check_module_access, has_module_access, is_owner_or_member
from bench.api.utils import (
    HasCrud,
    ModuleNode,
    asafe_mutation,
    get_client_origin_from_info,
    get_user_from_info,
    safe_mutation,
)
from bench.language import const
from bench.language.cache import _get_usage_key
from bench.models import ModuleAccessLevel
from bench.msg.core import NMessage, request
from bench.msg.messages import NMessageType, RepSnapshotModulePayload, ReqSnapshotModulePayload
from bench.utils.cache import redis_sync

if TYPE_CHECKING:
    from bench.api.file import File
    from bench.api.organization import Organization
    from bench.api.session import WorkerSet
    from bench.api.user import User

StatementType = strawberry.enum(const.StatementType)


@strawberry_django.filter(models.ProjectVersion)
class ProjectVersionFilter:
    from_id: GlobalID = UNSET
    to_id: GlobalID = UNSET

    def filter(self, queryset):
        if self.from_id is not None and self.to_id is not None:
            from_v = models.ProjectVersion.objects.get(id=self.from_id.node_id)
            to_v = models.ProjectVersion.objects.get(id=self.to_id.node_id)
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


ProjectVisibility = strawberry.enum(models.ProjectVisibility)


@strawberry.type
class ProjectUsage:
    records_active: int
    objects_bytes_total: int
    cache_bytes_total: int


def get_project_usage(info: Info) -> ProjectUsage:
    project_id = UUID(info.variable_values.get("projectId").node_id)

    # count total object bytes
    object_bytes_total = (
        models.Blob.objects.filter(project_id=project_id)
        .values("content_length")
        .aggregate(Sum("content_length"))["content_length__sum"]
    )

    # get cache bytes total
    cache_bytes_total = redis_sync.get(_get_usage_key(project_id))
    return ProjectUsage(
        records_active=0,  # doesn't matter anymore, will remove later in favor of bytes
        objects_bytes_total=object_bytes_total or 0,
        cache_bytes_total=cache_bytes_total or 0,
    )


@strawberry_django.type(models.Project)
class Project(relay.Node):
    created_at: auto
    updated_at: auto
    name: auto
    slug: auto
    visibility: ProjectVisibility
    path: auto
    description: auto

    owner: Union[Annotated["User", lazy(".user")], Annotated["Organization", lazy(".organization")]]
    base_level: int
    sharing_enabled: bool
    sharing_token: Optional[UUID]
    sharing_level: int

    head: "ProjectVersion"
    versions: strawberry_django.relay.ListConnectionWithTotalCount[
        "ProjectVersion"
    ] = strawberry_django.connection(filters=ProjectVersionFilter)

    worker_set: Annotated["WorkerSet", lazy(".session")]
    worker_sets: list[Annotated["WorkerSet", lazy(".session")]]
    usage: ProjectUsage = strawberry_django.field(resolver=get_project_usage)

    @strawberry_django.field
    def access_level(self, info: OperationInfo) -> int:
        access = has_module_access(info, self, models.ModuleAccessLevel.Read)
        return access.level if access else models.ModuleAccessLevel.Zero


@strawberry_django.type(models.ProjectMembership)
class ProjectMembership(relay.Node):
    project: Project
    user: Annotated["User", lazy(".user")]
    level: int
    created_at: auto
    updated_at: auto


@strawberry_django.type(models.ProjectInvite)
class ProjectInvite(relay.Node):
    project: Project
    user: Optional[Annotated["User", lazy(".user")]]
    email: auto
    level: int
    created_at: auto
    updated_at: auto
    email_sent_at: auto


@strawberry_django.type(models.ProjectVersion)
class ProjectVersion(HasCrud, ModuleNode, relay.Node):
    project: Project
    parent: Optional[ModuleNode]
    name: auto
    tag: auto  # :ProjectVersionTags
    description: auto
    parents: list["ProjectVersion"]
    children: list["ProjectVersion"]
    committed: auto
    committed_at: auto
    files: list[Annotated["File", lazy(".file")]] = strawberry_django.field(filters=FileFilter)


@strawberry.input
class ProjectCreateInput:
    owner_id: GlobalID
    name: str
    slug: str
    visibility: ProjectVisibility


@strawberry.input
class ProjectUpdateVisibilityInput(strawberry_django.NodeInput):
    visibility: ProjectVisibility


@strawberry.input
class ProjectUpdateSharingInput(strawberry_django.NodeInput):
    base_level: int
    sharing_enabled: bool
    sharing_token: UUID
    sharing_level: int


@strawberry.input
class ProjectUpdateNameInput(strawberry_django.NodeInput):
    name: str


@strawberry.input
class ProjectInviteInput(strawberry_django.NodeInput):
    emails: list[str]
    level: int
    message: Optional[str] = None


@strawberry.input
class ProjectUpdateMembershipInput(strawberry_django.NodeInput):
    user_id: GlobalID
    level: int


@strawberry.input
class ProjectRemoveMembershipInput(strawberry_django.NodeInput):
    user_id: GlobalID


@strawberry.type
class ProjectMutation:
    @safe_mutation
    def create_project(self, info: Info, input: "ProjectCreateInput") -> Project | OperationInfo:
        requesting_user = get_user_from_info(info)
        owner_model = models.User if input.owner_id.type_name == "User" else models.Organization
        owner = owner_model.objects.get(id=input.owner_id.node_id)
        if not is_owner_or_member(requesting_user, owner):
            raise PermissionError("cannot create project for this owner")
        project = models.Project.objects.create_project(
            owner=owner,
            name=input.name,
            slug=input.slug,
            visibility=input.visibility,
            create_onboarding_files=True,
        )
        return project

    @safe_mutation
    def update_project_visibility(
        self, info: Info, input: "ProjectUpdateVisibilityInput"
    ) -> Project | OperationInfo:
        project = models.Project.objects.get(id=input.id.node_id)
        check_module_access(info, project, ModuleAccessLevel.Manage)
        project.visibility = input.visibility
        project.save()
        return project

    @safe_mutation
    def update_project_sharing(
        self, info: Info, input: "ProjectUpdateSharingInput"
    ) -> Project | OperationInfo:
        project = models.Project.objects.get(id=input.id.node_id)
        check_module_access(info, project, ModuleAccessLevel.Manage)
        project.base_level = input.base_level
        project.sharing_enabled = input.sharing_enabled
        project.sharing_token = input.sharing_token
        project.sharing_level = input.sharing_level
        project.save()
        return project

    @safe_mutation
    def update_project_name(
        self, info: Info, input: "ProjectUpdateNameInput"
    ) -> Project | OperationInfo:
        project = models.Project.objects.get(id=input.id.node_id)
        check_module_access(info, project, ModuleAccessLevel.Manage)
        project.name = input.name
        project.save()
        return project

    @safe_mutation(atomic=True)
    def create_project_invites(self, info, input: ProjectInviteInput) -> Project | OperationInfo:
        project = models.Project.objects.get(id=input.id.node_id)
        user = get_user_from_info(info)
        check_module_access(info, project, ModuleAccessLevel.Manage)
        for email in input.emails:
            invite = project.create_invite(
                email=email, level=input.level, message=input.message, created_by=user
            )
            invite.full_clean()
        return project

    @safe_mutation
    def cancel_project_invite(self, info: Info, id: GlobalID) -> Project | OperationInfo:
        invite = models.ProjectInvite.objects.get(id=id.node_id)
        check_module_access(info, invite.project, ModuleAccessLevel.Manage)
        invite.delete()
        return invite.project

    @safe_mutation
    def remove_project_membership(
        self, info: Info, input: "ProjectRemoveMembershipInput"
    ) -> Project | OperationInfo:
        project = models.Project.objects.get(id=input.id.node_id)
        check_module_access(info, project, ModuleAccessLevel.Manage)
        project.memberships.filter(user_id=input.user_id.node_id).delete()
        return project


@strawberry.input
class UpdateProjectVersion(strawberry_django.NodeInput):
    name: str
    tag: Optional[str] = None  # :ProjectVersionTags
    description: Optional[str] = None


@strawberry.input
class SnapshotInput:
    project_version_id: GlobalID
    name: Optional[str] = None
    tag: Optional[str] = None  # :ProjectVersionTags
    description: Optional[str] = None


@strawberry.type
class SnapshotPayload:
    project: Project


@strawberry.type
class ProjectVersionMutation:
    @safe_mutation
    def update_project_version(
        self, info, input: "UpdateProjectVersion"
    ) -> ProjectVersion | OperationInfo:
        project_v = models.ProjectVersion.objects.select_related("project").get(id=input.id.node_id)
        check_module_access(info, project_v.project, ModuleAccessLevel.Edit)
        project_v.name = input.name
        project_v.description = input.description
        project_v.tag = input.tag
        project_v.save()
        return project_v

    @asafe_mutation
    async def snapshot(self, info, input: SnapshotInput) -> SnapshotPayload | OperationInfo:
        head = await models.ProjectVersion.objects.select_related("project").aget(
            id=input.project_version_id.node_id
        )
        project = head.project
        if head.id != project.head_id:
            raise ValueError("cannot commit version that's not the head")
        await sync_to_async(check_module_access)(info, project, ModuleAccessLevel.Edit)

        req = ReqSnapshotModulePayload(
            module_id=head.id,
            name=input.name,
            tag=input.tag,
            description=input.description,
            client=get_client_origin_from_info(info),
        )
        rep: NMessage[RepSnapshotModulePayload] = await request(NMessageType.SNAPSHOT_MODULE, req)
        if not rep.p.success:
            raise RuntimeError(f"failed to create snapshot: {rep.p.error}")

        return SnapshotPayload(project=project)
