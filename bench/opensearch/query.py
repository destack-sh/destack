from abc import ABC, abstractmethod
from typing import Any, Union

from bench.bench.query import (
    ComparisonQuery,
    CompoundQuery,
    Sort,
    QueryOp,
    KnnQuery,
    Query,
    ExistenceQuery,
)


class Compiler(ABC):
    @abstractmethod
    def compile(self, obj: Any) -> dict[str, Any]:
        raise NotImplementedError


_COMPILERS: dict[type, Compiler] = {}


def compiler(dsl_type: type):
    def wrapper(cls: type):
        if dsl_type in _COMPILERS:
            raise RuntimeError(
                f"compiler for {dsl_type} already registered: {_COMPILERS[dsl_type]}"
            )
        _COMPILERS[dsl_type] = cls()
        return cls

    return wrapper


@compiler(CompoundQuery)
class CompoundQueryCompiler(Compiler):
    MAPPING = {
        QueryOp.NOT: "must_not",
        QueryOp.AND: "must",
        QueryOp.OR: "should",
    }

    def compile(self, query: CompoundQuery) -> dict[str, Any]:
        return {"bool": {self.MAPPING[query.op]: compile_to_os(query.queries)}}


@compiler(ComparisonQuery)
class ComparisonQueryCompiler(Compiler):
    def compile(self, query: ComparisonQuery) -> dict[str, Any]:
        if query.op == QueryOp.EQUALS:
            if isinstance(query.value, list):
                return {"terms": {query.key: query.value}}
            else:
                return {"term": {query.key: query.value}}
        elif query.op == QueryOp.NOT_EQUALS:
            return {"bool": {"must_not": {"term": {query.key: query.value}}}}
        elif query.op == QueryOp.GREATER_THAN:
            return {"range": {query.key: {"gt": query.value}}}
        elif query.op == QueryOp.GREATER_THAN_OR_EQUALS:
            return {"range": {query.key: {"gte": query.value}}}
        elif query.op == QueryOp.LESS_THAN:
            return {"range": {query.key: {"lt": query.value}}}
        elif query.op == QueryOp.LESS_THAN_OR_EQUALS:
            return {"range": {query.key: {"lte": query.value}}}
        elif query.op == QueryOp.MATCHES:
            return {"match": {query.key: query.value}}
        elif query.op == QueryOp.STARTS_WITH:
            return {"prefix": {query.key: query.value}}
        else:
            raise RuntimeError(f"unexpected query: {query}")


@compiler(ExistenceQuery)
class ExistenceQueryCompiler(Compiler):
    def compile(self, query: ExistenceQuery) -> dict[str, Any]:
        if query.op == QueryOp.EXISTS:
            return {"exists": {"field": query.key}}
        elif query.op == QueryOp.DOES_NOT_EXIST:
            return {"bool": {"must_not": {"exists": {"field": query.key}}}}
        else:
            raise RuntimeError(f"unexpected query: {query}")


@compiler(KnnQuery)
class KnnQueryCompiler(Compiler):
    def compile(self, query: KnnQuery) -> dict[str, Any]:
        if query.approximate:
            return {"knn": {query.key: {"vector": query.value}}}
        else:
            raise NotImplementedError(f"exact knn not implemented: {query}")


@compiler(Sort)
class SortCompiler(Compiler):
    def compile(self, sort: Sort) -> dict[str, Any]:
        props = {"order": sort.order.value}
        if sort.mode:
            props["mode"] = sort.mode.value
        return {sort.key: props}


DslObj = Union[Query, Sort]


def compile_to_os(obj: DslObj | list[DslObj]) -> dict[str, Any] | list[dict[str, Any]]:
    if isinstance(obj, list):
        return [compile_to_os(o) for o in obj]
    else:
        compiler = _COMPILERS[type(obj)]
        return compiler.compile(obj)


def compact_os_queries(queries: list[dict[str, Any]]) -> dict[str, Any]:
    """
    Combine a list of queries into a single query.
    Unroll and un-nest as much as possible
     - two terms queries with different keys into one terms
    """
    if len(queries) == 1:
        return queries[0]
    else:
        return {"bool": {"must": queries}}  # nocheckin
