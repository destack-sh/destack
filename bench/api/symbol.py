from typing import TYPE_CHECKING, Annotated, Optional

from strawberry import lazy
from strawberry_django_plus import gql
from strawberry_django_plus.gql import auto
from strawberry_django_plus.relay import GlobalID

from bench import models

if TYPE_CHECKING:
    from bench.api.project import File, ProjectVersion
    from bench.api.schema import SchemaElement


@gql.django.type(models.Statement)
class Statement(gql.relay.Node):
    project_version: Annotated["ProjectVersion", lazy(".project")]
    file: Annotated["File", lazy(".project")]
    type: auto
    type_shortname: auto
    modifier: auto
    name: auto
    created_at: auto
    updated_at: auto
    deleted_at: auto
    commented: auto
    compiled: auto
    parent: Optional["Statement"]
    children: list["Statement"]
    index: auto
    symbol: Optional["Symbol"]
    source_symbol: Optional["Symbol"]
    reference: Optional["Statement"]
    parameters: list["Parameter"]
    arguments: list["Argument"]
    text: auto


@gql.django.type(models.Symbol)
class Symbol(gql.Node):
    project_version: Annotated["ProjectVersion", lazy(".project")]
    file: Annotated["File", lazy(".project")]
    type: auto
    type_shortname: auto
    created_at: auto
    updated_at: auto
    statement: "Statement"
    content: "SymbolContent"


@gql.django.interface(models.SymbolContent)
class SymbolContent(gql.Node):
    pass


@gql.django.type(models.Parameter)
class Parameter(gql.Node):
    symbol: Symbol
    name: auto
    created_at: auto
    updated_at: auto
    type: auto
    schema: Optional[Annotated["SchemaElement", lazy(".schema")]]


@gql.django.type(models.Argument)
class Argument(gql.Node):
    symbol: Symbol
    name: auto
    created_at: auto
    updated_at: auto
    reference: Optional[Statement]
    value: auto


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
class SymbolMutation:
    rename_statement: Statement = gql.django.update_mutation(StatementRenameInput)

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
        statement = models.Statement.objects.get(id=input.id.node_id)
        statement.restore()
        return StatementRestorePayload(statement=statement)
