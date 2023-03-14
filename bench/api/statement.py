from datetime import datetime
from typing import TYPE_CHECKING, Annotated, Iterable, Optional
from uuid import UUID

import pytz
from django.core.exceptions import ValidationError
from django.db.models import F
from strawberry import UNSET, lazy
from strawberry.scalars import JSON
from strawberry.types import Info
from strawberry_django_plus import gql
from strawberry_django_plus.gql import auto
from strawberry_django_plus.relay import GlobalID
from strawberry_django_plus.types import OperationInfo

from bench import language, models
from bench.api.auth import check_can_view_project, check_can_write_project
from bench.api.sync import PMT, project_mutation

if TYPE_CHECKING:
    from bench.api.project import File, ProjectVersion

StatementType = gql.enum(models.StatementType)
StatementModifier = gql.enum(language.StatementModifier)
SymbolType = gql.enum(models.SymbolType)


@gql.django.filter(models.DatasetRecord)
class DatasetRecordFilter:
    is_visible: Optional[bool] = True

    def filter(self, queryset):
        if self.is_visible is not UNSET and self.is_visible is not None:
            queryset = queryset.filter(deleted_at__isnull=self.is_visible)
        return queryset


@gql.django.filter(models.SimpleTypeNode)
class SimpleTypeNodeFilter:
    is_visible: Optional[bool] = True

    def filter(self, queryset):
        if self.is_visible is not UNSET and self.is_visible is not None:
            queryset = queryset.filter(deleted_at__isnull=self.is_visible)
        return queryset


@gql.type
class Type:
    description: Optional[str]
    btl: str


@gql.django.type(models.DatasetRecord)
class DatasetRecord(gql.Node):
    id: GlobalID
    statement: "Statement"
    revision: auto
    created_at: auto
    updated_at: auto
    deleted_at: auto
    order_key: str
    data: JSON


TypeTag = gql.enum(language.type.TypeTag)


@gql.interface
class SimplyTyped:
    """Anything typed using SimpleType nodes."""

    root_type_tag: Optional[TypeTag]
    type_nodes: Optional[list["SimpleType"]]


@gql.interface
class SimpleType:
    id: GlobalID
    name: Optional[str]
    order_key: str
    tag: TypeTag
    is_output: bool
    is_array: bool
    is_nullable: bool
    description: Optional[str]
    value: Optional[JSON]
    reference: Optional["Statement"]


@gql.django.type(models.SimpleTypeNode)
class SimpleTypeNode(gql.Node, SimpleType):
    statement: "Statement"
    revision: auto
    created_at: auto
    updated_at: auto
    deleted_at: auto
    name: auto
    order_key: auto
    tag: TypeTag
    is_output: auto
    is_array: auto
    is_nullable: auto
    description: auto
    value: auto
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
    generated: auto
    parent: Optional["Statement"]
    children: list["Statement"]
    descendants: list["Statement"]
    order_key: auto
    reference: Optional["Statement"]
    referenced_by: gql.relay.Connection["Statement"] = gql.django.connection()
    symbol_type: Optional[SymbolType]
    # symbol contents
    root_type_tag: Optional[TypeTag]
    type_nodes: list[SimpleTypeNode] = gql.django.field(filters=SimpleTypeNodeFilter)
    lang: auto
    code: auto
    description: auto
    reference_project_version: Optional[Annotated["ProjectVersion", lazy(".project")]]
    value: auto
    records: gql.relay.Connection[DatasetRecord] = gql.django.connection(
        filters=DatasetRecordFilter
    )


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
    """Creates a blank statement"""

    id: Optional[GlobalID] = None
    file_id: GlobalID
    order_key: str
    parent_id: Optional[GlobalID] = None


@gql.input
class StatementMorphInput(gql.NodeInput):
    type: StatementType
    symbol_type: Optional[SymbolType] = None
    name: Optional[str] = None
    root_type_tag: Optional[TypeTag] = None
    type_nodes: Optional[list["TypeNodeCreateInput"]] = None
    lang: Optional[str] = None


