from typing import TYPE_CHECKING, Annotated

from strawberry import auto, lazy
from strawberry_django_plus import gql

from bench import models
from bench.api.misc import SchemaElement
from bench.api.symbol import Symbol, SymbolContent

if TYPE_CHECKING:
    from bench.api.compilation import Compilation


@gql.django.type(models.Task)
class Task(SymbolContent):
    symbol: Symbol
    description: auto
    input_schema: SchemaElement
    output_schema: SchemaElement
    compilations: list[Annotated["Compilation", lazy(".compilation")]]
