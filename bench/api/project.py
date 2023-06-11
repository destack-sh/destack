from datetime import datetime
from typing import TYPE_CHECKING, Annotated, Optional, Union, cast
from uuid import UUID

import pytz
from django.core.exceptions import ValidationError
from strawberry import UNSET, lazy
from strawberry.types import Info
from strawberry_django_plus import gql
from strawberry_django_plus.gql import auto
from strawberry_django_plus.relay import GlobalID
from strawberry_django_plus.types import OperationInfo

import bench.bench.const
from bench import models
from bench.api.auth import can_write_project, check_can_write_project, is_owner_or_member
from bench.api.sync import MMT, tracked_mutation
from bench.api.utils import get_client_origin_from_info, safe_mutation
from bench.msg import NMessageType
from bench.msg.core import publish_soon
from bench.msg.messages import ProjectChangedPayload

if TYPE_CHECKING:
    from bench.api.organization import Organization
    from bench.api.statement import Statement
    from bench.api.user import User

StatementType = gql.enum(bench.bench.const.StatementType)


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


ProjectVisibility = gql.enum(models.ProjectVisibility)
ProjectType = gql.enum(models.ProjectType)


@gql.type
class ProjectMigrationInfo:
    is_reverse: bool
    source_version: "ProjectVersion"
    target_version: "ProjectVersion"
    ref_mappings: list["RefMapping"]


REF_TYPE_TO_TYPE_NAME = {
    models.RefType.FILE: "File",
    models.RefType.STATEMENT: "Statement",
    models.RefType.RECORD: "Record",
    models.RefType.FIELD: "Field",
}


@gql.django.type(models.Project)
class Project(gql.Node):
    name: auto
    slug: auto
    type: ProjectType
    visibility: ProjectVisibility
    path: auto
    description: auto
    owner: Union[Annotated["User", lazy(".user")], Annotated["Organization", lazy(".organization")]]
    created_at: auto
    updated_at: auto
    head: "ProjectVersion"
    versions: gql.relay.Connection["ProjectVersion"] = gql.django.connection(
        filters=ProjectVersionFilter
    )

    # TODO @Performance: specify only/select_related for can_write field
    @gql.field
    def can_write(self, info: OperationInfo) -> bool:
        user = cast(models.User, info.context.request.scope["user"]._wrapped)
        return can_write_project(user, self)

    @gql.field
    def migration_mappings(
        self, source_version_id: GlobalID, target_version_id: GlobalID
    ) -> ProjectMigrationInfo:
        source_version = models.ProjectVersion.objects.get(id=source_version_id.node_id)
        target_version = models.ProjectVersion.objects.get(id=target_version_id.node_id)
        if (
            source_version.project_id != target_version.project_id
            or source_version.project_id != self.id
        ):
            raise ValidationError("version ids belong to different projects")

        final_ref_mappings, is_reverse = models.ProjectVersion.objects.get_migration_mappings(
            UUID(source_version_id.node_id), UUID(target_version_id.node_id)
        )

        # map final ref mappings to global ids
        final_ref_mappings = [
            models.RefMapping(
                kind=ref_mapping.kind,
                type=ref_mapping.type,
                source_version=source_version,
                target_version=target_version,
                source_id=GlobalID(
                    REF_TYPE_TO_TYPE_NAME[ref_mapping.type], str(ref_mapping.source_id)
                ),
                source_revision=ref_mapping.source_revision,
                target_id=GlobalID(
                    REF_TYPE_TO_TYPE_NAME[ref_mapping.type], str(ref_mapping.target_id)
                ),
                target_revision=ref_mapping.target_revision,
            )
            for ref_mapping in final_ref_mappings
        ]

        return ProjectMigrationInfo(
            source_version=source_version,
            target_version=target_version,
            is_reverse=is_reverse,
            ref_mappings=final_ref_mappings,
        )


RefType = gql.enum(models.RefType)
RefMappingKind = gql.enum(models.RefMappingKind)


@gql.django.type(models.RefMapping)
class RefMapping(gql.Node):
    kind: RefMappingKind
    type: RefType
    source_version: "ProjectVersion"
    target_version: "ProjectVersion"
    source_id: GlobalID
    source_revision: int
    target_id: GlobalID
    target_revision: int

    @gql.field
    def source_version_id(self) -> GlobalID:
        return GlobalID("ProjectVersion", str(self.source_version_id))

    @gql.field
    def target_version_id(self) -> GlobalID:
        return GlobalID("ProjectVersion", str(self.target_version_id))


@gql.django.filter(models.RefMapping)
class RefMappingFilter:
    type: Optional[RefType] = None
    kind: Optional[RefMappingKind] = None

    def filter(self, queryset):
        if self.type is not None:
            queryset = queryset.filter(type=self.type)
        if self.kind is not None:
            queryset = queryset.filter(kind=self.kind)
        return queryset


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
    files: gql.relay.Connection["File"] = gql.django.connection(filters=FileFilter)
    child_refs: gql.relay.Connection[RefMapping] = gql.django.connection(filters=RefMappingFilter)
    parent_refs: gql.relay.Connection[RefMapping] = gql.django.connection(filters=RefMappingFilter)


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


@gql.input
class RestoreInput:
    project_version_id: GlobalID


