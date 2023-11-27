from typing import TYPE_CHECKING, Annotated, Optional, Union
from uuid import UUID

import strawberry
import strawberry_django
from strawberry import UNSET, auto, lazy, relay
from strawberry.relay import GlobalID
from strawberry_django.fields.types import OperationInfo

from bench import models
from bench.api.sync import bench_edit
from bench.api.utils import HasCrud, ModuleNode, Revisioned
from bench.language.edit import MET
from bench.utils.dt import utcnow_with_tz

if TYPE_CHECKING:
    from bench.api.interp import Issue
    from bench.api.project import ProjectVersion
    from bench.api.statement import Statement


@strawberry_django.filter(models.Statement)
class StatementFilter:
    is_visible: Optional[bool] = True

    def filter(self, queryset):
        if self.is_visible is not UNSET and self.is_visible is not None:
            queryset = queryset.filter(deleted_at__isnull=self.is_visible)
        return queryset


@strawberry_django.type(models.File)
class File(HasCrud, ModuleNode, Revisioned, relay.Node):
    project_version: Annotated["ProjectVersion", lazy(".project")]
    name: auto
    parent: Union[ModuleNode]
    statements: list[Annotated["Statement", lazy(".statement")]] = strawberry_django.field(
        filters=StatementFilter
    )
    issues: list[Annotated["Issue", lazy(".interp")]] = strawberry_django.field()


@strawberry.input
class FileCreateInput:
    id: GlobalID
    ck: UUID
    project_version_id: GlobalID
    name: str
    parent_id: Optional[GlobalID] = None


@strawberry.input
class FileUpdateInput(strawberry_django.NodeInput):
    name: str
    parent_id: Optional[GlobalID] = None


@strawberry.input
class FileDeleteInput(strawberry_django.NodeInput):
    pass


@strawberry.input
class FileRenameInput(strawberry_django.NodeInput):
    name: str


@strawberry.input
class FileMoveInput(strawberry_django.NodeInput):
    parent_id: Optional[GlobalID] = None


@strawberry.input
class FilePasteInput:
    source_id: GlobalID
    target_version_id: GlobalID
    target_id: GlobalID
    target_ck: UUID
    parent_id: Optional[GlobalID] = None


@strawberry.type
class FileMutation:
    @bench_edit(MET.CREATE_FILE)
    def create_file(self, input: FileCreateInput) -> File | OperationInfo:
        id = input.id.node_id if input.id else None
        return models.File(
            id=id,
            ck=input.ck,
            project_version_id=input.project_version_id.node_id,
            name=input.name,
            parent_file_id=input.parent_id.node_id if input.parent_id else None,
        )

    @bench_edit(MET.UPDATE_FILE)
    def update_file(self, input: FileUpdateInput) -> File | OperationInfo:
        file = models.File.objects.get(id=input.id.node_id)
        file.name = input.name
        file.parent_file_id = input.parent_id.node_id if input.parent_id else None
        return file

    @bench_edit(MET.DELETE_FILE)
    def delete_file(self, input: strawberry_django.NodeInput) -> File | OperationInfo:
        raise NotImplementedError

    @bench_edit(MET.SOFT_DELETE_FILE)
    def soft_delete_file(self, input: strawberry_django.NodeInput) -> File | OperationInfo:
        file = models.File.objects.get(id=input.id.node_id)
        file.deleted_at = utcnow_with_tz()
        return file

    @bench_edit(MET.RESTORE_FILE)
    def restore_file(self, input: strawberry_django.NodeInput) -> File | OperationInfo:
        # use _base_manager since soft deleted files are not visible
        file = models.File._base_manager.get(id=input.id.node_id)
        return file

    @bench_edit(MET.MOVE_FILE)
    def move_file(self, input: FileMoveInput) -> File | OperationInfo:
        file = models.File.objects.get(id=input.id.node_id)
        file.parent_file_id = input.parent_id.node_id if input.parent_id else None
        return file

    @bench_edit(MET.RENAME_FILE)
    def rename_file(self, input: FileRenameInput) -> File | OperationInfo:
        file = models.File.objects.get(id=input.id.node_id)
        file.name = input.name
        return file
