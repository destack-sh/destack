from typing import TYPE_CHECKING, Annotated, Optional, Union

from strawberry import UNSET, lazy
from strawberry_django_plus import gql
from strawberry_django_plus.gql import auto
from strawberry_django_plus.relay import GlobalID
from strawberry_django_plus.types import OperationInfo

from bench import models
from bench.api.sync import PMT, project_mutation
from bench.api.util import async_safe_mutation
from bench.models.project import RefDict

if TYPE_CHECKING:
    from bench.api.organization import Organization
    from bench.api.statement import Statement
    from bench.api.user import User

StatementType = gql.enum(models.StatementType)


@gql.django.filter(models.ProjectVersion)
class ProjectVersionFilter:
    after_id: GlobalID = UNSET

    def filter_after_id(self, queryset):
        version = models.ProjectVersion.objects.get(id=self.after_id.node_id)
        return queryset.filter(created_at__gt=version.committed_at)


@gql.django.type(models.Project)
class Project(gql.Node):
    name: auto
    slug: auto
    path: auto
    owner: Union[Annotated["User", lazy(".user")], Annotated["Organization", lazy(".organization")]]
    created_at: auto
    updated_at: auto
    head: "ProjectVersion"
    # TODO @Cleanup: use relay connections for (large?) relations
    versions: list["ProjectVersion"] = gql.django.field(filters=ProjectVersionFilter)


@gql.django.filter(models.Statement)
class StatementFilter:
    is_visible: Optional[bool] = True

    def filter(self, queryset):
        if self.is_visible is not UNSET and self.is_visible is not None:
            if self.is_visible:
                queryset = queryset.filter(deleted_at__isnull=True)
            else:
                queryset = queryset.filter(deleted_at__isnull=False)
        return queryset


@gql.django.filter(models.File)
class FileFilter:
    is_visible: Optional[bool] = True

    def filter(self, queryset):
        if self.is_visible is None:
            return queryset
        elif self.is_visible:
            return queryset.filter(deleted_at__isnull=True)
        else:
            return queryset.filter(deleted_at__isnull=False)


@gql.type
class RefMapping:
    source: GlobalID
    target: GlobalID


@gql.django.type(models.ProjectVersion)
class ProjectVersion(gql.Node):
    project: Project
    name: auto
    description: auto
    parents: list["ProjectVersion"]
    children: list["ProjectVersion"]
    created_at: auto
    committed: auto
    committed_at: auto
    dependencies: list["ProjectVersion"]
    files: list["File"] = gql.django.field(filters=FileFilter)
    statements: list[Annotated["Statement", lazy(".statement")]] = gql.django.field(
        filters=StatementFilter
    )

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
    generated: auto
    parent: Optional["File"]  # containing folder
    files: list["File"]  # if folder
    statements: list[Annotated["Statement", lazy(".statement")]] = gql.django.field(
        filters=StatementFilter
    )


@gql.input
class CommitInput:
    project_version_id: GlobalID
    name: str
    description: Optional[str] = None


@gql.type
class CommitPayload:
    project: Project
    committed_version: ProjectVersion
    new_working_version: ProjectVersion


@gql.type
class ProjectVersionMutation:
    @async_safe_mutation
    def commit(self, input: CommitInput) -> CommitPayload:
        project_v = models.ProjectVersion.objects.select_related("project").get(
            id=input.project_version_id.node_id
        )
        project = project_v.project
        if project_v.id != project.head_id:
            raise ValueError("cannot commit version that's not the head")

        new_head = project.create_version(
            parent=project_v,
            commit_name=input.name,
            commit_description=input.description,
            auto_commit=True,
        )
        return CommitPayload(
            project=project,
            committed_version=project_v,
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
