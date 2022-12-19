from typing import TYPE_CHECKING, Annotated, Optional

from strawberry import lazy
from strawberry_django_plus import gql
from strawberry_django_plus.gql import auto
from strawberry_django_plus.relay import GlobalID

from bench import models

if TYPE_CHECKING:
    from bench.api.project import File, ProjectVersion
    from bench.api.schema import SchemaElement

StatementType = gql.enum(models.StatementType)
SymbolType = gql.enum(models.SymbolType)


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
    parameters: list["Parameter"]
    arguments: list["Argument"]
    content: Optional["SymbolContent"]


@gql.django.interface(models.SymbolContent)
class SymbolContent(gql.Node):
    pass


@gql.django.type(models.Parameter)
class Parameter(gql.Node):
    statement: Statement
    name: auto
    created_at: auto
    updated_at: auto
    type: auto
    schema: Optional[Annotated["SchemaElement", lazy(".schema")]]


@gql.django.type(models.Argument)
class Argument(gql.Node):
    statement: Statement
    name: auto
    created_at: auto
    updated_at: auto
    reference: Optional[Statement]
    value: auto


@gql.input
class CreateStatementInput:
    fileId: GlobalID
    type: StatementType
    name: Optional[str]
    parent_id: Optional[GlobalID]
    index: Optional[int]


@gql.type
class CreateStatementPayload:
    statement: Statement


@gql.input
class MorphStatementInput:
    statementId: GlobalID
    type: StatementType
    symbol_type: Optional[SymbolType]


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

    @gql.mutation
    def create_statement(self, input: CreateStatementInput) -> CreateStatementPayload:
        file = models.File.objects.get(id=input.fileId.node_id)
        project_version = file.project_version
        parent = (
            models.Statement.objects.get(id=input.parent_id.node_id) if input.parent_id else None
        )
        statement = models.Statement.objects.create(
            project_version=project_version,
            file=file,
            type=input.type,
            name=input.name,
            parent=parent,
            index=input.index,
        )
        return CreateStatementPayload(statement=statement)

    @gql.mutation
    def morph_statement(self, input: MorphStatementInput) -> MorphStatementPayload:
        statement = models.Statement.objects.get(id=input.statementId.node_id)
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
        statement = models.Statement._base_manager.get(id=input.id.node_id)
        statement.restore()
        return StatementRestorePayload(statement=statement)
