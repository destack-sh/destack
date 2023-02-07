from dataclasses import asdict, replace
from typing import TYPE_CHECKING, Annotated, Optional
from uuid import UUID

from django import db
from django.core.exceptions import ValidationError
from django.db.models import F, Q, Value
from django.db.models.functions import Concat
from more_itertools import first
from strawberry import lazy
from strawberry.scalars import JSON
from strawberry_django_plus import gql
from strawberry_django_plus.gql import auto
from strawberry_django_plus.relay import GlobalID
from strawberry_django_plus.types import OperationInfo

from bench import language, models
from bench.api.sync import PMT, project_mutation
from bench.language import wire

if TYPE_CHECKING:
    from bench.api.project import File, ProjectVersion

StatementType = gql.enum(models.StatementType)
StatementModifier = gql.enum(language.StatementModifier)
SymbolType = gql.enum(models.SymbolType)


@gql.type
class Type:
    description: Optional[str]
    btl: str


@gql.type
class SourceMapping:
    source_id: UUID
    source_path: JSON
    source_revision: int
    target_id: UUID
    target_path: JSON
    target_revision: int


@gql.django.type(models.DatasetRecord)
class DatasetRecord:
    order_key: str
    data: JSON


TypeTag = gql.enum(language.type.TypeTag)


@gql.type
class TypeNodeData:
    id: GlobalID
    name: Optional[str]
    tag: TypeTag
    description: Optional[str]
    value: Optional[JSON]
    reference: Optional[str]
    parent_id: Optional[GlobalID]
    order_key: str


@gql.django.type(models.Statement)
class Statement(gql.Node):
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
    referenced_by: list["Statement"]
    symbol_type: Optional[SymbolType]
    text: auto
    # symbol contents
    lang: auto
    code: auto
    description: auto
    reference_project_version: Optional[Annotated["ProjectVersion", lazy(".project")]]
    value: auto
    records: list[DatasetRecord]
    mappings: list[SourceMapping]

    @gql.field(name="typeNodes")
    def type_nodes_(self) -> Optional[list[TypeNodeData]]:
        # map ids to global ids
        if self.type_nodes is None:
            return None
        return [
            replace(
                node,
                id=GlobalID("TypeNodeData", str(node.id)),
                parent_id=GlobalID("TypeNodeData", str(node.parent_id)) if node.parent_id else None,
            )
            for node in self.type_nodes
        ]

    @gql.field
    def import_path(self) -> Optional[str]:
        if self.type != StatementType.IMPORT:
            return None
        # TODO @Performance: statement import path should be batch loaded (DataLoader?)
        #  We try to only load the reference information required for the frontend,
        #  so we do some ugly SQL-side joins and concats.
        #  But maybe this is just the wrong approach overall, and we should just bake
        #  the import path into each project_version or even file already.
        is_local = Q(project_version_id=F("reference__project_version_id"))
        local_path = Concat(Value("."), F("reference__file__name"))
        absolute_path = Concat(
            F("reference__project_version__project__organization__slug"),
            Value("."),
            F("reference__project_version__project__slug"),
            local_path,
            output_field=db.models.CharField(),
        )
        import_path = db.models.Case(
            db.models.When(is_local, then=local_path),
            default=absolute_path,
            output_field=db.models.CharField(),
        )
        result = (
            models.Statement.objects.filter(id=self.id)
            .annotate(import_path=import_path)
            .values_list("import_path", flat=True)
            .first()
        )
        return result


#
# Project contents: statements (in files)
# :ProjectContentSync
#
# For synchronizing statements, to edit a statement:
#  1. Check that the containing project version is not committed
#  2. Increment 'revision' on the statement
#  [.. actual update ..]
#  3. Send zmq pub message
#


@gql.input
class StatementCreateInput:
    """Create a blank statement"""

    id: Optional[GlobalID] = None
    file_id: GlobalID
    order_key: str
    parent_id: Optional[GlobalID] = None


@gql.input
class StatementMorphInput(gql.NodeInput):
    type: StatementType
    symbol_type: Optional[SymbolType] = None
    name: Optional[str] = None
    type_nodes: Optional[list["StatementTypeNodeDataCreateInput"]] = None
    language: Optional[str] = None


@gql.input
class StatementSetModifierInput(gql.NodeInput):
    modifier: Optional[StatementModifier]


@gql.input
class StatementSetReferenceInput(gql.NodeInput):
    reference_id: Optional[GlobalID] = None


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


# TODO @Cleanup: type node mutations should be more atomic
@gql.input
class StatementTypeNodeDataCreateInput(gql.NodeInput):
    """Upsert a statement type node data"""

    node_id: GlobalID
    order_key: str
    parent_id: Optional[GlobalID] = None
    name: Optional[str] = None
    tag: TypeTag
    description: Optional[str] = None
    value: Optional[JSON] = None
    reference: Optional[str] = None

    def to_type_node_data(self) -> wire.TypeNodeData:
        return wire.TypeNodeData(
            id=UUID(self.node_id.node_id),
            order_key=self.order_key,
            parent_id=UUID(self.parent_id.node_id) if self.parent_id else None,
            name=self.name,
            tag=self.tag,
            description=self.description,
            value=self.value,
            reference=self.reference,
        )


