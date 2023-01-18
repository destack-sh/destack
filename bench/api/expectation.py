from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID

from bench import models
from bench.api.common import async_safe_mutation
from bench.api.symbol import Statement


@gql.input
class ExpectationUpdateContentDescription:
    statement_id: GlobalID
    description: str


@gql.type
class ExpectationMutation:
    @async_safe_mutation
    def update_expectation_content(self, input: ExpectationUpdateContentDescription) -> Statement:
        statement: models.Statement = models.Statement.objects.get(id=input.statement_id.node_id)
        statement.description = input.description
        statement.save()
        return statement
