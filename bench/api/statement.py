from typing import TYPE_CHECKING, Annotated, Optional, Union
from uuid import UUID

import strawberry
import strawberry_django
import structlog
from django.core.exceptions import ValidationError
from django.db.models import F
from strawberry import UNSET, auto, lazy, relay
from strawberry.relay import GlobalID
from strawberry.scalars import JSON
from strawberry.types import Info
from strawberry_django.fields.types import OperationInfo

from bench import language, models
from bench.api.auth import check_project_access
from bench.api.interp import Issue, ResolvedField
from bench.api.sync import MMT, BatchMutationInput, tracked_db_mutation
from bench.api.utils import HasCrud, ModuleNode, Revisioned, ThingBatch
from bench.language import const
from bench.models import ProjectAccessLevel
from bench.utils.dt import utcnow_with_tz

if TYPE_CHECKING:
    from bench.api.project import File, ProjectVersion

log = structlog.get_logger(__name__)

StatementType = strawberry.enum(const.StatementType)


@strawberry_django.filter(models.Trigger)
class TriggerFilter:
    is_visible: Optional[bool] = True

    def filter(self, queryset):
        if self.is_visible is not UNSET and self.is_visible is not None:
            queryset = queryset.filter(deleted_at__isnull=self.is_visible)
        return queryset


@strawberry_django.filter(models.Tagging)
class TaggingFilter:
    is_visible: Optional[bool] = True

    def filter(self, queryset):
        if self.is_visible is not UNSET and self.is_visible is not None:
            queryset = queryset.filter(deleted_at__isnull=self.is_visible)
        return queryset


@strawberry_django.filter(models.Field)
class FieldFilter:
    is_visible: Optional[bool] = True

    def filter(self, queryset):
        if self.is_visible is not UNSET and self.is_visible is not None:
            queryset = queryset.filter(deleted_at__isnull=self.is_visible)
        return queryset


TypeStorageFormat = strawberry.enum(language.TypeStorageFormat)
TypeTag = strawberry.enum(language.TypeTag)
TriggerType = strawberry.enum(language.TriggerType)
ScheduleType = strawberry.enum(language.ScheduleType)
TypeHint = strawberry.enum(language.TypeHint)


@strawberry_django.type(models.Trigger)
class Trigger(HasCrud, ModuleNode, Revisioned, relay.Node):
    parent: "Statement" = strawberry_django.field(field_name="statement")
    type: TriggerType
    active: bool
    mapping: auto
    schedule_type: ScheduleType
    timezone: auto
    interval: auto
    cron: auto
    runnable_ck: auto
    scope_ck: auto


@strawberry_django.type(models.Tagging)
class Tagging(HasCrud, ModuleNode, Revisioned, relay.Node):
    parent: "Statement" = strawberry_django.field(field_name="statement")
    statement: "Statement"
    reference_ck: auto
    key: auto
    metadata: auto


@strawberry_django.type(models.Field)
class Field(HasCrud, ModuleNode, Revisioned, relay.Node):
    parent: "Statement" = strawberry_django.field(field_name="statement")
    statement: "Statement"
    name: auto
    key: auto
    order_key: auto
    tag: TypeTag
    hint: Optional[TypeHint]
    flags: int
    description: auto
    reference_ck: auto
    metadata: auto


@strawberry_django.type(models.Statement)
class Statement(HasCrud, ModuleNode, Revisioned, relay.Node):
    project_version: Annotated["ProjectVersion", lazy(".project")]
    file: Annotated["File", lazy(".file")]
    parent: Union[ModuleNode]
    descendants: list["Statement"]
    type: StatementType
    name: auto
    key: auto
    order_key: auto
    text: auto
    # symbol contents
    reference_ck: auto
    root_type_tag: Optional[TypeTag]
    root_type_flags: Optional[int]
    tags: list[Tagging] = strawberry_django.field(filters=TaggingFilter)
    triggers: list[Trigger] = strawberry_django.field(filters=TriggerFilter)
    fields: list[Field] = strawberry_django.field(filters=FieldFilter)
    lang: auto
    code: auto
    description: auto
    value: auto
    # interp
    issues: Optional[list[Issue]] = strawberry_django.field()
    resolved_fields: Optional[list[ResolvedField]] = strawberry_django.field()


