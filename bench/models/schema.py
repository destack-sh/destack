from django.db import models

from bench.models.schema_field import SchemaElementField
from bench.models.symbol import SymbolContent, SymbolContentManager


class SchemaManager(SymbolContentManager, models.Manager["Schema"]):
    pass


def _make_default_schema_element():
    from bench.api.schema import SchemaElement
    from bench.utils.schema import ValueType

    return SchemaElement(name="", type=ValueType.OBJECT, required=True, elements=[])


class Schema(SymbolContent):
    """
    A schema defines the structure of a JSON object, potentially in relation to other schemas.
    """

    description = models.TextField(default="")
    element = SchemaElementField(default=_make_default_schema_element)

    def __str__(self):
        return f"({self.element})"

    objects = SchemaManager()
