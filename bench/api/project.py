from typing import TYPE_CHECKING, Annotated, Optional

from strawberry import UNSET, lazy
from strawberry_django_plus import gql
from strawberry_django_plus.gql import auto
from strawberry_django_plus.relay import GlobalID

from bench import models
from bench.api.common import async_safe_mutation
from bench.models import StatementType
from bench.models.project import RefDict

if TYPE_CHECKING:
    from bench.api.organization import Organization
    from bench.api.symbol import Statement


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
    organization: Annotated["Organization", lazy(".organization")]
    created_at: auto
    updated_at: auto
    head: "ProjectVersion"
    # TODO @Cleanup: use relay connections for (large?) relations
    versions: list["ProjectVersion"] = gql.django.field(filters=ProjectVersionFilter)


@gql.django.filter(models.Statement)
class StatementFilter:
    is_visible: Optional[bool] = True
    type: Optional[StatementType] = None
    id: Optional[GlobalID]

    def filter(self, queryset):
        if self.is_visible is not UNSET and self.is_visible is not None:
            if self.is_visible:
                queryset = queryset.filter(deleted_at__isnull=True)
            else:
                queryset = queryset.filter(deleted_at__isnull=False)
        if self.type is not UNSET and self.type is not None:
            queryset = queryset.filter(type=self.type)
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
    main_program: Optional[Annotated["Statement", lazy(".symbol")]]
    files: list["File", FileFilter] = gql.django.field(filters=FileFilter)
    statements: list[Annotated["Statement", lazy(".symbol")]] = gql.django.field(
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
    type: auto
    name: auto
    path: auto
    path_without_extension: auto
    created_at: auto
    updated_at: auto
    deleted_at: auto
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


@gql.django.input(models.File)
class FileCreateInput:
    project_version: auto
    type: auto
    name: auto
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

    @async_safe_mutation
    def soft_delete_file(self, input: FileSoftDeleteInput) -> File:
        file = models.File.objects.get(id=input.id.node_id)
        file.soft_delete()
        return file

    @async_safe_mutation
    def restore_file(self, input: FileRestoreInput) -> File:
        file = models.File._base_manager.get(id=input.id.node_id)
        file.restore()
        return file
