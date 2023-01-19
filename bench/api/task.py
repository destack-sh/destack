from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID

from bench import models
from bench.api.symbol import Statement
from bench.api.util import async_safe_mutation


@gql.input
class TaskUpdateContentDescription:
    statement_id: GlobalID
    description: str


@gql.type
class TaskMutation:
    @async_safe_mutation
    def update_task_content(self, input: TaskUpdateContentDescription) -> Statement:
        statement: models.Statement = models.Statement.objects.get(id=input.statement_id.node_id)
        statement.description = input.description
        statement.save()
        return statement
