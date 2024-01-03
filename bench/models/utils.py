import uuid
from enum import Enum
from functools import cache
from typing import Type

from django.db import models


class UUIDModel(models.Model):
    id = models.UUIDField(primary_key=True, default=uuid.uuid4, editable=False)

    class Meta:
        abstract = True


@cache
def get_choices(enum_cls: Type[Enum]) -> list[tuple[str, str]]:
    """
    Get choices from enum class.
    """
    return [(member.value, member.name) for member in enum_cls]
