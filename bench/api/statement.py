from datetime import datetime
from typing import TYPE_CHECKING, Annotated, Iterable, Optional
from uuid import UUID

import pytz
import structlog
from django.core.exceptions import ValidationError
from django.db.models import F
from strawberry import UNSET, lazy
from strawberry.types import Info
from strawberry_django_plus import gql
from strawberry_django_plus.gql import auto
from strawberry_django_plus.relay import GlobalID
from strawberry_django_plus.types import OperationInfo

from bench import bench as language
from bench import models
from bench.api.auth import check_can_read_project, check_can_write_project
from bench.api.interp import Issue
from bench.api.sync import MMT, BatchMutationInput, tracked_mutation

if TYPE_CHECKING:
    from bench.api.project import File, ProjectVersion

log = structlog.get_logger(__name__)

StatementType = gql.enum(language.StatementType)
SymbolType = gql.enum(language.SymbolType)
ExpectationModifier = gql.enum(language.ExpectationModifier)


@gql.django.filter(models.Field)
class FieldFilter:
    is_visible: Optional[bool] = True

    def filter(self, queryset):
        if self.is_visible is not UNSET and self.is_visible is not None:
            queryset = queryset.filter(deleted_at__isnull=self.is_visible)
        return queryset


TypeTag = gql.enum(language.TypeTag)
TypeHint = gql.enum(language.TypeHint)


@gql.interface
class SimplyTyped:
    """Anything typed using SimpleType nodes."""

    root_type_tag: Optional[TypeTag]
    fields: Optional[list["SimpleType"]]


@gql.interface
class SimpleType:
    id: GlobalID
    name: Optional[str]
    key: str
    order_key: str
    tag: TypeTag
    hint: Optional[TypeHint]
    flags: int
    description: Optional[str]
    reference: Optional["Statement"]


@gql.django.type(models.Field)
class Field(gql.Node, SimpleType):
    statement: "Statement"
    revision: auto
    created_at: auto
    updated_at: auto
    deleted_at: auto
    name: auto
    key: auto
    order_key: auto
    tag: TypeTag
    hint: Optional[TypeHint]
    flags: int
    description: auto
    reference: Optional["Statement"]


@gql.django.type(models.Statement)
class Statement(gql.Node, SimplyTyped):
    project_version: Annotated["ProjectVersion", lazy(".project")]
    file: Annotated["File", lazy(".project")]
    revision: auto
    type: StatementType
    modifier: auto
    name: auto
    created_at: auto
    updated_at: auto
    deleted_at: auto
    commented: auto
    parent: Optional["Statement"]
    children: list["Statement"]
    descendants: list["Statement"]
    order_key: auto
    text: auto
    symbol_type: Optional[SymbolType]
    # symbol contents
    root_type_tag: Optional[TypeTag]
    root_type_flags: Optional[int]
    fields: list[Field] = gql.django.field(filters=FieldFilter)
    lang: auto
    code: auto
    description: auto
    reference_project_version: Optional[Annotated["ProjectVersion", lazy(".project")]]
    # interp
    issues: Optional[list[Issue]]
    resolved_fields: Optional[list[Field]] = gql.django.field(filters=FieldFilter)


#
# Project contents: statements (in files)
# :ProjectContentSync
#
# For synchronizing statements, to edit a statement:
#  1. Check that the containing project version is not committed
#  2. Increment 'revision' on the statement
#  [.. actual update ..]
#  3. Send pub message
#


@gql.input
class StatementCreateInput:
    """Creates a full statement"""

    id: Optional[GlobalID] = None
    file_id: GlobalID
    order_key: str
    type: StatementType
    parent_id: Optional[GlobalID] = None
    commented: Optional[bool] = None
    modifier: Optional[ExpectationModifier] = None
    name: Optional[str] = None
    root_type_tag: Optional[TypeTag] = None
    root_type_flags: Optional[int] = None
    symbol_type: Optional[SymbolType] = None
    description: Optional[str] = None
    lang: Optional[str] = None
    text: Optional[str] = None
    code: Optional[str] = None


@gql.input
class StatementDeleteInput(gql.NodeInput):
    pass


