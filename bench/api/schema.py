from typing import Optional

from strawberry_django_plus import gql

from bench import models
from bench.api.symbol import Symbol, SymbolContent
from bench.utils import schema

ValueType = gql.enum(schema.ValueType)


@gql.type
class SchemaElement:
    name: str
    type: ValueType
    required: bool
    schema_id: Optional[str]
    # TODO @Cleanup: schema element choices should be unions
    choices: Optional[list[str]]
    elements: Optional[list["SchemaElement"]]


@gql.django.type(models.Schema)
class Schema(SymbolContent):
    symbol: Symbol
    description: str
    element: SchemaElement
