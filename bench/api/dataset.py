from __future__ import annotations

from strawberry import auto
from strawberry_django_plus import gql

from bench import models
from bench.api.symbol import Symbol, SymbolContent


@gql.django.type(models.Dataset)
class Dataset(SymbolContent):
    symbol: Symbol
    length: auto
    records: list[DatasetRecord]


@gql.django.type(models.DatasetRecord)
class DatasetRecord(gql.Node):
    index: auto
    data: auto
