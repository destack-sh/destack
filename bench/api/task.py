from typing import TYPE_CHECKING, Annotated

from strawberry import auto, lazy
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID

from bench import models
from bench.api.symbol import Statement, SymbolContent

if TYPE_CHECKING:
    from bench.api.compilation import Compilation


@gql.django.type(models.Task)
class Task(SymbolContent):
    definition: Statement
    description: auto
    compilations: list[Annotated["Compilation", lazy(".compilation")]]


@gql.input
class TaskUpdateContentDescription:
    statement_id: GlobalID
    description: str


@gql.type
class TaskMutation:
    @gql.mutation
    def update_task_content(self, input: TaskUpdateContentDescription) -> Statement:
        statement: models.Statement = models.Statement.objects.get(id=input.statement_id.node_id)
        statement.task_.description = input.description
        statement.task_.save()
        return statement