@strawberry.input
class StatementCreateInput:
    """Creates a full statement"""

    id: GlobalID
    ck: UUID
    file_id: GlobalID
    order_key: str
    type: StatementType
    parent_id: Optional[GlobalID] = None
    name: Optional[str] = None
    root_type_tag: Optional[TypeTag] = None
    root_type_flags: Optional[int] = None
    description: Optional[str] = None
    lang: Optional[str] = None
    key: Optional[str] = None
    reference_ck: Optional[UUID] = None
    text: Optional[str] = None
    code: Optional[str] = None
    value: Optional[JSON] = None


@strawberry.input
class StatementUpdateInput(strawberry_django.NodeInput):
    """Updates a statement"""

    id: GlobalID
    order_key: Optional[str] = None
    type: Optional[StatementType] = None
    name: Optional[str] = None
    root_type_tag: Optional[TypeTag] = None
    root_type_flags: Optional[int] = None
    description: Optional[str] = None
    lang: Optional[str] = None
    key: Optional[str] = None
    reference_ck: Optional[UUID] = None
    text: Optional[str] = None
    code: Optional[str] = None
    value: Optional[JSON] = None


@strawberry.input
class StatementDeleteInput(strawberry_django.NodeInput):
    pass


@strawberry.input
class StatementMorphInput(strawberry_django.NodeInput):
    type: StatementType
    name: Optional[str] = None
    root_type_tag: Optional[TypeTag] = None
    root_type_flags: Optional[int] = None
    lang: Optional[str] = None


@strawberry_django.partial(models.Statement)
class StatementRenameInput(strawberry_django.NodeInput):
    name: auto


@strawberry.input
class StatementMoveInput(strawberry_django.NodeInput):
    file_id: GlobalID
    parent_id: Optional[GlobalID] = None
    order_key: Optional[str] = None


@strawberry.input
class StatementSoftDeleteInput(strawberry_django.NodeInput):
    pass


@strawberry.input
class StatementRestoreInput(strawberry_django.NodeInput):
    pass


# batch operations


@strawberry.input
class StatementBatchSoftDeleteInput(BatchMutationInput):
    ids: list[GlobalID]

    def unbatch(self) -> list[StatementSoftDeleteInput]:
        return [StatementSoftDeleteInput(id=id) for id in self.ids]


@strawberry.input
class StatementBatchRestoreInput(BatchMutationInput):
    ids: list[GlobalID]

    def unbatch(self) -> list[StatementRestoreInput]:
        return [StatementRestoreInput(id=id) for id in self.ids]


@strawberry.input
class StatementBatchMoveInput(BatchMutationInput):
    ids: list[GlobalID]
    file_id: GlobalID
    parent_ids: list[Optional[GlobalID]]
    order_keys: list[str]

    def unbatch(self) -> list[StatementMoveInput]:
        return [
            StatementMoveInput(
                id=id,
                file_id=self.file_id,
                parent_id=self.parent_ids[i],
                order_key=self.order_keys[i],
            )
            for i, id in enumerate(self.ids)
        ]


@strawberry.input
class StatementBatchPasteInput:
    source_ids: list[GlobalID]
    target_ids: list[GlobalID]
    target_cks: list[UUID]
    target_file_id: GlobalID
    target_parent_ids: list[Optional[GlobalID]]
    target_order_keys: list[str]


@strawberry.type
class StatementBatch(ThingBatch):
    statements: list[Statement]

    @property
    def things(self):
        return self.statements


