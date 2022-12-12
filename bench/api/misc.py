from __future__ import annotations

from typing import Optional

from strawberry_django_plus import gql

from bench.utils import schema

ValueType = gql.enum(schema.ValueType)


@gql.type
class SchemaElement:
    name: str
    type: ValueType
    required: bool
    # TODO @Cleanup: schema element choices should be unions
    choices: Optional[list[str]]
    elements: Optional[list[SchemaElement]]
