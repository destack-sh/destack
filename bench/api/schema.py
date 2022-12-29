from typing import Optional

from strawberry_django_plus import gql

from bench import models
from bench.api.symbol import Statement, SymbolContent
from bench.language import schema

ValueType = gql.enum(schema.ValueType)


@gql.type
class SchemaElement:
    name: Optional[str]
    type: ValueType
    required: bool
    schema_id: Optional[str] = None
    elements: Optional[list["SchemaElement"]] = None


@gql.django.type(models.Schema)
class Schema(SymbolContent):
    definition: Statement
    description: str
    element: SchemaElement
