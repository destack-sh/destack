from django.db import models

from bench.models.schema_field import SchemaElementField
from bench.models.symbol import SymbolContent, SymbolContentManager


class SchemaManager(SymbolContentManager, models.Manager["Schema"]):
    pass


class Schema(SymbolContent):
    """
    A schema defines the structure of a JSON object, potentially in relation to other schemas.
    """

    description = models.TextField()
    element = SchemaElementField()

    def __str__(self):
        return f"({self.element})"

    objects = SchemaManager()
