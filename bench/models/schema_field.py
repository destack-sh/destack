from __future__ import annotations

from typing import TYPE_CHECKING, Any, Optional

from django.db import models

from bench.utils.schema import SchemaElement, SchemaElementSerializer, ValueType


class SchemaElementField(models.JSONField):
    description = "A schema element"

    def from_db_value(self, value, expression, connection):
        if value is None:
            return value
        value = super().from_db_value(value, expression, connection)  # type: ignore
        return SchemaElementSerializer.from_json(value)

    def to_python(self, value):
        if isinstance(value, SchemaElement):
            return value
        return SchemaElementSerializer.from_json(value)

    def get_prep_value(self, value):
        if value is None:
            return value
        as_json = SchemaElementSerializer.to_json(value)
        return super().get_prep_value(as_json)

    def value_to_string(self, obj):
        value = self.value_from_object(obj)
        return self.get_prep_value(value)


class SchemaField(models.JSONField):
    description = "A schema element describing an object"
    """A schema is SchemaElement of type object."""

    def __init__(self, name: str, *args, **kwargs):
        self.name = name
        super().__init__(*args, **kwargs)

    def from_db_value(self, value, expression, connection):
        if value is None:
            return value
        value = super().from_db_value(value, expression, connection)  # type: ignore
        elements = [SchemaElementSerializer.from_json(e) for e in value]
        return SchemaElement(name=self.name, type=ValueType.OBJECT, elements=elements)

    def to_python(self, value):
        if value is None:
            return value
        elements = [SchemaElementSerializer.from_json(e) for e in value]
        return SchemaElement(name=self.name, type=ValueType.OBJECT, elements=elements)

    def get_prep_value(self, value):
        if value is None:
            return value
        as_json = [SchemaElementSerializer.to_json(e) for e in value.elements]
        return super().get_prep_value(as_json)

    def value_to_string(self, obj):
        value = self.value_from_object(obj)
        return self.get_prep_value(value)

    def deconstruct(self):
        name, path, args, kwargs = super().deconstruct()
        kwargs["name"] = self.name
        return name, path, args, kwargs


if TYPE_CHECKING:
    from bench.models import Schema, SymbolContent


class Schemad:
    @property
    def schema(self: SymbolContent) -> Optional["Schema"]:
        from bench.models import SymbolType

        child_statement = self.statement.child_of_symbol_type(SymbolType.SCHEMA)
        return child_statement.symbol.schema_ if child_statement else None

    @property
    def schema_(self) -> "Schema":
        if self.schema is None:
            raise ValueError(f"{self} doesn't have a schema")
        return self.schema

    def set_schema_element(self: Any, element: SchemaElement):
        if self.schema is None:
            # auto define new schema if none exists
            from bench.models import Schema

            schema = Schema(element=element)
            self.statement.file.define_symbol(
                name=self.symbol.name + "_schema", content=schema, parent=self.statement
            )
        else:
            self.schema.element = element
