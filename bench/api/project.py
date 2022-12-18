from typing import TYPE_CHECKING, Annotated, Optional

from strawberry import UNSET, lazy
from strawberry_django_plus import gql
from strawberry_django_plus.gql import auto
from strawberry_django_plus.relay import GlobalID

from bench import models

if TYPE_CHECKING:
    from bench.api.compilation import Compilation
    from bench.api.organization import Organization
    from bench.api.symbol import Statement


@gql.django.type(models.Project)
class Project(gql.Node):
    name: auto
    slug: auto
    organization: Annotated["Organization", lazy(".organization")]
    created_at: auto
    updated_at: auto
    head: "ProjectVersion"
    versions: list["ProjectVersion"]  # TODO @Cleanup: use relay connections


@gql.django.filter(models.Statement)
class StatementFilter:
    is_visible: Optional[bool] = True
    id: Optional[GlobalID]

    def filter(self, queryset):
        if self.is_visible is not UNSET and self.is_visible is not None:
            if self.is_visible:
                queryset = queryset.filter(deleted_at__isnull=True)
            else:
                queryset = queryset.filter(deleted_at__isnull=False)
        if self.id is not UNSET and self.id is not None:
            queryset = queryset.filter(id=self.id.node_id)
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
    main_program: Optional[Annotated["Statement", lazy(".symbol")]]
    files: list["File", FileFilter] = gql.django.field(filters=FileFilter)
    statements: list[Annotated["Statement", lazy(".symbol")]] = gql.django.field(
        filters=StatementFilter
    )
    compilations: list[Annotated["Compilation", lazy(".compilation")]]


@gql.django.type(models.File)
class File(gql.Node):
    project_version: ProjectVersion
    type: auto
    name: auto
    path: auto
    created_at: auto
    updated_at: auto
    deleted_at: auto
    is_folder: auto
    parent: Optional["File"]  # containing folder
    files: list["File"]  # if folder
    statements: list[Annotated["Statement", lazy(".symbol")]] = gql.django.field(
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
    @gql.mutation
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


@gql.django.input(models.File)
class FileCreateInput:
    project_version: auto
    type: auto
    name: auto
    is_folder: auto
    parent: auto


@gql.django.partial(models.File)
class FileRenameInput(gql.NodeInput):
    name: auto


@gql.input
class FileSoftDeleteInput(gql.NodeInput):
    pass


@gql.type
class FileSoftDeletePayload:
    file: File


@gql.input
class FileRestoreInput(gql.NodeInput):
    pass


@gql.type
class FileRestorePayload:
    file: File


@gql.type
class FileMutation:
    create_file: File = gql.django.create_mutation(FileCreateInput)
    rename_file: File = gql.django.update_mutation(FileRenameInput)

    @gql.mutation
    def soft_delete_file(self, input: FileSoftDeleteInput) -> File:
        file = models.File.objects.get(id=input.id.node_id)
        file.soft_delete()
        return file

    @gql.mutation
    def restore_file(self, input: FileRestoreInput) -> File:
        file = models.File._base_manager.get(id=input.id.node_id)
        file.restore()
        return file
