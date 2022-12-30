from typing import Optional

from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID

from bench import models
from bench.api.symbol import Statement, SymbolContent
from bench.language import schema
from bench.language.schema import parse_bsl

ValueType = gql.enum(schema.ValueType)


@gql.type
class SchemaElement:
    name: Optional[str]
    type: ValueType
    required: bool
    schema_id: Optional[str] = None
    elements: Optional[list["SchemaElement"]] = None


@gql.django.type(models.Schema)
class Schema(SymbolContent):
    definition: Statement
    description: str
    element: SchemaElement
    bsl: str


@gql.input
class SchemaUpdateContentBsl:
    statement_id: GlobalID
    bsl: str


@gql.type
class SchemaMutation:
    @gql.mutation
    def update_schema_content(self, input: SchemaUpdateContentBsl) -> Statement:
        statement: models.Statement = models.Statement.objects.get(id=input.statement_id.node_id)
        statement.schema_.element = parse_bsl(input.bsl)
        statement.schema_.save()
        return statement
