from typing import TYPE_CHECKING, Annotated, Optional, Union
from uuid import UUID

import strawberry
import strawberry_django
from django.core.exceptions import ValidationError
from strawberry import UNSET, auto, lazy, relay
from strawberry.relay import GlobalID
from strawberry.types import Info
from strawberry_django.fields.types import OperationInfo

from bench import models
from bench.api.auth import check_module_access, check_module_node_access
from bench.api.module import read_module_node
from bench.api.sync import tracked_db_mutation
from bench.api.utils import HasCrud, ModuleNode, Revisioned
from bench.language.mutate import MMT
from bench.models import ModuleAccessLevel

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
    @tracked_db_mutation(MMT.CREATE_FILE)
    def create_file(self, input: FileCreateInput) -> File | OperationInfo:
        id = input.id.node_id if input.id else None
        return models.File(
            id=id,
            ck=input.ck,
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
    def delete_file(self, input: strawberry_django.NodeInput) -> File | OperationInfo:
        file = models.File.objects.get(id=input.id.node_id)
        file.delete()
        return file

    @tracked_db_mutation(MMT.SOFT_DELETE_FILE, atomic=True)
    def soft_delete_file(self, input: strawberry_django.NodeInput) -> File | OperationInfo:
        file = models.File.objects.get(id=input.id.node_id)
        file.soft_delete()
        return file

    @tracked_db_mutation(MMT.RESTORE_FILE, atomic=True)
    def restore_file(self, input: strawberry_django.NodeInput) -> File | OperationInfo:
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

    @tracked_db_mutation(MMT.PASTE_FILE, atomic=True, skip_save=True, skip_auth_check=True)
    def paste_file(self, info: Info, input: FilePasteInput) -> File | OperationInfo:
        # get and check source/target
        source_file = models.File.objects.get(id=input.source_id.node_id)
        check_module_node_access(info, source_file, ModuleAccessLevel.Read)
        target_version = models.ProjectVersion.objects.get(id=input.target_version_id.node_id)
        check_module_access(info, target_version.project, ModuleAccessLevel.Edit)
        parent_file = (
            models.File.objects.get(id=input.parent_id.node_id) if input.parent_id else None
        )
        if parent_file is not None and parent_file.project_version_id != target_version.id:
            raise ValidationError("target version does not match parent file version")
        if input.target_id and models.File.objects.filter(id=input.target_id.node_id).exists():
            raise ValidationError("target file already exists")

        # copy file
        target_id = UUID(input.target_id.node_id)
        target_ck = input.target_ck
        target_file = models.File.objects.copy(
            file=source_file,
            source=source_file.project_version,
            target=target_version,
            target_id=target_id,
            target_ck=target_ck,
            keep_cks=False,
            include_interp=False,
        )
        root_fragment = info.selected_fields[0].selections[0]  # mutation, returned object is first
        resolved_file = read_module_node(info, target_file, root_fragment=root_fragment)
        resolved_file.project_version = target_version  # ensure real project version is set (auth)
        return resolved_file  # type: ignore
