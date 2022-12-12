from typing import TYPE_CHECKING, Annotated, Optional

from strawberry import lazy
from strawberry_django_plus import gql

from bench import models
from bench.api.misc import SchemaElement
from bench.api.symbol import Symbol, SymbolContent

if TYPE_CHECKING:
    from bench.api.compilation import Compilation
    from bench.api.expectation import Expectation


@gql.django.type(models.Task)
class Task(SymbolContent):
    symbol: Symbol
    input_schema: SchemaElement
    output_schema: SchemaElement
    expectations: list[Annotated["Expectation", lazy(".expectation")]]
    template_implementation: Optional[Symbol]
    compilations: list[Annotated["Compilation", lazy(".compilation")]]