@strawberry.type
class StatementMutation:
    @tracked_db_mutation(MMT.CREATE_STATEMENT, atomic=True)
    def create_statement(self, input: StatementCreateInput) -> Statement | OperationInfo:
        file = models.File.objects.get(id=input.file_id.node_id)
        parent_statement = (
            models.Statement.objects.filter(id=input.parent_id.node_id).first()
            if input.parent_id
            else None
        )
        statement = models.Statement(
            id=(UUID(input.id.node_id) if input.id else None),
            ck=input.ck,
            project_version=file.project_version,
            file=file,
            type=input.type,
            name=input.name,
            parent_statement=parent_statement,
            order_key=input.order_key,
            root_type_tag=input.root_type_tag,
            root_type_flags=input.root_type_flags,
            description=input.description,
            key=input.key,
            reference_ck=input.reference_ck,
            lang=input.lang,
            code=input.code,
            text=input.text,
            value=input.value,
        )
        return statement

    @tracked_db_mutation(MMT.UPDATE_STATEMENT, atomic=True)
    def update_statement(self, input: StatementUpdateInput) -> Statement | OperationInfo:
        raise NotImplementedError("only for sync")

    @tracked_db_mutation(MMT.MORPH_STATEMENT, atomic=True)
    def morph_statement(self, input: StatementMorphInput) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.type = input.type
        statement.name = input.name
        statement.root_type_tag = input.root_type_tag
        statement.root_type_flags = input.root_type_flags
        statement.lang = input.lang
        return statement

    @tracked_db_mutation(MMT.RENAME_STATEMENT)
    def rename_statement(self, input: StatementRenameInput) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.name = input.name
        return statement

    @tracked_db_mutation(MMT.SOFT_DELETE_STATEMENT, atomic=True)
    def soft_delete_statement(self, input: StatementSoftDeleteInput) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.soft_delete()
        return statement

    @tracked_db_mutation(MMT.RESTORE_STATEMENT, atomic=True)
    def restore_statement(self, input: StatementRestoreInput) -> Statement | OperationInfo:
        # use base manager since default manager excludes soft deleted statements
        statement = models.Statement._base_manager.get(id=input.id.node_id)
        statement.restore()
        return statement

    @tracked_db_mutation(MMT.DELETE_STATEMENT)
    def delete_statement(self, input: StatementDeleteInput) -> Statement | OperationInfo:
        raise NotImplementedError("only for sync")

    @tracked_db_mutation(MMT.MOVE_STATEMENT, atomic=True)
    def move_statement(self, input: StatementMoveInput) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.file_id = UUID(input.file_id.node_id)
        parent_statement = (
            models.Statement.objects.filter(id=input.parent_id.node_id).first()
            if input.parent_id
            else None
        )
        statement.parent_statement = parent_statement
        # :CircularAncestry
        # TODO @Robustness:: check for circular ancestry via parent_id on move
        if statement.parent_id == statement.id:
            raise ValidationError("circular ancestry")
        # TODO @Robustness: return a different order key if conflict on move/insert?
        statement.order_key = input.order_key
        return statement

    @tracked_db_mutation(MMT.SOFT_DELETE_STATEMENT, atomic=True, batch=True, register=False)
    def batch_soft_delete_statement(
        self, input: StatementBatchSoftDeleteInput
    ) -> StatementBatch | OperationInfo:
        statement_ids = [UUID(i.node_id) for i in input.ids]
        # imitate Statement.soft_delete but for a batch
        deleted_at = utcnow_with_tz()
        models.Statement.objects.filter(id__in=statement_ids).update(deleted_at=deleted_at)
        models.Statement.objects.get_descendants(statement_ids, deleted_at=None).update(
            deleted_at=deleted_at
        )
        # use base manager since the statements are now deleted
        statements = models.Statement._base_manager.filter(id__in=statement_ids)
        return StatementBatch(statements=list(statements))

    @tracked_db_mutation(MMT.RESTORE_STATEMENT, atomic=True, batch=True, register=False)
    def batch_restore_statement(
        self, input: StatementBatchRestoreInput
    ) -> StatementBatch | OperationInfo:
        statement_ids = [UUID(i.node_id) for i in input.ids]
        # imitate Statement.restore but for a batch
        deleted_at = models.Statement._base_manager.values_list("deleted_at", flat=True).get(
            id=statement_ids[0]
        )
        statements = models.Statement._base_manager.filter(id__in=statement_ids)
        models.Statement.objects.get_descendants(statement_ids, deleted_at=deleted_at).update(
            deleted_at=None
        )
        statements.update(deleted_at=None)
        return StatementBatch(statements=list(statements))

    @tracked_db_mutation(MMT.MOVE_STATEMENT, atomic=True, batch=True, register=False)
    def batch_move_statement(
        self, input: StatementBatchMoveInput
    ) -> StatementBatch | OperationInfo:
        statement_ids = [UUID(i.node_id) for i in input.ids]
        statements = models.Statement.objects.filter(id__in=statement_ids)
        statements_parents_by_id = {
            s.id: s
            for s in models.Statement.objects.filter(
                id__in=[UUID(i.node_id) for i in input.parent_ids if i is not None]
            )
        }
        file_id = UUID(input.file_id.node_id)
        for i, statement in enumerate(statements):
            statement.file_id = file_id
            parent_id = UUID(input.parent_ids[i].node_id) if input.parent_ids[i] else None
            statement.parent_statement = statements_parents_by_id.get(parent_id)
            statement.order_key = input.order_keys[i]
            statement.revision = F("revision") + 1
        models.Statement.objects.bulk_update(
            statements, ["file_id", "parent_statement_id", "order_key", "revision"]
        )
        # refresh revisions from DB
        new_revisions = models.Statement.objects.filter(id__in=statement_ids).values_list(
            "revision"
        )
        for i, statement in enumerate(statements):
            statement.revision = new_revisions[i][0]
        return StatementBatch(statements=list(statements))

    # we check auth manually here (simpler for copy/paste across projects & versions)
    @tracked_db_mutation(MMT.PASTE_STATEMENT, atomic=True, batch=True, skip_auth_check=True)
    def batch_paste_statement(
        self, info: Info, input: StatementBatchPasteInput
    ) -> StatementBatch | OperationInfo:
        source_ids = [UUID(i.node_id) for i in input.source_ids]
        source_statements = models.Statement._base_manager.filter(id__in=source_ids)
        source_cks = [s.ck for s in source_statements]
        if source_statements.count() != len(input.source_ids):
            raise ValidationError("statements not found")

        # check user access
        source_file_ids = set(s.file_id for s in source_statements)
        target_file = models.File.objects.get(id=input.target_file_id.node_id)
        source_project_v = source_statements[0].project_version
        source_project_v_ids = set(s.project_version_id for s in source_statements)
        if len(source_project_v_ids) > 1:
            raise ValidationError("statements must be from the same project version")
        if source_project_v != target_file.project_version:
            check_project_access(info, source_project_v.project, models.ProjectAccessLevel.Read)
        check_project_access(info, target_file.project_version.project, ProjectAccessLevel.Edit)

        # do the copy paste
        target_ids = [UUID(i.node_id) for i in input.target_ids]
        target_parent_ids = {
            **{
                s: UUID(t.node_id)
                for s, t in zip(target_ids, input.target_parent_ids)
                if t is not None
            },
            **{s: target_file.id for s in source_file_ids},
        }
        target_order_keys = {s: t for s, t in zip(target_ids, input.target_order_keys)}
        models.Statement.objects.copy(
            statements=source_statements,
            source=source_project_v,
            target=target_file.project_version,
            target_ids={s: t for s, t in zip(source_ids, target_ids)},
            target_cks={s: t for s, t in zip(source_cks, input.target_cks)},
            target_parent_ids=target_parent_ids,
            target_order_keys=target_order_keys,
            keep_cks=False,
            include_interp=False,
        )

        target_statements = models.Statement.objects.filter(id__in=target_ids)
        if target_statements.count() != len(input.target_ids):
            raise RuntimeError(
                f"paste {target_statements} is incomplete (wanted {input.target_ids})"
            )
        return StatementBatch(statements=target_statements)


