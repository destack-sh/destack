import json
import uuid
from enum import Enum
from functools import cache
from itertools import groupby
from typing import Any, Collection, Iterator, Optional, Type, TypeVar

from django.core.validators import RegexValidator
from django.db import models

from bench.utils.uuidt import NAME_REGEX, UUIDT


class UUIDModel(models.Model):
    id = models.UUIDField(primary_key=True, default=uuid.uuid4, editable=False)

    class Meta:
        abstract = True


class UUIDTModel(UUIDModel):
    id = models.UUIDField(primary_key=True, default=UUIDT, editable=False)

    class Meta:
        abstract = True


class Revisioned(models.Model):
    """Versioned model."""

    revision = models.IntegerField(default=0)

    class Meta:
        abstract = True


class ModuleNode:
    """A node in the module graph"""

    @property
    def parent_id(self) -> Optional[uuid.UUID]:
        raise NotImplementedError

    @property
    def parent(self) -> Optional["ModuleNode"]:
        raise NotImplementedError


class CrudModel(models.Model):
    """Versioned CRUD-tracked model."""

    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    deleted_at = models.DateTimeField(null=True, blank=True)
    created_by = models.ForeignKey(
        "User", on_delete=models.CASCADE, related_name="+", null=True, blank=True
    )
    last_edited_at = models.DateTimeField(auto_now=True)
    last_edited_by = models.ForeignKey(
        "User", on_delete=models.CASCADE, related_name="+", null=True, blank=True
    )
    last_changed_at = models.DateTimeField(auto_now=True)

    class Meta:
        abstract = True


NAME_VALIDATOR = RegexValidator(NAME_REGEX)

ModelT = TypeVar("ModelT", bound=models.Model)


def is_jsonable(value: Any) -> bool:
    """Check if value is JSON-serializable."""
    try:
        json.dumps(value)
        return True
    except TypeError:
        return False


T = TypeVar("T")


def create_models_bfs(layers: Iterator[Collection[ModuleNode]], exclude: set[uuid.UUID] = None):
    for node_batch in layers:
        if exclude and any(node.id in exclude for node in node_batch):
            node_batch = [node for node in node_batch if node.id not in exclude]
        for model_class, nodes_of_cls in groupby(node_batch, type):
            model_class.objects.bulk_create(nodes_of_cls)


@cache
def get_choices(enum_cls: Type[Enum]) -> list[tuple[str, str]]:
    """
    Get choices from enum class.
    """
    return [(member.value, member.name) for member in enum_cls]
