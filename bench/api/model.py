from __future__ import annotations

from typing import Optional

from strawberry import auto
from strawberry_django_plus import gql

from bench import models
from bench.api.symbol import Symbol, SymbolContent


@gql.django.type(models.Model)
class Model(SymbolContent):
    symbol: Symbol
    baseline: Optional[Model]
    provider: auto
