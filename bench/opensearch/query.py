from abc import ABC, abstractmethod
from typing import Any

from bench.bench.query import ComparisonQuery, CompoundQuery


class Compiler(ABC):
    @abstractmethod
    def compile(self, obj: Any) -> dict[str, Any]:
        raise NotImplementedError


COMPILERS: dict[type, Compiler] = {}


def compiler(dsl_type: type):
    def wrapper(cls: type):
        if not issubclass(cls, Compiler):
            raise TypeError(f"Class {cls.__name__} is not a subclass of Compiler")
        COMPILERS[dsl_type] = cls()
        return cls

    return wrapper


@compiler(CompoundQuery)
class CompoundQueryCompiler(Compiler):
    def compile(self, query: CompoundQuery) -> dict[str, Any]:
        return {f"{query.op.value}": [compile(q) for q in query.queries]}


@compiler(ComparisonQuery)
class ComparisonQueryCompiler(Compiler):
    def compile(self, query: ComparisonQuery) -> dict[str, Any]:
        raise NotImplementedError


def compile_to_os(obj: Any) -> dict[str, Any]:
    compiler = COMPILERS[type(obj)]
    return compiler.compile(obj)