@gql.type
class CommitPayload:
    project: Project
    committed_version: ProjectVersion


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
        head = models.ProjectVersion.objects.select_related("project").get(
            id=input.project_version_id.node_id
        )
        project = head.project
        if head.id != project.head_id:
            raise ValueError("cannot commit version that's not the head")
        check_can_write_project(info, head)

        # 'insert' new head between parents and head
        snapshot = models.ProjectVersion.objects.create(
            project=project,
            name=input.name,
            tag=input.tag,
            description=input.description,
            committed_at=datetime.utcnow().replace(tzinfo=pytz.utc),
        )
        snapshot.parents.set(head.parents.all())
        head.parents.set([snapshot])

        old_refmaps = list(head.parent_refs.filter(kind=RefMappingKind.COMMIT))
        new_refmaps = models.ProjectVersion.objects.copy(
            head, snapshot, invert_mappings=True, copy_revisions=True
        )

        # update previous head's target ref mappings to point to snapshot's refs
        new_refmaps_by_target_id = {refmap.target_id: refmap for refmap in new_refmaps}
        deleted_refmaps_ids = []
        for refmap in old_refmaps:
            if refmap.target_id not in new_refmaps_by_target_id:
                # we don't copy (soft) deleted objects, so the refmap cannot be updated
                deleted_refmaps_ids.append(refmap.id)
                continue
            new_refmap = new_refmaps_by_target_id[refmap.target_id]
            refmap.target_version_id = snapshot.id
            refmap.target_id = new_refmap.source_id
            refmap.target_revision = new_refmap.source_revision
        models.RefMapping.objects.bulk_update(
            old_refmaps, ["target_version_id", "target_id", "target_revision"]
        )
        models.RefMapping.objects.filter(id__in=deleted_refmaps_ids).delete()

        # publish
        origin = get_client_origin_from_info(info)
        publish_soon(
            NMessageType.PROJECT_CHANGED,
            ProjectChangedPayload(project_id=project.id, origins=[origin]),
        )
        return CommitPayload(project=project, committed_version=snapshot)

    @safe_mutation(atomic=True)
    def restore(self, info: Info, input: RestoreInput) -> CommitPayload | OperationInfo:
        to_restore = models.ProjectVersion.objects.select_related("project").get(
            id=input.project_version_id.node_id
        )
        check_can_write_project(info, to_restore)
        project = to_restore.project
        if to_restore.id == project.head_id:
            raise ValueError("cannot restore version that's already the head")

        # auto-snapshot current head
        old_head = to_restore.project.head
        if not old_head.committed:
            snapshot = models.ProjectVersion.objects.create(
                project=project,
                name="Autosave",
                tag=None,
                description="Autosave before restoring version",
                committed_at=datetime.utcnow().replace(tzinfo=pytz.utc),
            )
            snapshot.parents.set(old_head.parents.all())
            models.ProjectVersion.objects.copy(
                old_head, snapshot, invert_mappings=True, copy_revisions=True
            )

        # then restore working version to the selected version (new head)
        new_head = project.create_new_blank_head(parent=to_restore)
        project.head = new_head
        models.ProjectVersion.objects.copy(source=to_restore, target=new_head)
        project.save()

        # publish
        origin = get_client_origin_from_info(info)
        publish_soon(
            NMessageType.PROJECT_CHANGED,
            ProjectChangedPayload(project_id=project.id, origins=[origin]),
        )

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
#  3. Send pub message
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
class FileDeleteInput(gql.NodeInput):
    pass


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
    @tracked_mutation(MMT.CREATE_FILE)
    def create_file(self, input: FileCreateInput) -> File | OperationInfo:
        id = input.id.node_id if input.id else None
        return models.File(
            id=id,
            project_version_id=input.project_version_id.node_id,
            name=input.name,
            parent_id=input.parent_id.node_id if input.parent_id else None,
            directory=input.directory,
        )

    @tracked_mutation(MMT.UPDATE_FILE)
    def update_file(self, input: FileCreateInput) -> File | OperationInfo:
        file = models.File.objects.get(id=input.id.node_id)
        file.name = input.name
        file.parent_id = input.parent_id.node_id if input.parent_id else None
        file.directory = input.directory
        return file

    @tracked_mutation(MMT.DELETE_FILE, atomic=True)
    def delete_file(self, input: gql.NodeInput) -> File | OperationInfo:
        file = models.File.objects.get(id=input.id.node_id)
        file.delete()
        return file

    @tracked_mutation(MMT.SOFT_DELETE_FILE, atomic=True)
    def soft_delete_file(self, input: gql.NodeInput) -> File | OperationInfo:
        file = models.File.objects.get(id=input.id.node_id)
        file.soft_delete()
        return file

    @tracked_mutation(MMT.RESTORE_FILE, atomic=True)
    def restore_file(self, input: gql.NodeInput) -> File | OperationInfo:
        # use _base_manager since soft deleted files are not visible
        file = models.File._base_manager.get(id=input.id.node_id)
        file.restore()
        return file

    @tracked_mutation(MMT.MOVE_FILE)
    def move_file(self, input: FileMoveInput) -> File | OperationInfo:
        file = models.File.objects.get(id=input.id.node_id)
        file.parent_id = input.parent_id.node_id if input.parent_id else None
        return file

    @tracked_mutation(MMT.RENAME_FILE)
    def rename_file(self, input: FileRenameInput) -> File | OperationInfo:
        file = models.File.objects.get(id=input.id.node_id)
        file.name = input.name
        return file
