from strawberry import auto
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID

from bench import models
from bench.api.symbol import Symbol, SymbolContent


@gql.django.type(models.Expectation)
class Expectation(SymbolContent):
    symbol: Symbol
    description: auto


@gql.input
class ExpectationUpdateContentDescription:
    symbol_id: GlobalID
    description: str


@gql.type
class ExpectationMutation:
    @gql.mutation
    def update_expectation_content(self, input: ExpectationUpdateContentDescription) -> Symbol:
        symbol: models.Symbol = models.Symbol.objects.get(id=input.symbol_id.node_id)
        symbol.expectation_.description = input.description
        symbol.expectation_.save()
        return symbol