@gql.input
class StatementMorphInput(gql.NodeInput):
    type: StatementType
    symbol_type: Optional[SymbolType] = None
    name: Optional[str] = None
    root_type_tag: Optional[TypeTag] = None
    root_type_flags: Optional[int] = None
    lang: Optional[str] = None


@gql.input
class StatementSetExpectationModifierInput(gql.NodeInput):
    modifier: Optional[ExpectationModifier]


@gql.django.partial(models.Statement)
class StatementRenameInput(gql.NodeInput):
    name: auto


@gql.input
class StatementMoveInput(gql.NodeInput):
    file_id: GlobalID
    parent_id: Optional[GlobalID] = None
    order_key: Optional[str] = None


@gql.input
class StatementSoftDeleteInput(gql.NodeInput):
    pass


@gql.input
class StatementRestoreInput(gql.NodeInput):
    pass


@gql.input
class StatementCommentedInput(gql.NodeInput):
    commented: bool


# batch operations


@gql.input
class StatementBatchSoftDeleteInput(BatchMutationInput):
    ids: list[GlobalID]

    def unbatch(self) -> list[StatementSoftDeleteInput]:
        return [StatementSoftDeleteInput(id=id) for id in self.ids]


@gql.input
class StatementBatchRestoreInput(BatchMutationInput):
    ids: list[GlobalID]

    def unbatch(self) -> list[StatementRestoreInput]:
        return [StatementRestoreInput(id=id) for id in self.ids]


@gql.input
class StatementBatchCommentedInput(BatchMutationInput):
    ids: list[GlobalID]
    commented: bool

    def unbatch(self) -> list[StatementCommentedInput]:
        return [
            StatementCommentedInput(
                id=id,
                commented=self.commented,
            )
            for id in self.ids
        ]


@gql.input
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


@gql.input
class StatementBatchPasteInput:
    source_ids: list[GlobalID]
    target_ids: list[GlobalID]
    target_file_id: GlobalID
    target_parent_ids: list[Optional[GlobalID]]
    target_order_keys: list[str]


@gql.input
class StatementUpdateTextInput(gql.NodeInput):
    text: Optional[str] = None


class ThingBatch(Iterable):
    @property
    def things(self):
        raise NotImplementedError

    # pretend to be an iterable for simpler perms checking
    # (doesn't need to know about the Batch type, which is
    #  required because we can't union list[Statement] | OperationInfo)

    def __getitem__(self, item):
        return self.things[item]

    def __len__(self):
        return len(self.things)

    def __iter__(self):
        return iter(self.things)


@gql.type
class StatementBatch(ThingBatch):
    statements: list[Statement]

    @property
    def things(self):
        return self.statements


