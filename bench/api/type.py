from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID

from bench import models
from bench.api.common import async_safe_mutation
from bench.api.symbol import Statement


@gql.input
class TypeUpdateContentBtl:
    statement_id: GlobalID
    btl: str


@gql.type
class TypeMutation:
    @async_safe_mutation
    def update_type_content(self, input: TypeUpdateContentBtl) -> Statement:
        statement: models.Statement = models.Statement.objects.get(id=input.statement_id.node_id)
        statement.btl = input.btl
        statement.save()
        return statement
