import json
import uuid
from typing import Any, Type, TypeVar

from django.db import models
from django.db.models import QuerySet

from bench.utils.uuidt import UUIDT


class UUIDModel(models.Model):
    id = models.UUIDField(primary_key=True, default=uuid.uuid4, editable=False)

    class Meta:
        abstract = True


class UUIDTModel(UUIDModel):
    id = models.UUIDField(primary_key=True, default=UUIDT, editable=False)

    class Meta:
        abstract = True


ModelT = TypeVar("ModelT", bound=models.Model)


def proxies(query: QuerySet, proxy_type: Type[ModelT]) -> QuerySet[ModelT]:
    """
    Return proxies of query type.
    """
    if query.model not in proxy_type.mro():
        raise TypeError(f"cannot cast {query.model} to {proxy_type}")
    # TODO @Robustness: test and/or replace mechanism for "casting" model to proxy subtype
    #  Potential alternatives include using QuerySet.annotate.
    #  May also want to monkey-patch (and type) .proxies on standard QuerySet.
    query.model = proxy_type
    return query


def is_jsonable(value: Any) -> bool:
    """Check if value is JSON-serializable."""
    try:
        json.dumps(value)
        return True
    except TypeError:
        return False
