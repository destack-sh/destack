from typing import TYPE_CHECKING, Annotated, Optional
from uuid import UUID

from django import db
from django.core.exceptions import ValidationError
from django.db.models import F, Q, Value
from django.db.models.functions import Concat
from strawberry import lazy
from strawberry.scalars import JSON
from strawberry_django_plus import gql
from strawberry_django_plus.gql import auto
from strawberry_django_plus.relay import GlobalID
from strawberry_django_plus.types import OperationInfo

from bench import language, models
from bench.api.sync import PMT, project_mutation

if TYPE_CHECKING:
    from bench.api.project import File, ProjectVersion

StatementType = gql.enum(models.StatementType)
StatementModifier = gql.enum(language.StatementModifier)
SymbolType = gql.enum(models.SymbolType)


@gql.type
class Type:
    description: Optional[str]
    btl: str


@gql.django.type(models.SourceMapping)
class SourceMapping:
    statement_id: GlobalID
    source_id: GlobalID
    source_revision: int
    target_id: Optional[GlobalID]
    target_revision: Optional[int]


@gql.django.type(models.DatasetRecord)
class DatasetRecord(gql.Node):
    id: GlobalID
    created_at: auto
    updated_at: auto
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
    statement: auto
    created_at: auto
    updated_at: auto
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
    type_nodes: list[SimpleTypeNode]
    lang: auto
    code: auto
    description: auto
    reference_project_version: Optional[Annotated["ProjectVersion", lazy(".project")]]
    value: auto
    records: gql.relay.Connection[DatasetRecord] = gql.django.connection()

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

    @project_mutation(PMT.RENAME_STATEMENT)
    def rename_statement(self, input: StatementRenameInput) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.name = input.name
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
class StatementUpdateLanguageInput(gql.NodeInput):
    language: str


@gql.input
class RecordCreateInput(gql.NodeInput):
    statement_id: GlobalID
    data: JSON
    order_key: str


@gql.input
class RecordUpdateInput(gql.NodeInput):
    statement_id: GlobalID
    data: JSON


@gql.input
class RecordMoveInput(gql.NodeInput):
    statement_id: GlobalID
    order_key: str


@gql.input
class RecordDeleteInput(gql.NodeInput):
    statement_id: GlobalID


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
    statement_id: GlobalID
    order_key: str


@gql.input
class TypeNodeDeleteInput(gql.NodeInput):
    statement_id: GlobalID


# TODO @Cleanup: trivial statement field mutations should be much less code
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

    @project_mutation(PMT.UPDATE_STATEMENT_RECORDS, atomic=True)
    def create_statement_record(self, input: RecordCreateInput) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.statement_id.node_id)
        record = models.DatasetRecord(
            statement=statement,
            id=UUID(input.id.node_id),
            data=input.data,
            order_key=input.order_key,
        )
        record.save()
        return statement

    # TODO @Cleanup @Performance: DatasetRecord and TypeNode want to be their own objects in updates
    #  But we encapsulate them in Statement for unified save & revision updates (and auth checks).
    #  Maybe they they should have their own revisions (additionally?), though that may complicate syncing.
    #  Right now they're nested inside Statement so we need to run updates as atomic (inefficient).
    #  :SubSymbolRevisions

    @project_mutation(PMT.UPDATE_STATEMENT_RECORDS, atomic=True)
    def update_statement_record(self, input: RecordUpdateInput) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.statement_id.node_id)
        record = statement.records.get(id=UUID(input.id.node_id))
        record.data = input.data
        record.save()
        return statement

    @project_mutation(PMT.UPDATE_STATEMENT_RECORDS, atomic=True)
    def move_statement_record(self, input: RecordMoveInput) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.statement_id.node_id)
        record = statement.records.get(id=input.id)
        record.order_key = input.order_key
        record.save()
        return statement

    @project_mutation(PMT.UPDATE_STATEMENT_RECORDS, atomic=True)
    def delete_statement_record(self, input: RecordDeleteInput) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.statement_id.node_id)
        _ = statement.records.filter(id=UUID(input.id.node_id)).delete()
        return statement

    @project_mutation(PMT.UPDATE_STATEMENT_TYPE_NODE, atomic=True)
    def create_statement_type_node(self, input: TypeNodeCreateInput) -> Statement | OperationInfo:
        statement = models.Statement.objects.get(id=input.statement_id.node_id)
        type_node = input.to_model()
        type_node.statement = statement
        type_node.full_clean(validate_unique=False, validate_constraints=False)
        type_node.save()
        return statement

    @project_mutation(PMT.UPDATE_STATEMENT_TYPE_NODE, atomic=True)
    def update_statement_type_node(self, input: TypeNodeUpdateInput) -> Statement | OperationInfo:
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
        type_node.save()
        return type_node.statement

    @project_mutation(PMT.UPDATE_STATEMENT_TYPE_NODE, atomic=True)
    def move_statement_type_node(self, input: TypeNodeMoveInput) -> Statement | OperationInfo:
        type_node = models.SimpleTypeNode.objects.get(id=input.id.node_id)
        type_node.order_key = input.order_key
        type_node.save()
        return type_node.statement

    @project_mutation(PMT.UPDATE_STATEMENT_TYPE_NODE, atomic=True)
    def delete_statement_type_node(self, input: TypeNodeDeleteInput) -> Statement | OperationInfo:
        type_node = models.SimpleTypeNode.objects.get(id=input.id.node_id)
        type_node.delete()
        return type_node.statement
