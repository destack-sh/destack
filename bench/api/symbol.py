from typing import TYPE_CHECKING, Annotated, Optional, Union

from strawberry import lazy
from strawberry.scalars import JSON
from strawberry_django_plus import gql
from strawberry_django_plus.gql import auto
from strawberry_django_plus.relay import GlobalID

import bench.language.types
from bench import language, models

if TYPE_CHECKING:
    from bench.api.project import File, ProjectVersion

StatementType = gql.enum(models.StatementType)
SymbolType = gql.enum(models.SymbolType)


@gql.type
class Type:
    description: str
    element: "TypeElement"
    btl: str


ValueType = gql.enum(bench.language.types.TypeTag)


@gql.type
class TypeElement:
    name: Optional[str]
    type: ValueType
    required: bool = True
    schema_id: Optional[str] = None
    elements: Optional[list["TypeElement"]] = None


@gql.type
class Capability:
    description: str


@gql.type
class Task:
    description: str


@gql.type
class Expectation:
    description: str


@gql.type
class Code:
    builtin_id: str
    code: str


@gql.type
class Model:
    provider: str
    external_name: str


@gql.type
class Dataset:
    records: list["DatasetRecord"]


@gql.type
class DatasetRecord(gql.Node):
    data: JSON


@gql.type
class Value:
    value: JSON


@gql.type
class Requirement:
    project_version: Annotated["ProjectVersion", lazy(".project")]


@gql.type
class Compilation:
    mappings: list["SourceMapping"]


@gql.type
class SourceMapping:
    source: Annotated["Statement", lazy(".symbol")]
    source_path: JSON
    source_revision: int
    target: Annotated["Statement", lazy(".symbol")]
    target_path: JSON
    target_revision: int


SymbolContent = Union[Type, Capability, Task, Expectation, Code, Dataset, Model, Value, Requirement]


@gql.django.type(models.Statement)
class Statement(gql.relay.Node):
    project_version: Annotated["ProjectVersion", lazy(".project")]
    file: Annotated["File", lazy(".project")]
    type: StatementType
    modifier: auto
    name: auto
    created_at: auto
    updated_at: auto
    deleted_at: auto
    commented: auto
    compiled: auto
    parent: Optional["Statement"]
    children: list["Statement"]
    descendants: list["Statement"]
    index: auto
    symbol_type: Optional[SymbolType]
    text: auto
    source_definition: Optional["Statement"]
    reference: Optional["Statement"]
    referenced_by: list["Statement"]
    content: Optional[SymbolContent]


@gql.input
class StatementCreateInput:
    file_id: GlobalID
    type: StatementType
    name: Optional[str] = None
    parent_id: Optional[GlobalID] = None
    index: Optional[int] = None


@gql.type
class StatementCreatePayload:
    statement: Statement


@gql.input
class StatementMorphInput:
    statement_id: GlobalID
    type: StatementType
    symbol_type: Optional[SymbolType] = None


@gql.django.partial(models.Statement)
class StatementSetModifierInput(gql.NodeInput):
    modifier: auto


@gql.input
class StatementSetReferenceInput:
    statement_id: GlobalID
    reference_id: Optional[GlobalID] = None


@gql.type
class StatementSetReferencePayload:
    statement: Statement


@gql.type
class MorphStatementPayload:
    statement: Statement


@gql.django.partial(models.Statement)
class StatementRenameInput(gql.NodeInput):
    name: auto


@gql.django.partial(models.Statement)
class StatementTextInput(gql.NodeInput):
    text: auto


@gql.input
class StatementMoveInput:
    id: GlobalID
    file_id: GlobalID
    parent_id: Optional[GlobalID] = None
    index: Optional[int] = None


@gql.type
class StatementMovePayload:
    statement: Statement
    old_file: Annotated["File", lazy(".project")]
    new_file: Annotated["File", lazy(".project")]


@gql.input
class StatementSoftDeleteInput(gql.NodeInput):
    pass


@gql.type
class StatementSoftDeletePayload:
    statement: Statement


@gql.input
class StatementRestoreInput(gql.NodeInput):
    pass


@gql.type
class StatementRestorePayload:
    statement: Statement


@gql.input
class StatementCommentedInput(gql.NodeInput):
    commented: bool


@gql.type
class StatementCommentedPayload:
    statement: Statement


@gql.type
class StatementMutation:
    rename_statement: Statement = gql.django.update_mutation(StatementRenameInput)
    set_modifier_statement: Statement = gql.django.update_mutation(StatementSetModifierInput)

    @gql.mutation
    def create_statement(self, input: StatementCreateInput) -> StatementCreatePayload:
        file = models.File.objects.get(id=input.file_id.node_id)
        project_version = file.project_version
        parent = (
            models.Statement.objects.get(id=input.parent_id.node_id) if input.parent_id else None
        )
        statement = models.Statement.objects.create_statement(
            project_version=project_version,
            file=file,
            type=input.type,
            name=input.name,
            parent=parent,
            index=input.index,
        )
        return StatementCreatePayload(statement=statement)

    @gql.mutation
    def set_reference_statement(
        self, input: StatementSetReferenceInput
    ) -> StatementSetReferencePayload:
        statement = models.Statement.objects.get(id=input.statement_id.node_id)
        reference = (
            models.Statement.objects.get(id=input.reference_id.node_id)
            if input.reference_id
            else None
        )
        statement.reference = reference
        statement.save()
        return StatementSetReferencePayload(statement=statement)

    @gql.mutation
    def morph_statement(self, input: StatementMorphInput) -> MorphStatementPayload:
        statement = models.Statement.objects.get(id=input.statement_id.node_id)
        statement.morph_to(input.type, input.symbol_type)
        return MorphStatementPayload(statement=statement)

    @gql.mutation
    def comment_statement(self, input: StatementCommentedInput) -> StatementCommentedPayload:
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.set_commented(input.commented)
        return StatementCommentedPayload(statement=statement)

    @gql.mutation
    def move_statement(self, input: StatementMoveInput) -> StatementMovePayload:
        statement = models.Statement.objects.get(id=input.id.node_id)
        file = models.File.objects.get(id=input.file_id.node_id)
        parent = (
            models.Statement.objects.get(id=input.parent_id.node_id) if input.parent_id else None
        )
        old_file = statement.file
        statement.move_to(file, parent, input.index)
        return StatementMovePayload(statement=statement, old_file=old_file, new_file=file)

    @gql.mutation
    def soft_delete_statement(self, input: StatementSoftDeleteInput) -> StatementSoftDeletePayload:
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.soft_delete()
        return StatementSoftDeletePayload(statement=statement)

    @gql.mutation
    def restore_statement(self, input: StatementRestoreInput) -> StatementRestorePayload:
        # use base manager since default manager excludes soft deleted statements
        statement = models.Statement._base_manager.get(id=input.id.node_id)
        statement.restore()
        return StatementRestorePayload(statement=statement)
