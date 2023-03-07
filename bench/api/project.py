from typing import TYPE_CHECKING, Annotated, Optional, Union, cast

from django.core.exceptions import ValidationError
from strawberry import UNSET, lazy
from strawberry.types import Info
from strawberry_django_plus import gql
from strawberry_django_plus.gql import auto
from strawberry_django_plus.relay import GlobalID
from strawberry_django_plus.types import OperationInfo

from bench import models
from bench.api.auth import can_write_project, check_can_write_project, is_owner_or_member
from bench.api.sync import PMT, project_mutation
from bench.api.util import safe_mutation
from bench.models.project import RefDict

if TYPE_CHECKING:
    from bench.api.deployment import Deployment
    from bench.api.organization import Organization
    from bench.api.statement import Statement
    from bench.api.user import User

StatementType = gql.enum(models.StatementType)


@gql.django.filter(models.ProjectVersion)
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


@gql.django.filter(models.Statement)
class StatementFilter:
    is_visible: Optional[bool] = True

    def filter(self, queryset):
        if self.is_visible is not UNSET and self.is_visible is not None:
            queryset = queryset.filter(deleted_at__isnull=self.is_visible)
        return queryset


@gql.django.filter(models.File)
class FileFilter:
    is_visible: Optional[bool] = True

    def filter(self, queryset):
        if self.is_visible is not None:
            queryset = queryset.filter(deleted_at__isnull=self.is_visible)
        return queryset


@gql.django.filter(models.Deployment)
class DeploymentFilter:
    is_owned: Optional[bool] = None

    def filter(self, queryset):
        if self.is_owned is not None:
            queryset = queryset.filter(owned=self.is_owned)
        return queryset


ProjectVisibility = gql.enum(models.ProjectVisibility)
ProjectType = gql.enum(models.ProjectType)


@gql.django.type(models.Project)
class Project(gql.Node):
    name: auto
    slug: auto
    type: ProjectType
    visibility: ProjectVisibility
    path: auto
    owner: Union[Annotated["User", lazy(".user")], Annotated["Organization", lazy(".organization")]]
    created_at: auto
    updated_at: auto
    head: "ProjectVersion"
    versions: gql.relay.Connection["ProjectVersion"] = gql.django.connection(
        filters=ProjectVersionFilter
    )
    deployments: gql.relay.Connection[
        Annotated["Deployment", lazy(".deployment")]
    ] = gql.django.connection(filters=DeploymentFilter)

    # TODO @Performance: specify only/select_related for can_write field
    @gql.field
    def can_write(self, info: OperationInfo) -> bool:
        user = cast(models.User, info.context.request.scope["user"]._wrapped)
        return can_write_project(user, self)


@gql.type
class RefMapping:
    source: GlobalID
    target: GlobalID


@gql.django.type(models.ProjectVersion)
class ProjectVersion(gql.Node):
    project: Project
    name: auto
    tag: auto  # :ProjectVersionTags
    description: auto
    parents: list["ProjectVersion"]
    children: list["ProjectVersion"]
    created_at: auto
    committed: auto
    committed_at: auto
    dependencies: list["ProjectVersion"]
    files: gql.relay.Connection["File"] = gql.django.connection(filters=FileFilter)
    deployments: gql.relay.Connection[
        Annotated["Deployment", lazy(".deployment")]
    ] = gql.django.connection(filters=DeploymentFilter)

    @gql.field
    def parents_refs(self) -> list[RefMapping]:
        refs: list[RefDict] = self.parents_refs
        ref_mappings = [
            RefMapping(
                source=GlobalID(ref["type"], ref["source"]),
                target=GlobalID(ref["type"], ref["target"]),
            )
            for ref in refs
        ]
        return ref_mappings


@gql.django.type(models.File)
class File(gql.Node):
    project_version: ProjectVersion
    revision: auto
    name: auto
    path: auto
    created_at: auto
    updated_at: auto
    deleted_at: auto
    directory: auto
    generated: auto
    parent: Optional["File"]  # containing folder
    files: list["File"]  # if folder
    # TODO @Cleanup: File.statements should be a connection (but strawberry errors)
    #  There is an error with double-prefetching type_nodes when using a connection.
    statements: list[Annotated["Statement", lazy(".statement")]] = gql.django.field(
        filters=StatementFilter
    )


@gql.input
class ProjectCreateInput:
    owner_id: GlobalID
    name: str
    slug: str
    visibility: ProjectVisibility
    type: ProjectType = ProjectType.EXECUTABLE


@gql.input
class ProjectUpdateVisibilityInput(gql.NodeInput):
    visibility: ProjectVisibility


@gql.input
class ProjectUpdateNameInput(gql.NodeInput):
    name: str


@gql.type
class ProjectMutation:
    @safe_mutation
    def create_project(self, info, input: "ProjectCreateInput") -> Project | OperationInfo:
        requesting_user = info.context.request.scope["user"]
        owner = input.owner_id.resolve_node(info, required=True)
        if not is_owner_or_member(requesting_user, owner):
            raise PermissionError("cannot create project for this owner")
        project = models.Project.objects.create_project(
            owner=owner,
            name=input.name,
            slug=input.slug,
            type=input.type,
            visibility=input.visibility,
            create_adhoc_deployment=True,
            create_onboarding_files=True,
        )
        return project

    @safe_mutation
    def update_project_visibility(
        self, info, input: "ProjectUpdateVisibilityInput"
    ) -> Project | OperationInfo:
        project = models.Project.objects.get(id=input.id.node_id)
        check_can_write_project(info, project)
        project.visibility = input.visibility
        project.save()
        return project

    @safe_mutation
    def update_project_name(self, info, input: "ProjectUpdateNameInput") -> Project | OperationInfo:
        project = models.Project.objects.get(id=input.id.node_id)
        check_can_write_project(info, project)
        project.name = input.name
        project.save()
        return project


