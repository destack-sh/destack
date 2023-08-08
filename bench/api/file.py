from typing import TYPE_CHECKING, Annotated, Optional
from uuid import UUID

from django.core.exceptions import ValidationError
from strawberry import UNSET, auto, lazy
from strawberry.types import Info
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID
from strawberry_django_plus.types import OperationInfo

from bench import models
from bench.api.auth import check_can_read_project, check_can_write_project
from bench.api.interp import IssueFilter
from bench.api.sync import tracked_db_mutation
from bench.api.utils import CrudModel, ModuleNode, Revisioned
from bench.language.mutate import MMT

if TYPE_CHECKING:
    from bench.api.interp import Issue
    from bench.api.project import ProjectVersion
    from bench.api.statement import Statement


@gql.django.filter(models.Statement)
class StatementFilter:
    is_visible: Optional[bool] = True

    def filter(self, queryset):
        if self.is_visible is not UNSET and self.is_visible is not None:
            queryset = queryset.filter(deleted_at__isnull=self.is_visible)
        return queryset


@gql.django.type(models.File)
class File(CrudModel, ModuleNode, Revisioned, gql.Node):
    project_version: Annotated["ProjectVersion", lazy(".project")]
    name: auto
    files: list["File"]  # if folder
    parent: ModuleNode
    statements: list[Annotated["Statement", lazy(".statement")]] = gql.django.field(
        filters=StatementFilter
    )
    issues: list[Annotated["Issue", lazy(".interp")]] = gql.django.field(filters=IssueFilter)


@gql.input
class FileCreateInput:
    id: Optional[GlobalID] = None
    project_version_id: GlobalID
    name: str
    parent_id: Optional[GlobalID] = None


@gql.input
class FileDeleteInput(gql.NodeInput):
    pass


@gql.input
class FileRenameInput(gql.NodeInput):
    name: str


@gql.input
class FileMoveInput(gql.NodeInput):
    parent_id: Optional[GlobalID] = None


@gql.input
class FilePasteInput:
    source_id: GlobalID
    target_version_id: GlobalID
    target_id: Optional[GlobalID] = None
    parent_id: Optional[GlobalID] = None


@gql.type
class FileMutation:
    @tracked_db_mutation(MMT.CREATE_FILE)
    def create_file(self, input: FileCreateInput) -> File | OperationInfo:
        id = input.id.node_id if input.id else None
        return models.File(
            id=id,
            project_version_id=input.project_version_id.node_id,
            name=input.name,
            parent_file_id=input.parent_id.node_id if input.parent_id else None,
        )

    @tracked_db_mutation(MMT.UPDATE_FILE)
    def update_file(self, input: FileCreateInput) -> File | OperationInfo:
        file = models.File.objects.get(id=input.id.node_id)
        file.name = input.name
        file.parent_file_id = input.parent_id.node_id if input.parent_id else None
        return file

    @tracked_db_mutation(MMT.DELETE_FILE, atomic=True)
    def delete_file(self, input: gql.NodeInput) -> File | OperationInfo:
        file = models.File.objects.get(id=input.id.node_id)
        file.delete()
        return file

    @tracked_db_mutation(MMT.SOFT_DELETE_FILE, atomic=True)
    def soft_delete_file(self, input: gql.NodeInput) -> File | OperationInfo:
        file = models.File.objects.get(id=input.id.node_id)
        file.soft_delete()
        return file

    @tracked_db_mutation(MMT.RESTORE_FILE, atomic=True)
    def restore_file(self, input: gql.NodeInput) -> File | OperationInfo:
        # use _base_manager since soft deleted files are not visible
        file = models.File._base_manager.get(id=input.id.node_id)
        file.restore()
        return file

    @tracked_db_mutation(MMT.MOVE_FILE)
    def move_file(self, input: FileMoveInput) -> File | OperationInfo:
        file = models.File.objects.get(id=input.id.node_id)
        file.parent_file_id = input.parent_id.node_id if input.parent_id else None
        return file

    @tracked_db_mutation(MMT.RENAME_FILE)
    def rename_file(self, input: FileRenameInput) -> File | OperationInfo:
        file = models.File.objects.get(id=input.id.node_id)
        file.name = input.name
        return file

    @tracked_db_mutation(MMT.PASTE_FILE, atomic=True, skip_auth_check=True)
    def paste_file(self, info: Info, input: FilePasteInput) -> File | OperationInfo:
        from bench.models import RefMappingKind

        # get and check source/target
        source_file = models.File.objects.get(id=input.source_id.node_id)
        check_can_read_project(info, source_file)
        target_version = models.ProjectVersion.objects.get(id=input.target_version_id.node_id)
        check_can_write_project(info, target_version.project)
        parent_file = (
            models.File.objects.get(id=input.parent_id.node_id) if input.parent_id else None
        )
        if parent_file is not None and parent_file.project_version_id != target_version.id:
            raise ValidationError("target version does not match parent file version")
        if input.target_id and models.File.objects.filter(id=input.target_id.node_id).exists():
            raise ValidationError("target file already exists")

        # copy file
        target_id = UUID(input.target_id.node_id) if input.target_id else None
        target_file = models.File.objects.copy(
            file=source_file,
            source=source_file.project_version,
            target=target_version,
            target_id=target_id,
            kind=RefMappingKind.PASTE,
        )
        return target_file