@gql.input
class StatementSetModifierInput(gql.NodeInput):
    modifier: Optional[StatementModifier]


@gql.input
class StatementSetReferenceInput(gql.NodeInput):
    reference_id: Optional[GlobalID] = None
    reference_name: Optional[str] = None  # unused, only for optimistic updates


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
class StatementBatchSoftDeleteInput:
    ids: list[GlobalID]
    deleted_at: Optional[datetime] = None


@gql.input
class StatementBatchRestoreInput:
    ids: list[GlobalID]


@gql.input
class StatementBatchCommentedInput:
    commented: bool
    ids: list[GlobalID]


@gql.input
class StatementBatchMoveInput:
    file_id: GlobalID
    parent_ids: list[Optional[GlobalID]]
    order_keys: list[str]
    ids: list[GlobalID]


@gql.input
class StatementBatchPasteInput:
    source_ids: list[GlobalID]
    target_ids: list[GlobalID]
    target_file_id: GlobalID
    target_parent_ids: list[Optional[GlobalID]]
    target_order_keys: list[str]


@gql.type
class StatementBatch(Iterable):
    statements: list[Statement]

    @property
    def things(self):
        return self.statements

    # pretend to be an iterable for simpler perms checking
    # (doesn't need to know about the Batch type, which is
    #  required because we can't union list[Statement] | OperationInfo)

    def __getitem__(self, item):
        return self.statements[item]

    def __len__(self):
        return len(self.statements)

    def __iter__(self):
        return iter(self.statements)