@gql.type
class StatementMutation:
    @tracked_mutation(MMT.CREATE_STATEMENT)
    def create_statement(self, input: StatementCreateInput) -> Statement | OperationInfo:
        file = models.File.objects.get(id=input.file_id.node_id)
        statement = models.Statement(
            id=(input.id.node_id if input.id else None),
            project_version=file.project_version,
            file=file,
            type=input.type,
            name=input.name,
            parent_id=input.parent_id.node_id if input.parent_id else None,
            order_key=input.order_key,
            commented=input.commented,
            modifier=input.modifier,
            root_type_tag=input.root_type_tag,
            root_type_flags=input.root_type_flags,
            symbol_type=input.symbol_type,
            description=input.description,
            lang=input.lang,
            code=input.code,
            text=input.text,
        )
        return statement

    @tracked_mutation(MMT.UPDATE_STATEMENT)
    def update_statement(self, input: StatementCreateInput) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        if statement.symbol_type != input.symbol_type:
            raise ValueError(
                f"cannot change symbol type: {statement.symbol_type} -> {input.symbol_type}"
            )
        statement.type = input.type
        statement.name = input.name
        statement.file_id = input.file_id.node_id
        statement.parent_id = input.parent_id.node_id if input.parent_id else None
        statement.order_key = input.order_key
        statement.revision = input.revision
        statement.commented = input.commented
        statement.modifier = input.modifier
        statement.root_type_tag = input.root_type_tag
        statement.root_type_flags = input.root_type_flags
        statement.symbol_type = input.symbol_type
        statement.description = input.description
        statement.lang = input.lang
        statement.code = input.code
        statement.text = input.text
        return statement

    @tracked_mutation(MMT.DELETE_STATEMENT)
    def delete_statement(self, input: StatementDeleteInput) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.delete()
        return statement

    @tracked_mutation(MMT.MORPH_STATEMENT, atomic=True)
    def morph_statement(self, input: StatementMorphInput) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.type = input.type
        statement.symbol_type = input.symbol_type
        statement.name = input.name
        statement.root_type_tag = input.root_type_tag
        statement.root_type_flags = input.root_type_flags
        statement.lang = input.lang
        return statement

    @tracked_mutation(MMT.RENAME_STATEMENT)
    def rename_statement(self, input: StatementRenameInput) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.name = input.name
        return statement

    @tracked_mutation(MMT.SOFT_DELETE_STATEMENT, atomic=True)
    def soft_delete_statement(self, input: StatementSoftDeleteInput) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.soft_delete()
        return statement

    @tracked_mutation(MMT.RESTORE_STATEMENT, atomic=True)
    def restore_statement(self, input: StatementRestoreInput) -> Statement | OperationInfo:
        # use base manager since default manager excludes soft deleted statements
        statement = models.Statement._base_manager.get(id=input.id.node_id)
        statement.restore()
        return statement

    @tracked_mutation(MMT.UPDATE_SYMBOL_MODIFIER)
    def update_statement_modifier(
        self, input: StatementSetExpectationModifierInput
    ) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.modifier = input.modifier
        return statement

    @tracked_mutation(MMT.COMMENT_STATEMENT, atomic=True)
    def comment_statement(self, input: StatementCommentedInput) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.set_commented(input.commented)
        return statement

    @tracked_mutation(MMT.MOVE_STATEMENT, atomic=True)
    def move_statement(self, input: StatementMoveInput) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.file_id = UUID(input.file_id.node_id)
        statement.parent_id = UUID(input.parent_id.node_id) if input.parent_id else None
        # :CircularAncestry
        # TODO @Robustness:: check for circular ancestry via parent_id on move
        if statement.parent_id == statement.id:
            raise ValidationError("circular ancestry")
        # TODO @Robustness: return a different order key if conflict on move/insert
        statement.order_key = input.order_key
        return statement

    @tracked_mutation(MMT.SOFT_DELETE_STATEMENT, atomic=True, batch=True, register=False)
    def batch_soft_delete_statement(
        self, input: StatementBatchSoftDeleteInput
    ) -> StatementBatch | OperationInfo:
        statement_ids = [UUID(i.node_id) for i in input.ids]
        # imitate Statement.soft_delete but for a batch
        deleted_at = datetime.utcnow().replace(tzinfo=pytz.utc)
        models.Statement.objects.filter(id__in=statement_ids).update(deleted_at=deleted_at)
        models.Statement.objects.get_descendants(statement_ids, deleted_at=None).update(
            deleted_at=deleted_at
        )
        # use base manager since the statements are now deleted
        statements = models.Statement._base_manager.filter(id__in=statement_ids)
        return StatementBatch(statements=list(statements))

    @tracked_mutation(MMT.RESTORE_STATEMENT, atomic=True, batch=True, register=False)
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

    @tracked_mutation(MMT.COMMENT_STATEMENT, atomic=True, batch=True, register=False)
    def batch_comment_statement(
        self, input: StatementBatchCommentedInput
    ) -> StatementBatch | OperationInfo:
        statement_ids = [UUID(i.node_id) for i in input.ids]
        statements = models.Statement.objects.filter(id__in=statement_ids)
        models.Statement.objects.get_descendants(statement_ids, deleted_at=None).update(
            commented=input.commented
        )
        return StatementBatch(statements=list(statements))

    @tracked_mutation(MMT.MOVE_STATEMENT, atomic=True, batch=True, register=False)
    def batch_move_statement(
        self, input: StatementBatchMoveInput
    ) -> StatementBatch | OperationInfo:
        statement_ids = [UUID(i.node_id) for i in input.ids]
        statements = models.Statement.objects.filter(id__in=statement_ids)
        file_id = UUID(input.file_id.node_id)
        for i, statement in enumerate(statements):
            statement.file_id = file_id
            statement.parent_id = UUID(input.parent_ids[i].node_id) if input.parent_ids[i] else None
            statement.order_key = input.order_keys[i]
            statement.revision = F("revision") + 1
        models.Statement.objects.bulk_update(
            statements, ["file_id", "parent_id", "order_key", "revision"]
        )
        # refresh revisions from DB
        new_revisions = models.Statement.objects.filter(id__in=statement_ids).values_list(
            "revision"
        )
        for i, statement in enumerate(statements):
            statement.revision = new_revisions[i][0]
        return StatementBatch(statements=list(statements))

    # we check auth manually here (simpler for copy/paste across projects & versions)
    @tracked_mutation(MMT.PASTE_STATEMENT, atomic=True, batch=True, skip_auth_check=True)
    def batch_paste_statement(
        self, info: Info, input: StatementBatchPasteInput
    ) -> StatementBatch | OperationInfo:
        source_ids = [UUID(i.node_id) for i in input.source_ids]
        source_statements = models.Statement._base_manager.prefetch_related(
            "records", "fields"
        ).filter(id__in=source_ids)
        if source_statements.count() != len(input.source_ids):
            raise ValidationError("statements not found")

        # check that the user can write the target
        source_file_ids = set(s.file_id for s in source_statements)
        target_file = models.File.objects.get(id=input.target_file_id.node_id)
        check_can_write_project(info, target_file)
        source_project_v = source_statements[0].project_version
        source_project_v_ids = set(s.project_version_id for s in source_statements)
        if len(source_project_v_ids) > 1:
            raise ValidationError("statements must be from the same project version")
        # check that the user can read the source (if different)
        if source_project_v != target_file.project_version:
            check_can_read_project(info, source_project_v.project)

        # actually paste and store paste refmappings
        target_ids = [UUID(i.node_id) for i in input.target_ids]
        target_parent_ids = {
            s: UUID(t.node_id) if t is not None else None
            for s, t in zip(target_ids, input.target_parent_ids)
        }
        ref_mappings = models.Statement.objects.copy_statements(
            statements=source_statements,
            target_files={s: target_file for s in source_file_ids},
            source_version=source_project_v,
            target_version=target_file.project_version,
            target_statement_ids={s: t for s, t in zip(source_ids, target_ids)},
            target_parent_ids=target_parent_ids,
            target_order_keys={s: t for s, t in zip(target_ids, input.target_order_keys)},
            copy_revisions=False,
        )
        for mapping in ref_mappings:
            mapping.kind = models.RefMappingKind.PASTE
        models.RefMapping.objects.bulk_create(ref_mappings)

        target_statements = models.Statement.objects.filter(id__in=target_ids)
        if target_statements.count() != len(input.target_ids):
            raise RuntimeError(
                f"failed to paste statements {target_statements} is incomplete (wanted {input.target_ids})"
            )
        return StatementBatch(statements=target_statements)

    @tracked_mutation(MMT.UPDATE_STATEMENT_TEXT)
    def update_statement_text(self, input: StatementUpdateTextInput) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.text = input.text
        return statement