#
# Statement content / symbol mutations
#


@strawberry.input
class StatementUpdateTextInput(strawberry_django.NodeInput):
    text: Optional[str] = None


@strawberry.input
class SymbolUpdateDescriptionInput(strawberry_django.NodeInput):
    description: str


@strawberry.input
class StatementUpdateReferenceInput(strawberry_django.NodeInput):
    reference_ck: Optional[UUID] = None


@strawberry.input
class SymbolUpdateCodeInput(strawberry_django.NodeInput):
    code: Optional[str] = None


@strawberry.input
class StatementUpdateLanguageInput(strawberry_django.NodeInput):
    language: str


@strawberry.input
class SymbolUpdateValueInput(strawberry_django.NodeInput):
    value: Optional[JSON] = None


@strawberry.input
class TaggingCreateInput:
    id: GlobalID
    ck: UUID
    statement_id: GlobalID
    key: str
    reference_ck: UUID
    metadata: Optional[JSON] = None


@strawberry.input
class TaggingUpdateInput(strawberry_django.NodeInput):
    metadata: Optional[JSON] = None


@strawberry.input
class TaggingDeleteInput(strawberry_django.NodeInput):
    pass


@strawberry.input
class TaggingSoftDeleteInput(strawberry_django.NodeInput):
    pass