@gql.type
class StatementMutation:
    @project_mutation(PMT.CREATE_STATEMENT_BLANK)
    def create_statement(self, input: StatementCreateInput) -> Statement | OperationInfo:
        file = models.File.objects.get(id=input.file_id.node_id)
        project_version = file.project_version
        parent = (
            models.Statement.objects.get(id=input.parent_id.node_id) if input.parent_id else None
        )
        id = input.id.node_id if input.id else None
        statement = models.Statement(
            id=id,
            project_version=project_version,
            file=file,
            type=StatementType.BLANK,
            name=None,
            parent=parent,
            order_key=input.order_key,
        )
        return statement

    @project_mutation(PMT.MORPH_STATEMENT, atomic=True)
    def morph_statement(self, input: StatementMorphInput) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        if (
            input.type == StatementType.DEFINITION
            and input.symbol_type in (SymbolType.DATA, SymbolType.CODE, SymbolType.TASK)
            and input.root_type_tag is None
        ):
            raise ValidationError(f"root_type_tag is required for {input.type} {input.symbol_type}")

        statement.type = input.type
        statement.symbol_type = input.symbol_type
        statement.name = input.name
        statement.root_type_tag = input.root_type_tag
        # update related type nodes
        if input.type_nodes:
            SimpleTypeNode.objects.filter(statement=statement).delete()
            model_type_nodes = []
            for type_node in input.type_nodes:
                type_node = type_node.to_model()
                type_node.statement = statement
                type_node.full_clean()
                model_type_nodes.append(type_node)
            SimpleTypeNode.objects.bulk_create(model_type_nodes)
        statement.lang = input.lang
        return statement

    @project_mutation(PMT.RENAME_STATEMENT)
    def rename_statement(self, input: StatementRenameInput) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.name = input.name
        return statement

    @project_mutation(PMT.SOFT_DELETE_STATEMENT, atomic=True)
    def soft_delete_statement(self, input: StatementSoftDeleteInput) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.soft_delete()
        return statement

    @project_mutation(PMT.RESTORE_STATEMENT, atomic=True)
    def restore_statement(self, input: StatementRestoreInput) -> Statement | OperationInfo:
        # use base manager since default manager excludes soft deleted statements
        statement = models.Statement._base_manager.get(id=input.id.node_id)
        statement.restore()
        return statement

    @project_mutation(PMT.UPDATE_STATEMENT_MODIFIER)
    def update_statement_modifier(
        self, input: StatementSetModifierInput
    ) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.modifier = input.modifier
        return statement

    @project_mutation(PMT.UPDATE_STATEMENT_REFERENCE)
    def update_statement_reference(
        self, input: StatementSetReferenceInput
    ) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.reference_id = input.reference_id.node_id if input.reference_id else None
        return statement

    @project_mutation(PMT.COMMENT_STATEMENT, atomic=True)
    def comment_statement(self, input: StatementCommentedInput) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.set_commented(input.commented)
        return statement

    @project_mutation(PMT.MOVE_STATEMENT, atomic=True)
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

    @project_mutation(PMT.SOFT_DELETE_STATEMENT, atomic=True, batch=True)
    def batch_soft_delete_statement(
        self, input: StatementBatchSoftDeleteInput
    ) -> StatementBatch | OperationInfo:
        statement_ids = [UUID(i.node_id) for i in input.ids]
        # imitate Statement.soft_delete but for a batch
        deleted_at = input.deleted_at or datetime.utcnow().replace(tzinfo=pytz.utc)
        models.Statement.objects.filter(id__in=statement_ids).update(deleted_at=deleted_at)
        models.Statement.objects.get_descendants(statement_ids).filter(deleted_at=None).update(
            deleted_at=deleted_at
        )
        # use base manager since the statements are now deleted
        statements = models.Statement._base_manager.filter(id__in=statement_ids)
        return StatementBatch(statements=list(statements))

    @project_mutation(PMT.RESTORE_STATEMENT, atomic=True, batch=True)
    def batch_restore_statement(
        self, input: StatementBatchRestoreInput
    ) -> StatementBatch | OperationInfo:
        statement_ids = [UUID(i.node_id) for i in input.ids]
        # imitate Statement.restore but for a batch
        deleted_at = models.Statement._base_manager.values_list("deleted_at", flat=True).get(
            id=statement_ids[0]
        )
        statements = models.Statement._base_manager.filter(id__in=statement_ids)
        models.Statement.objects.get_descendants(statement_ids).filter(
            deleted_at=deleted_at
        ).update(deleted_at=None)
        statements.update(deleted_at=None)
        return StatementBatch(statements=list(statements))

    @project_mutation(PMT.COMMENT_STATEMENT, atomic=True, batch=True)
    def batch_comment_statement(
        self, input: StatementBatchCommentedInput
    ) -> StatementBatch | OperationInfo:
        statement_ids = [UUID(i.node_id) for i in input.ids]
        statements = models.Statement.objects.filter(id__in=statement_ids)
        models.Statement.objects.get_descendants(statement_ids).update(commented=input.commented)
        return StatementBatch(statements=list(statements))

    @project_mutation(PMT.MOVE_STATEMENT, atomic=True, batch=True)
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
    @project_mutation(PMT.CREATE_STATEMENT, atomic=True, batch=True, skip_auth_check=True)
    def batch_paste_statement(
        self, info: Info, input: StatementBatchPasteInput
    ) -> StatementBatch | OperationInfo:
        source_ids = [UUID(i.node_id) for i in input.source_ids]
        source_statements = models.Statement._base_manager.filter(id__in=source_ids)
        if source_statements.count() != len(input.source_ids):
            raise ValidationError("statements not found")

        # check that the user can read the source
        source_project_v = source_statements[0].project_version
        source_project_v_ids = set(s.project_version_id for s in source_statements)
        if len(source_project_v_ids) > 1:
            raise ValidationError("statements must be from the same project version")
        check_can_view_project(info, source_project_v.project)
        # check that the user can write the target
        source_file_ids = set(s.file_id for s in source_statements)
        target_file = models.File.objects.get(id=input.target_file_id.node_id)
        check_can_write_project(info, target_file)

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
            copy_generated_mappings=False,
            target_statement_ids={s: t for s, t in zip(source_ids, target_ids)},
            target_parent_ids=target_parent_ids,
            target_order_keys={s: t for s, t in zip(target_ids, input.target_order_keys)},
        )
        for mapping in ref_mappings:
            mapping.kind = models.RefMappingKind.PASTE
        models.RefMapping.objects.bulk_create(ref_mappings)

        target_statements = models.Statement.objects.filter(id__in=target_ids)
        if target_statements.count() != len(input.target_ids):
            raise RuntimeError(f"failed to paste statements {target_statements} is incomplete")
        return StatementBatch(statements=target_statements)