#
# Statement content / symbol mutations
#


@gql.input
class StatementTextInput(gql.NodeInput):
    text: str


@gql.input
class SymbolUpdateDescriptionInput(gql.NodeInput):
    description: str


@gql.input
class SymbolUpdateCodeInput(gql.NodeInput):
    code: Optional[str] = None


@gql.input
class StatementUpdateLanguageInput(gql.NodeInput):
    language: str


@gql.input
class FieldCreateInput:
    id: GlobalID
    key: str
    order_key: str
    statement_id: GlobalID
    name: Optional[str] = None
    tag: TypeTag
    hint: Optional[TypeHint] = None
    description: Optional[str] = None
    flags: int = 0
    reference_id: Optional[GlobalID] = None


@gql.input
class FieldUpdateInput(gql.NodeInput):
    name: Optional[str] = None
    tag: TypeTag
    hint: Optional[TypeHint] = None
    description: Optional[str] = None
    flags: int = 0
    reference_id: Optional[GlobalID] = None


@gql.input
class FieldRenameInput(gql.NodeInput):
    name: Optional[str] = None


@gql.input
class FieldUpdateDescriptionInput(gql.NodeInput):
    description: Optional[str] = None


@gql.input
class FieldUpdateTypeInput(gql.NodeInput):
    tag: TypeTag
    hint: Optional[TypeHint] = None
    flags: int = 0
    reference_id: Optional[GlobalID] = None