@strawberry.input
class TaggingRestoreInput(strawberry_django.NodeInput):
    pass


@strawberry.input
class TriggerCreateInput:
    id: GlobalID
    ck: UUID
    statement_id: GlobalID
    type: TriggerType
    active: bool
    mapping: Optional[JSON] = None
    schedule_type: Optional[ScheduleType] = None
    timezone: Optional[str] = None
    interval: Optional[int] = None
    cron: Optional[str] = None
    runnable_ck: Optional[UUID] = None
    scope_ck: Optional[UUID] = None


@strawberry.input
class TriggerUpdateInput(strawberry_django.NodeInput):
    type: TriggerType
    active: bool
    mapping: Optional[JSON] = None
    schedule_type: Optional[ScheduleType] = None
    timezone: Optional[str] = None
    interval: Optional[int] = None
    cron: Optional[str] = None
    runnable_ck: Optional[UUID] = None
    scope_ck: Optional[UUID] = None


@strawberry.input
class TriggerDeleteInput(strawberry_django.NodeInput):
    pass


@strawberry.input
class TriggerSoftDeleteInput(strawberry_django.NodeInput):
    pass


@strawberry.input
class TriggerRestoreInput(strawberry_django.NodeInput):
    pass


@strawberry.input
class FieldCreateInput:
    id: GlobalID
    ck: UUID
    key: str
    order_key: str
    statement_id: GlobalID
    name: Optional[str] = None
    tag: TypeTag
    hint: Optional[TypeHint] = None
    description: Optional[str] = None
    flags: int = 0
    reference_ck: Optional[UUID] = None
    metadata: Optional[JSON] = None


@strawberry.input
class FieldUpdateInput(strawberry_django.NodeInput):
    name: Optional[str] = None
    tag: TypeTag
    hint: Optional[TypeHint] = None
    description: Optional[str] = None
    flags: int = 0
    reference_ck: Optional[UUID] = None
    metadata: Optional[JSON] = None


@strawberry.input
class FieldRenameInput(strawberry_django.NodeInput):
    name: Optional[str] = None


@strawberry.input
class FieldUpdateDescriptionInput(strawberry_django.NodeInput):
    description: Optional[str] = None


@strawberry.input
class FieldUpdateTypeInput(strawberry_django.NodeInput):
    tag: TypeTag
    hint: Optional[TypeHint] = None
    flags: int = 0
    reference_ck: Optional[GlobalID] = None


@strawberry.input
class FieldMoveInput(strawberry_django.NodeInput):
    order_key: str


