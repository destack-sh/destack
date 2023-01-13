from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID

from bench import models
from bench.api.symbol import Statement
from bench.language.schema import parse_bsl


@gql.input
class SchemaUpdateContentBsl:
    statement_id: GlobalID
    bsl: str


@gql.type
class SchemaMutation:
    @gql.mutation
    def update_schema_content(self, input: SchemaUpdateContentBsl) -> Statement:
        statement: models.Statement = models.Statement.objects.get(id=input.statement_id.node_id)
        statement.element = parse_bsl(input.bsl)
        statement.save()
        return statement