#
# Statement content / symbol mutations
#


@gql.input
class StatementTextInput(gql.NodeInput):
    text: str


@gql.input
class StatementUpdateDescriptionInput(gql.NodeInput):
    description: str


@gql.input
class StatementUpdateCodeInput(gql.NodeInput):
    code: Optional[str] = None


@gql.input
class StatementUpdateLanguageInput(gql.NodeInput):
    language: str


@gql.input
class RecordCreateInput(gql.NodeInput):
    statement_id: GlobalID
    data: JSON
    order_key: str


@gql.input
class RecordUpdateInput(gql.NodeInput):
    data: JSON


@gql.input
class RecordMoveInput(gql.NodeInput):
    order_key: str


@gql.input
class RecordDeleteInput(gql.NodeInput):
    pass


@gql.input
class TypeNodeCreateInput:
    """Upsert a statement type node data"""

    id: GlobalID
    order_key: str
    statement_id: GlobalID
    name: Optional[str] = None
    tag: TypeTag
    description: Optional[str] = None
    is_output: bool = False
    is_nullable: bool = False
    is_array: bool = False
    value: Optional[JSON] = None
    reference_id: Optional[GlobalID] = None

    def to_model(self) -> models.SimpleTypeNode:
        return models.SimpleTypeNode(
            id=UUID(self.id.node_id),
            order_key=self.order_key,
            name=self.name,
            description=self.description,
            tag=self.tag,
            is_output=self.is_output,
            is_nullable=self.is_nullable,
            is_array=self.is_array,
            value=self.value,
            reference_id=UUID(self.reference_id.node_id) if self.reference_id else None,
        )


@gql.input
class TypeNodeUpdateInput(gql.NodeInput):
    name: Optional[str] = None
    tag: TypeTag
    description: Optional[str] = None
    is_output: bool = False
    is_nullable: bool = False
    is_array: bool = False
    value: Optional[JSON] = None
    reference_id: Optional[GlobalID] = None


@gql.input
class TypeNodeMoveInput(gql.NodeInput):
    order_key: str


@gql.input
class TypeNodeDeleteInput(gql.NodeInput):
    pass


