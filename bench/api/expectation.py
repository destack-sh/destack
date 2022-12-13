from strawberry import auto
from strawberry_django_plus import gql

from bench import models
from bench.api.symbol import Symbol, SymbolContent


@gql.django.type(models.Expectation)
class Expectation(SymbolContent):
    symbol: Symbol
    description: auto
