from __future__ import annotations

import typing
from dataclasses import dataclass
from functools import cached_property, partial

from bench.backend.provider import ModelHandle
from bench.language.types import Code, Dataset, Model, SymbolContent

CodeCallable = typing.Callable[..., typing.Coroutine]
Value = typing.Any


@dataclass(repr=False)
class LoadedDataset(Dataset):
    pass  # no additional attributes for now


@dataclass(repr=False)
class LoadedModel(Model):
    handle: ModelHandle


@dataclass(repr=False)
class LoadedCode(Code):
    code_callable: CodeCallable
    loaded_arguments: dict[str, SymbolContent | typing.Any]

    @cached_property
    def callable_name(self) -> str:
        # get callable name (if partial get underlying func name)
        if isinstance(self.code_callable, partial):
            return self.code_callable.func.__name__
        else:
            return self.code_callable.__name__


LoadedSymbol = typing.Union[LoadedDataset, LoadedModel, LoadedCode]
