from __future__ import annotations

from abc import ABC
from typing import Any, Optional, Type, Union, cast

from bench.utils.record import Record, RecordBatch
from bench.utils.registry import Registry, RegistryError, get_qualified_name
from bench.utils.spec import (
    ArtifactSetSpec,
    ArtifactSetType,
    ConfigSpec,
    ConfigType,
    RecordSpec,
    RecordType,
    convert_to_config_spec,
)


class FunctionBase(ABC):
    """
    A generic pure function that operates either on records/batches or on artifacts.
    """

    # could auto-infer this from function constructor in some cases via inspect
    config_spec: Union[ConfigType, ConfigSpec]


class ArtifactFunction(FunctionBase, ABC):
    """
    A pure function that operates on artifacts.
    """

    input_spec: Union[None, ArtifactSetType, ArtifactSetSpec]
    output_spec: Union[None, ArtifactSetType, ArtifactSetSpec]


class RecordFunction(FunctionBase, ABC):
    """
    A pure function that operates on records/batches.
    """

    input_spec: Union[None, RecordType, RecordSpec]
    output_spec: Union[None, RecordType, RecordSpec]


class Transform(RecordFunction, ABC):
    """
    A transformation function mapping input records to output records
    """

    def __call__(self, record: Record) -> Record:
        raise NotImplementedError


class BatchTransform(RecordFunction, ABC):
    """
    A transformation function mapping input records to output records in batches
    """

    def __call__(self, records: RecordBatch) -> RecordBatch:
        raise NotImplementedError


class Predicate(RecordFunction, ABC):
    output_spec = None

    def __call__(self, record: Record) -> bool:
        raise NotImplementedError


def map_to_function_cls(
    func: Any, impl: Optional[Type[FunctionBase]]
) -> Type[FunctionBase]:
    """
    Map the given class or function-based Function into a full Function type,
    type-checking and filling in spec information as needed.

    @param func: A class function (FunctionBase) or basic function (any Python function)
    @param impl: The intended type of the Function, required for Python functions
    @return: The full Function type
    """
    if isinstance(func, type):  # func is a class
        if impl is not None and not issubclass(impl, FunctionBase):
            raise RegistryError(
                f"registered function class {get_qualified_name(func)}"
                f" is not a subclass of FunctionBase"
            )
        if impl is not None and not issubclass(func, impl):
            raise RegistryError(
                f"given function class {get_qualified_name(func)} does not match"
                f" given type {get_qualified_name(type)}, remove or change impl=<...>"
            )

        func = cast(Type[FunctionBase], func)
        declared_config_spec = convert_to_config_spec(func.config_spec)
        # inferred_config_spec = infer_config_spec(func.__init__)

        # overwrite config spec with cleaned and merged config
        func.config_spec = declared_config_spec

        return func
    else:  # func is a Python function and must be mapped to FunctionBase subtype
        if type is None:
            raise RegistryError(
                f"non-class function {get_qualified_name(func)} must specify its type"
                f" via register, like with register(..., impl=Transform)"
            )

        # inferred_config_spec = infer_config_spec(func.__init__)

        return func


functions: Registry[Type[FunctionBase]] = Registry(
    ("functions",), mapper=map_to_function_cls
)

# TODO @Feature: figure out better registration mechanism for registered objects
import bench.function.transform.text  # noqa