@gql.input
class StatementTypeNodeDataDeleteInput(gql.NodeInput):
    node_id: GlobalID


@gql.type
class StatementMutation:
    @project_mutation(PMT.CREATE_STATEMENT)
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

    @project_mutation(PMT.MORPH_STATEMENT)
    def morph_statement(self, input: StatementMorphInput) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        if (
            input.type == StatementType.DEFINITION
            and input.symbol_type in (SymbolType.DATASET, SymbolType.CODE, SymbolType.TASK)
            and input.type_nodes is None
        ):
            raise ValidationError(f"type_nodes is required for {input.symbol_type}")

        statement.type = input.type
        statement.symbol_type = input.symbol_type
        statement.name = input.name
        statement.type_nodes = (
            [d.to_type_node_data() for d in input.type_nodes] if input.type_nodes else None
        )
        statement.language = input.language
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

    @project_mutation(PMT.MOVE_STATEMENT)
    def move_statement(self, input: StatementMoveInput) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.file_id = input.file_id.node_id
        statement.parent_id = input.parent_id.node_id if input.parent_id else None
        # TODO @Robustness: return a different order key if conflict on move/insert
        statement.order_key = input.order_key
        return statement

    @project_mutation(PMT.RENAME_STATEMENT)
    def rename_statement(self, input: StatementRenameInput) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.name = input.name
        return statement

    @project_mutation(PMT.UPDATE_STATEMENT_TYPE_NODE)
    def create_statement_type_node(
        self, input: StatementTypeNodeDataCreateInput
    ) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.type_nodes.append(input.to_type_node_data())
        return statement

    @project_mutation(PMT.UPDATE_STATEMENT_TYPE_NODE)
    def update_statement_type_node(
        self, input: StatementTypeNodeDataCreateInput
    ) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        node = first([n for n in statement.type_nodes if n == input.node_id.node_id])
        node = replace(node, **asdict(input.to_type_node_data()))
        # replace type node in array
        statement.type_nodes = [n for n in statement.type_nodes if n.id != input.node_id.node_id]
        statement.type_nodes.append(node)
        return statement

    @project_mutation(PMT.UPDATE_STATEMENT_TYPE_NODE)
    def delete_statement_type_node(
        self, input: StatementTypeNodeDataDeleteInput
    ) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.type_nodes = [n for n in statement.type_nodes if n.id != input.node_id.node_id]
        return statement


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
class StatementUpdateTypeInput(gql.NodeInput):
    btl: str


@gql.input
class StatementUpdateLanguageInput(gql.NodeInput):
    language: str


@gql.input
class StatementUpdateRecordsInput(gql.NodeInput):
    records: list[JSON]


@gql.input
class StatementCreateRecordInput(gql.NodeInput):
    record_id: UUID
    data: JSON
    order_key: str


@gql.input
class StatementUpdateRecordInput(gql.NodeInput):
    record_id: UUID
    data: JSON


@gql.input
class StatementDeleteRecordInput(gql.NodeInput):
    record_id: UUID


@gql.type
class SymbolMutation:
    # TODO @Cleanup: trivial statement field mutations should be much less code
    @project_mutation(PMT.UPDATE_STATEMENT_TEXT)
    def update_statement_text(self, input: StatementTextInput) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.text = input.text
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

    @project_mutation(PMT.UPDATE_STATEMENT_TYPE_NODE)
    def update_statement_type_node(
        self, input: StatementUpdateTypeInput
    ) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.btl = input.btl
        return statement

    @project_mutation(PMT.UPDATE_STATEMENT_LANGUAGE)
    def update_statement_language(
        self, input: StatementUpdateLanguageInput
    ) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.language = input.language
        return statement

    # TODO @Performance: separate statement record updates from regular statement updates
    @project_mutation(PMT.UPDATE_STATEMENT_RECORDS, atomic=True)
    def update_statement_records(
        self, input: StatementUpdateRecordsInput
    ) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.records.all().delete()
        db_records = [models.DatasetRecord(dataset=statement, data=data) for data in input.records]
        models.DatasetRecord.objects.bulk_create(db_records)
        return statement

    @project_mutation(PMT.UPDATE_STATEMENT_RECORDS, atomic=True)
    def create_statement_record(
        self, input: StatementCreateRecordInput
    ) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        record = models.DatasetRecord(dataset=statement, id=input.record_id, data=input.data)
        record.save()
        return statement

    @project_mutation(PMT.UPDATE_STATEMENT_RECORDS, atomic=True)
    def update_statement_record(
        self, input: StatementUpdateRecordInput
    ) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        record = statement.records.get_or_create(id=input.record_id, defaults={"data": input.data})
        record.data = input.data
        record.save()
        return statement
