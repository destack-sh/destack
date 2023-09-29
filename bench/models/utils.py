import json
import uuid
from enum import Enum
from functools import cache
from itertools import groupby
from typing import Any, Collection, Iterator, Optional, Type, TypeVar

from django.core.validators import RegexValidator
from django.db import models

from bench.language.validation import NAME_REGEX
from bench.utils.uuidt import UUIDT


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


class ModuleNode(models.Model):
    """A node in the module tree. See language/core."""

    id = models.UUIDField(primary_key=True, editable=False)  # must be set manually
    ck = models.UUIDField(default=uuid.uuid4, editable=False)

    @property
    def parent_id(self) -> Optional[uuid.UUID]:
        raise NotImplementedError(f"{self} does not implement parent_id")

    @property
    def parent(self) -> Optional["ModuleNode"]:
        raise NotImplementedError(f"{self} does not implement parent")

    class Meta:
        abstract = True


class DetachedModuleNode(ModuleNode):
    """A cross-Bench node that doesn't belong to a single module (version)."""

    initial_project_version = models.ForeignKey(
        "ProjectVersion", null=True, blank=True, on_delete=models.CASCADE
    )

    class Meta:
        abstract = True


class CrudModel(models.Model):
    """Versioned CRUD-tracked model."""

    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    deleted_at = models.DateTimeField(null=True, blank=True)
    created_by = models.ForeignKey(
        "User", on_delete=models.SET_NULL, related_name="+", null=True, blank=True
    )
    last_edited_at = models.DateTimeField(auto_now=True)
    last_edited_by = models.ForeignKey(
        "User", on_delete=models.SET_NULL, related_name="+", null=True, blank=True
    )

    class Meta:
        abstract = True


class CrudNode(CrudModel, ModuleNode, Revisioned):
    last_edited_in = models.ForeignKey(
        "Run", on_delete=models.SET_NULL, null=True, blank=True, related_name="+"
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