@gql.type
class SymbolMutation:
    # both text and code save to code, but UPDATE_STATEMENT_TEXT is more descriptive
    # and allows us to ignore comment updates trivially :StatementCodeTextReuse
    @project_mutation(PMT.UPDATE_STATEMENT_TEXT)
    def update_statement_text(self, input: StatementUpdateCodeInput) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.code = input.code
        return statement

    @project_mutation(PMT.UPDATE_STATEMENT_DESCRIPTION)
    def update_statement_description(
        self, input: StatementUpdateDescriptionInput
    ) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.description = input.description
        return statement

    @project_mutation(PMT.UPDATE_STATEMENT_CODE)
    def update_statement_code(self, input: StatementUpdateCodeInput) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.code = input.code
        return statement

    @project_mutation(PMT.UPDATE_STATEMENT_LANGUAGE)
    def update_statement_language(
        self, input: StatementUpdateLanguageInput
    ) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.language = input.language
        return statement

    @project_mutation(PMT.CREATE_STATEMENT_RECORD)
    def create_statement_record(self, input: RecordCreateInput) -> DatasetRecord | OperationInfo:
        statement = models.Statement.objects.get(id=input.statement_id.node_id)
        record = models.DatasetRecord(
            statement=statement,
            id=UUID(input.id.node_id),
            data=input.data,
            order_key=input.order_key,
        )
        return record

    @project_mutation(PMT.UPDATE_STATEMENT_RECORD)
    def update_statement_record(self, input: RecordUpdateInput) -> DatasetRecord | OperationInfo:
        record = models.DatasetRecord.objects.get(id=input.id.node_id)
        record.data = input.data
        return record

    @project_mutation(PMT.MOVE_STATEMENT_TYPE_NODE)
    def move_statement_record(self, input: RecordMoveInput) -> DatasetRecord | OperationInfo:
        record = models.DatasetRecord.objects.get(id=input.id.node_id)
        record.order_key = input.order_key
        return record

    @project_mutation(PMT.DELETE_STATEMENT_RECORD)
    def delete_statement_record(self, input: RecordDeleteInput) -> DatasetRecord | OperationInfo:
        record = models.DatasetRecord.objects.get(id=input.id.node_id)
        record.soft_delete()
        return record

    @project_mutation(PMT.CREATE_STATEMENT_RECORD)
    def restore_statement_record(self, input: RecordDeleteInput) -> DatasetRecord | OperationInfo:
        # use _base_manager since soft deleted records are not visible
        record = models.DatasetRecord._base_manager.get(id=input.id.node_id)
        record.restore()
        return record

    @project_mutation(PMT.CREATE_STATEMENT_TYPE_NODE)
    def create_statement_type_node(
        self, input: TypeNodeCreateInput
    ) -> SimpleTypeNode | OperationInfo:
        statement = models.Statement.objects.get(id=input.statement_id.node_id)
        type_node = input.to_model()
        type_node.statement = statement
        type_node.full_clean(validate_unique=False, validate_constraints=False)
        return type_node

    @project_mutation(PMT.UPDATE_STATEMENT_TYPE_NODE)
    def update_statement_type_node(
        self, input: TypeNodeUpdateInput
    ) -> SimpleTypeNode | OperationInfo:
        type_node = models.SimpleTypeNode.objects.get(id=input.id.node_id)
        type_node.name = input.name
        type_node.description = input.description
        type_node.tag = input.tag
        type_node.is_output = input.is_output
        type_node.is_nullable = input.is_nullable
        type_node.is_array = input.is_array
        type_node.value = input.value
        type_node.reference_id = UUID(input.reference_id.node_id) if input.reference_id else None
        type_node.full_clean(validate_unique=False, validate_constraints=False)
        return type_node

    @project_mutation(PMT.MOVE_STATEMENT_TYPE_NODE)
    def move_statement_type_node(self, input: TypeNodeMoveInput) -> SimpleTypeNode | OperationInfo:
        type_node = models.SimpleTypeNode.objects.get(id=input.id.node_id)
        type_node.order_key = input.order_key
        return type_node

    @project_mutation(PMT.UPDATE_STATEMENT_TYPE_NODE)
    def delete_statement_type_node(
        self, input: TypeNodeDeleteInput
    ) -> SimpleTypeNode | OperationInfo:
        # use _base_manager since soft deleted type nodes are not visible
        type_node = models.SimpleTypeNode._base_manager.get(id=input.id.node_id)
        type_node.soft_delete()
        return type_node

    @project_mutation(PMT.CREATE_STATEMENT_TYPE_NODE)
    def restore_statement_type_node(
        self, input: TypeNodeDeleteInput
    ) -> SimpleTypeNode | OperationInfo:
        type_node = models.SimpleTypeNode.objects.get(id=input.id.node_id)
        type_node.restore()
        return type_node
