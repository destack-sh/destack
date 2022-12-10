from strawberry_django_plus import gql
from strawberry_django_plus.gql import auto

from bench import models
from bench.api import types


@gql.django.input(models.Symbol)
class SymbolCreateInput:
    project_version: auto
    name: auto
    type: auto
    file: auto
    parent: auto
    index: auto


@gql.django.partial(models.Symbol)
class SymbolRenameInput(gql.NodeInput):
    name: auto


@gql.type
class SymbolMutation:
    create_symbol: types.Symbol = gql.django.create_mutation(SymbolCreateInput)
    rename_symbol: types.Symbol = gql.django.update_mutation(SymbolRenameInput)
