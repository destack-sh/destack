from strawberry_django_plus import gql
from strawberry_django_plus.gql import auto

from bench import models
from bench.api import types


@gql.django.input(models.SymbolDefinition)
class SymbolDefinitionInput:
    name: auto


@gql.django.partial(models.SymbolDefinition)
class SymbolDefinitionInputPartial(gql.NodeInput):
    name: auto


@gql.type
class SymbolDefinitionMutation:
    create_symbol_definition: types.SymbolDefinition = gql.django.create_mutation(
        SymbolDefinitionInput
    )
    update_symbol_definition: types.SymbolDefinition = gql.django.update_mutation(
        SymbolDefinitionInputPartial
    )
    delete_symbol_definition: types.SymbolDefinition = gql.django.delete_mutation(gql.NodeInput)
