from typing import Optional

from strawberry_django_plus import gql

from bench import models
from bench.api.symbol import Statement, SymbolContent
from bench.language import schema

ValueType = gql.enum(schema.ValueType)


@gql.type
class SchemaElement:
    name: str
    type: ValueType
    required: bool
    schema_id: Optional[str] = None
    # TODO @Cleanup: schema element choices should be unions
    choices: Optional[list[str]] = None
    elements: Optional[list["SchemaElement"]] = None


@gql.django.type(models.Schema)
class Schema(SymbolContent):
    definition: Statement
    description: str
    element: SchemaElement