@gql.input
class UpdateProjectVersion(gql.NodeInput):
    name: str
    tag: Optional[str] = None  # :ProjectVersionTags
    description: Optional[str] = None


@gql.input
class CommitInput:
    project_version_id: GlobalID
    name: Optional[str] = None
    tag: Optional[str] = None  # :ProjectVersionTags
    description: Optional[str] = None
    auto_deploy: bool = False


@gql.input
class RestoreInput:
    project_version_id: GlobalID


@gql.type
class CommitPayload:
    project: Project
    committed_version: ProjectVersion
    new_working_version: ProjectVersion


@gql.type
class ProjectVersionMutation:
    @safe_mutation
    def update_project_version(
        self, info, input: "UpdateProjectVersion"
    ) -> ProjectVersion | OperationInfo:
        project_v = models.ProjectVersion.objects.get(id=input.id.node_id)
        check_can_write_project(info, project_v)
        project_v.name = input.name
        project_v.description = input.description
        project_v.tag = input.tag
        project_v.save()
        return project_v

    @safe_mutation(atomic=True)
    def commit(self, info, input: CommitInput) -> CommitPayload | OperationInfo:
        project_v = models.ProjectVersion.objects.select_related("project").get(
            id=input.project_version_id.node_id
        )
        check_can_write_project(info, project_v)
        project = project_v.project
        if project_v.id != project.head_id:
            raise ValueError("cannot commit version that's not the head")

        if input.auto_deploy:
            models.Deployment.objects.filter(project_version=project_v).update(
                type=models.DeploymentType.MANUAL, status=models.DeploymentStatus.ACTIVE
            )

        new_head = project.create_version(
            parent=project_v,
            commit_name=input.name,
            commit_tag=input.tag,
            commit_description=input.description,
            auto_commit=True,
        )
        return CommitPayload(
            project=project,
            committed_version=project_v,
            new_working_version=new_head,
        )

    @safe_mutation(atomic=True)
    def restore(self, info: Info, input: RestoreInput) -> CommitPayload | OperationInfo:
        project_v = models.ProjectVersion.objects.select_related("project").get(
            id=input.project_version_id.node_id
        )
        check_can_write_project(info, project_v)
        project = project_v.project
        if project_v.id == project.head_id:
            raise ValueError("cannot restore version that's already the head")

        # auto-save current head
        old_head = project_v.project.head
        if not old_head.committed:
            old_head.commit(description="Snapshot before restoring another version", tag=None)

        # then restore working version to the selected version (new head)
        new_head = project.create_version(
            parent=project_v, description="Restore", auto_commit=False
        )
        project.head = new_head
        project.save()
        return CommitPayload(
            project=project,
            committed_version=old_head,
            new_working_version=new_head,
        )


#
# Project contents: files
# :ProjectContentSync

# For synchronizing file contents, to edit a file:
#  1. Check that the containing project version is not committed
#  2. Increment 'revision' on the file
#  [.. actual update ..]
#  3. Send zmq pub message
#


@gql.input
class FileCreateInput:
    id: Optional[GlobalID] = None
    project_version_id: GlobalID
    name: str
    path: str  # not used here, needed to update optimistically in client
    parent_id: Optional[GlobalID] = None
    directory: bool = False


@gql.input
class FileRenameInput(gql.NodeInput):
    name: str
    path: str  # unused, as in FileCreateInput


@gql.input
class FileMoveInput(gql.NodeInput):
    parent_id: Optional[GlobalID] = None
    path: str  # unused, as in FileCreateInput


@gql.type
class FileMutation:
    @project_mutation(PMT.CREATE_FILE)
    def create_file(self, input: FileCreateInput) -> File | OperationInfo:
        id = input.id.node_id if input.id else None
        return models.File(
            id=id,
            project_version_id=input.project_version_id.node_id,
            name=input.name,
            parent_id=input.parent_id.node_id if input.parent_id else None,
            directory=input.directory,
        )

    @project_mutation(PMT.SOFT_DELETE_FILE, atomic=True)
    def soft_delete_file(self, input: gql.NodeInput) -> File | OperationInfo:
        file = models.File.objects.get(id=input.id.node_id)
        file.soft_delete()
        return file

    @project_mutation(PMT.RESTORE_FILE, atomic=True)
    def restore_file(self, input: gql.NodeInput) -> File | OperationInfo:
        # use _base_manager since soft deleted files are not visible
        file = models.File._base_manager.get(id=input.id.node_id)
        file.restore()
        return file

    @project_mutation(PMT.MOVE_FILE)
    def move_file(self, input: FileMoveInput) -> File | OperationInfo:
        file = models.File.objects.get(id=input.id.node_id)
        file.parent_id = input.parent_id.node_id if input.parent_id else None
        return file

    @project_mutation(PMT.RENAME_FILE)
    def rename_file(self, input: FileRenameInput) -> File | OperationInfo:
        file = models.File.objects.get(id=input.id.node_id)
        file.name = input.name
        return file