@gql.input
class FieldMoveInput(gql.NodeInput):
    order_key: str


@gql.input
class FieldDeleteInput(gql.NodeInput):
    pass


@gql.input
class FieldRestoreInput(gql.NodeInput):
    pass


@gql.type
class SymbolMutation:
    @tracked_mutation(MMT.UPDATE_SYMBOL_DESCRIPTION)
    def update_symbol_description(
        self, input: SymbolUpdateDescriptionInput
    ) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.description = input.description
        return statement

    @tracked_mutation(MMT.UPDATE_SYMBOL_CODE)
    def update_symbol_code(self, input: SymbolUpdateCodeInput) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.code = input.code
        return statement

    @tracked_mutation(MMT.CREATE_FIELD)
    def create_field(self, input: FieldCreateInput) -> Field | OperationInfo:
        field = models.Field(
            id=UUID(input.id.node_id),
            statement_id=UUID(input.statement_id.node_id),
            key=input.key,
            order_key=input.order_key,
            name=input.name,
            description=input.description,
            tag=input.tag,
            hint=input.hint,
            flags=input.flags,
            reference_id=UUID(input.reference_id.node_id) if input.reference_id else None,
        )
        return field

    @tracked_mutation(MMT.UPDATE_FIELD)
    def update_field(self, input: FieldUpdateInput) -> Field | OperationInfo:
        field = models.Field.objects.get(id=input.id.node_id)
        field.name = input.name
        field.description = input.description
        field.tag = input.tag
        field.hint = input.hint
        field.flags = input.flags
        field.reference_id = UUID(input.reference_id.node_id) if input.reference_id else None
        return field

    @tracked_mutation(MMT.RENAME_FIELD)
    def update_field_name(self, input: FieldRenameInput) -> Field | OperationInfo:
        field = models.Field.objects.get(id=input.id.node_id)
        field.name = input.name
        return field

    @tracked_mutation(MMT.UPDATE_FIELD_DESCRIPTION)
    def update_field_description(self, input: FieldUpdateDescriptionInput) -> Field | OperationInfo:
        field = models.Field.objects.get(id=input.id.node_id)
        field.description = input.description
        return field

    @tracked_mutation(MMT.UPDATE_FIELD_TYPE)
    def update_field_type(self, input: FieldUpdateTypeInput) -> Field | OperationInfo:
        field = models.Field.objects.get(id=input.id.node_id)
        field.tag = input.tag
        field.hint = input.hint
        field.flags = input.flags
        field.reference_id = UUID(input.reference_id.node_id) if input.reference_id else None
        return field

    @tracked_mutation(MMT.MOVE_FIELD)
    def move_field(self, input: FieldMoveInput) -> Field | OperationInfo:
        field = models.Field.objects.get(id=input.id.node_id)
        field.order_key = input.order_key
        return field

    @tracked_mutation(MMT.SOFT_DELETE_FIELD)
    def soft_delete_field(self, input: FieldDeleteInput) -> Field | OperationInfo:
        # use _base_manager since soft deleted type nodes are not visible
        field = models.Field._base_manager.get(id=input.id.node_id)
        field.soft_delete()
        return field

    @tracked_mutation(MMT.DELETE_FIELD)
    def delete_field(self, input: FieldDeleteInput) -> Field | OperationInfo:
        field = models.Field.objects.get(id=input.id.node_id)
        field.delete()
        return field

    @tracked_mutation(MMT.RESTORE_FIELD)
    def restore_statement_field(self, input: FieldRestoreInput) -> Field | OperationInfo:
        field = models.Field.objects.get(id=input.id.node_id)
        field.restore()
        return field
