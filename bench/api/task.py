from typing import TYPE_CHECKING, Annotated

from strawberry import auto, lazy
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID

from bench import models
from bench.api.symbol import Symbol, SymbolContent

if TYPE_CHECKING:
    from bench.api.compilation import Compilation


@gql.django.type(models.Task)
class Task(SymbolContent):
    symbol: Symbol
    description: auto
    compilations: list[Annotated["Compilation", lazy(".compilation")]]


@gql.input
class TaskUpdateContentDescription:
    symbol_id: GlobalID
    description: str


@gql.type
class TaskMutation:
    @gql.mutation
    def update_task_content(self, input: TaskUpdateContentDescription) -> Symbol:
        symbol: models.Symbol = models.Symbol.objects.get(id=input.symbol_id.node_id)
        symbol.task_.description = input.description
        symbol.task_.save()
        return symbol
