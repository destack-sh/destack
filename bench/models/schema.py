from django.db import models

from bench.models.schema_field import SchemaElementField
from bench.models.symbol import SymbolContent, SymbolContentManager


class SchemaManager(SymbolContentManager, models.Manager["Schema"]):
    pass


class Schema(SymbolContent):
    """
    A schema defines the structure of a JSON object, potentially in relation to other schemas.
    """

    description = models.TextField(null=True, blank=True)
    element = SchemaElementField()

    objects = SchemaManager()