@strawberry.input
class FieldDeleteInput(strawberry_django.NodeInput):
    pass


@strawberry.input
class FieldRestoreInput(strawberry_django.NodeInput):
    pass


@strawberry.type
class SymbolMutation:
    @tracked_db_mutation(MMT.UPDATE_STATEMENT_TEXT)
    def update_statement_text(self, input: StatementUpdateTextInput) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.text = input.text
        return statement

    @tracked_db_mutation(MMT.UPDATE_STATEMENT_REFERENCE)
    def update_statement_reference(
        self, input: StatementUpdateReferenceInput
    ) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.reference_ck = input.reference_ck
        return statement

    @tracked_db_mutation(MMT.UPDATE_SYMBOL_DESCRIPTION)
    def update_symbol_description(
        self, input: SymbolUpdateDescriptionInput
    ) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.description = input.description
        return statement

    @tracked_db_mutation(MMT.UPDATE_SYMBOL_CODE)
    def update_symbol_code(self, input: SymbolUpdateCodeInput) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.code = input.code
        return statement

    @tracked_db_mutation(MMT.UPDATE_SYMBOL_VALUE)
    def update_symbol_value(self, input: SymbolUpdateValueInput) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.value = input.value
        return statement

    @tracked_db_mutation(MMT.CREATE_FIELD)
    def create_field(self, input: FieldCreateInput) -> Field | OperationInfo:
        field = models.Field(
            id=UUID(input.id.node_id),
            ck=input.ck,
            statement_id=UUID(input.statement_id.node_id),
            key=input.key,
            order_key=input.order_key,
            name=input.name,
            description=input.description,
            tag=input.tag,
            hint=input.hint,
            flags=input.flags,
            reference_ck=input.reference_ck,
            metadata=input.metadata,
        )
        return field

    @tracked_db_mutation(MMT.UPDATE_FIELD)
    def update_field(self, input: FieldUpdateInput) -> Field | OperationInfo:
        field = models.Field.objects.get(id=input.id.node_id)
        field.name = input.name
        field.description = input.description
        field.tag = input.tag
        field.hint = input.hint
        field.flags = input.flags
        field.reference_ck = input.reference_ck
        field.metadata = input.metadata
        return field

    @tracked_db_mutation(MMT.RENAME_FIELD)
    def update_field_name(self, input: FieldRenameInput) -> Field | OperationInfo:
        field = models.Field.objects.get(id=input.id.node_id)
        field.name = input.name
        return field

    @tracked_db_mutation(MMT.UPDATE_FIELD_DESCRIPTION)
    def update_field_description(self, input: FieldUpdateDescriptionInput) -> Field | OperationInfo:
        field = models.Field.objects.get(id=input.id.node_id)
        field.description = input.description
        return field

    @tracked_db_mutation(MMT.UPDATE_FIELD_TYPE)
    def update_field_type(self, input: FieldUpdateTypeInput) -> Field | OperationInfo:
        field = models.Field.objects.get(id=input.id.node_id)
        field.tag = input.tag
        field.hint = input.hint
        field.flags = input.flags
        field.reference_ck = input.reference_ck
        return field

    @tracked_db_mutation(MMT.MOVE_FIELD)
    def move_field(self, input: FieldMoveInput) -> Field | OperationInfo:
        field = models.Field.objects.get(id=input.id.node_id)
        field.order_key = input.order_key
        return field

    @tracked_db_mutation(MMT.SOFT_DELETE_FIELD)
    def soft_delete_field(self, input: FieldDeleteInput) -> Field | OperationInfo:
        # use _base_manager since soft deleted type nodes are not visible
        field = models.Field._base_manager.get(id=input.id.node_id)
        field.soft_delete()
        return field

    @tracked_db_mutation(MMT.DELETE_FIELD)
    def delete_field(self, input: FieldDeleteInput) -> Field | OperationInfo:
        field = models.Field.objects.get(id=input.id.node_id)
        field.delete()
        return field

    @tracked_db_mutation(MMT.RESTORE_FIELD)
    def restore_field(self, input: FieldRestoreInput) -> Field | OperationInfo:
        field = models.Field.objects.get(id=input.id.node_id)
        field.restore()
        return field

    @tracked_db_mutation(MMT.CREATE_TAGGING)
    def create_tagging(self, input: TaggingCreateInput) -> Tagging | OperationInfo:
        tagging = models.Tagging(
            id=UUID(input.id.node_id),
            ck=input.ck,
            statement_id=UUID(input.statement_id.node_id),
            key=input.key,
            reference_ck=input.reference_ck,
            metadata=input.metadata,
        )
        return tagging

    @tracked_db_mutation(MMT.UPDATE_TAGGING)
    def update_tagging(self, input: TaggingUpdateInput) -> Tagging | OperationInfo:
        tagging = models.Tagging.objects.get(id=input.id.node_id)
        tagging.metadata = input.metadata
        return tagging

    @tracked_db_mutation(MMT.DELETE_TAGGING)
    def delete_tagging(self, input: TaggingDeleteInput) -> Tagging | OperationInfo:
        tagging = models.Tagging.objects.get(id=input.id.node_id)
        tagging.delete()
        return tagging

    @tracked_db_mutation(MMT.SOFT_DELETE_TAGGING)
    def soft_delete_tagging(self, input: TaggingDeleteInput) -> Tagging | OperationInfo:
        tagging = models.Tagging.objects.get(id=input.id.node_id)
        tagging.soft_delete()
        return tagging

    @tracked_db_mutation(MMT.RESTORE_TAGGING)
    def restore_tagging(self, input: TaggingRestoreInput) -> Tagging | OperationInfo:
        tagging = models.Tagging.objects.get(id=input.id.node_id)
        tagging.restore()
        return tagging

    @tracked_db_mutation(MMT.CREATE_TRIGGER)
    def create_trigger(self, input: TriggerCreateInput) -> Trigger | OperationInfo:
        trigger = models.Trigger(
            id=UUID(input.id.node_id),
            ck=input.ck,
            statement_id=UUID(input.statement_id.node_id),
            type=input.type,
            active=input.active,
            mapping=input.mapping,
            schedule_type=input.schedule_type,
            timezone=input.timezone,
            interval=input.interval,
            cron=input.cron,
            runnable_ck=input.runnable_ck,
            scope_ck=input.scope_ck,
        )
        return trigger

    @tracked_db_mutation(MMT.UPDATE_TRIGGER)
    def update_trigger(self, input: TriggerUpdateInput) -> Trigger | OperationInfo:
        trigger = models.Trigger.objects.get(id=input.id.node_id)
        trigger.type = input.type
        trigger.active = input.active
        trigger.mapping = input.mapping
        trigger.schedule_type = input.schedule_type
        trigger.timezone = input.timezone
        trigger.interval = input.interval
        trigger.cron = input.cron
        trigger.runnable_ck = input.runnable_ck
        trigger.scope_ck = input.scope_ck
        return trigger

    @tracked_db_mutation(MMT.DELETE_TRIGGER)
    def delete_trigger(self, input: TriggerDeleteInput) -> Trigger | OperationInfo:
        trigger = models.Trigger.objects.get(id=input.id.node_id)
        trigger.delete()
        return trigger

    @tracked_db_mutation(MMT.SOFT_DELETE_TRIGGER)
    def soft_delete_trigger(self, input: TriggerDeleteInput) -> Trigger | OperationInfo:
        trigger = models.Trigger.objects.get(id=input.id.node_id)
        trigger.soft_delete()
        return trigger

    @tracked_db_mutation(MMT.RESTORE_TRIGGER)
    def restore_trigger(self, input: TriggerRestoreInput) -> Trigger | OperationInfo:
        trigger = models.Trigger.objects.get(id=input.id.node_id)
        trigger.restore()
        return trigger
