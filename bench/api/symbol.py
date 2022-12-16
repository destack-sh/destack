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
    modifier: auto
    type_shortname: auto
    created_at: auto
    updated_at: auto
    commented: auto
    generated: auto
    parent: Optional["Statement"]
    children: list["Statement"]
    index: auto
    symbol: Optional["Symbol"]
    reference: Optional["Symbol"]
    arguments: list["SymbolArgument"]
    text: auto


@gql.django.type(models.Symbol)
class Symbol(gql.Node):
    project_version: Annotated["ProjectVersion", lazy(".project")]
    file: Annotated["File", lazy(".project")]
    name: auto
    type: auto
    type_shortname: auto
    type_name_declaration: auto
    created_at: auto
    updated_at: auto
    statement: "Statement"
    content: "SymbolContent"
    parameters: list["SymbolParameter"]
    arguments: list["SymbolArgument"]


@gql.django.interface(models.SymbolContent)
class SymbolContent(gql.Node):
    # TODO @Cleanup: add symbol in SymbolContent interface
    #  (should work since it's a 1:1 but strawberry claims it's undefined)
    # symbol: Symbol
    pass


@gql.django.type(models.SymbolParameter)
class SymbolParameter(gql.Node):
    symbol: Symbol
    name: auto
    created_at: auto
    updated_at: auto
    type: auto
    schema: Optional[Annotated["SchemaElement", lazy(".schema")]]


@gql.django.type(models.SymbolArgument)
class SymbolArgument(gql.Node):
    symbol: Symbol
    name: auto
    created_at: auto
    updated_at: auto
    type: auto
    reference: Optional[Symbol]
    value: auto


@gql.django.partial(models.Symbol)
class SymbolRenameInput(gql.NodeInput):
    name: auto


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


@gql.type
class SymbolMutation:
    rename_symbol: Symbol = gql.django.update_mutation(SymbolRenameInput)

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
