from __future__ import annotations

import enum
from abc import ABC
from dataclasses import dataclass
from typing import (
    Any,
    Callable,
    Dict,
    Literal,
    Mapping,
    Optional,
    OrderedDict,
    Type,
    Union,
    cast,
)

from bench.utils.record import Record, RecordBatch, RecordList
from bench.utils.registry import Registry, RegistryError, get_qualified_name
from bench.utils.spec import (
    ArtifactSetSpec,
    ConfigSpec,
    FunctionSpec,
    RecordSpec,
    convert_to_config_spec,
    convert_to_record_spec,
    infer_config_spec,
    reduce_to_record_type_spec,
)


class Function(ABC):
    """
    A generic pure function that operates either on records/batches or on artifacts.
    """

    config_spec: ConfigSpec


class ArtifactFunction(Function, ABC):
    """
    A pure function that operates on artifacts.
    """

    input_spec: Mapping[str, ArtifactSetSpec]
    output_spec: OrderedDict[str, ArtifactSetSpec]


class RecordFunction(Function, ABC):
    """
    A pure function that operates on records/batches.
    """

    input_spec: Mapping[str, RecordSpec]
    output_spec: OrderedDict[str, RecordSpec]


class Metric(RecordFunction):
    """
    A metric computed over an arbitrary set of input record batches.
    """

    def compute(self, **kwargs: RecordBatch) -> Record:
        raise NotImplementedError


class RecordTransform(RecordFunction, ABC):
    """
    A transformation function mapping input records to output records
    """

    def transform(self, record: Record) -> Union[Record, RecordBatch]:
        raise NotImplementedError

    def transform_batch(self, records: RecordBatch) -> RecordBatch:
        raise NotImplementedError


class SingleRecordTransform(RecordTransform, ABC):
    def transform_batch(self, records: RecordBatch) -> RecordBatch:
        outputs = []
        for record in records:
            output = self.transform(record)
            if isinstance(output, RecordBatch):
                outputs.extend(output)
            else:
                outputs.append(output)
        return RecordList(outputs)


class BatchRecordTransform(RecordTransform, ABC):
    def transform(self, record: Record) -> Union[Record, RecordBatch]:
        input_batch = RecordList([record])
        output_batch = self.transform_batch(input_batch)
        if len(output_batch) == 1:
            return output_batch[0]
        else:
            return output_batch


class Test(Function):
    def __init__(self, result_key: str):
        self.result_key = result_key

    def passed(self, output: Record) -> bool:
        output = cast(dict, output)  # assume output is dict
        return cast(bool, output[self.result_key])


def map_to_function_cls(func: Any, impl: Optional[Type[Function]]) -> Type[Function]:
    """
    Map the given class or function-based Function into a full Function type,
    type-checking and filling in spec information as needed.

    @param func: A class function (FunctionBase) or basic function (any Python function)
    @param impl: The intended type of the Function, required for Python functions
    @return: The full Function type
    """
    if isinstance(func, type):  # func is a class
        if impl is not None and not issubclass(impl, Function):
            raise RegistryError(
                f"registered function class {get_qualified_name(func)}"
                f" is not a subclass of FunctionBase"
            )
        if impl is not None and not issubclass(func, impl):
            raise RegistryError(
                f"given function class {get_qualified_name(func)} does not match"
                f" given type {get_qualified_name(type)}, remove or change impl=<...>"
            )

        func = cast(Type[Function], func)
        return map_cls_to_function_cls(func)
    else:  # func is a Python function and must be mapped to FunctionBase subtype
        if impl is None:
            raise RegistryError(
                f"non-class function {get_qualified_name(func)} must specify its type"
                f" via register, like with register(..., impl=Transform)"
            )

        func = cast(Callable, func)
        return map_callable_to_function_cls(func, impl)


def map_cls_to_function_cls(func: Type[Function]) -> Type[Function]:
    if hasattr(func, "config_spec"):
        declared_config_spec = convert_to_config_spec(func.config_spec)
    else:
        declared_config_spec = None
    # TODO @Robustness: check declared_config_spec against inferred_config_spec
    inferred_config_spec = infer_config_spec(func.__init__)  # noqa

    # overwrite config spec with clean config
    config_spec = declared_config_spec or inferred_config_spec
    func.config_spec = config_spec
    return func


def map_callable_to_function_cls(func: Callable, impl: Type[Function]) -> Type[Function]:
    """Maps a callable representing a Function to an actual FunctionBase type

    Implementation is basic right now and cannot construct any complex functions.
    """

    inferred_config_spec = infer_config_spec(func)
    try:
        inferred_input_type = reduce_to_record_type_spec(inferred_config_spec.type)
    except ValueError as e:
        raise ValueError("callable function definition has non-FieldType parameters") from e
    inferred_input_spec = convert_to_record_spec(inferred_input_type)

    actual_config_spec = ConfigSpec(
        name=inferred_config_spec.name,
        description=inferred_config_spec.description,
        type={},  # actual config is empty
    )
    attrs: dict = {
        "__call__": func,
        "config_spec": actual_config_spec,
    }
    # We currently only support one input type, one output type for
    # callable RecordFunctions and assume that the spec is equal. This is
    # very simplistic and we may want more complex behavior later.
    if issubclass(impl, RecordFunction):
        attrs["input_spec"] = inferred_input_spec
        if issubclass(impl, RecordTransform):
            if len(inferred_config_spec.type) != 1:
                raise ValueError(
                    f"mapping callable {get_qualified_name(func)} with !=1 arguments is not supported"
                )
            attrs["output_spec"] = inferred_input_spec
    else:
        raise ValueError(
            f"mapping callable {get_qualified_name(func)} to non-Record functions it not supported"
        )

    func_cls: Type[Function] = cast(Type[Function], type(inferred_config_spec.name, (impl,), attrs))
    return func_cls


functions: Registry[Type[Function]] = Registry(("functions",), mapper=map_to_function_cls)
# TODO @Cleanup: figure out better registration mechanism for registered objects
import bench.function.metrics  # noqa
import bench.function.test  # noqa
import bench.function.transform.text  # noqa
import bench.function.utils  # noqa


class FunctionType(enum.Enum):
    RecordTransform = "RecordTransform"
    Metric = "MetricFunction"
    Test = "Test"


@dataclass
class FunctionHandlerSpec:
    name: str
    description: str
    type: Union[Literal["RecordTransform"], Literal["MetricFunction"], Literal["Test"]]
    config_spec: ConfigSpec
    base_spec: FunctionSpec


def get_function_handler_specs() -> list[FunctionHandlerSpec]:
    function_handler_specs = []
    for handler_id in functions.names():
        function_handler_spec = get_function_handler_spec(handler_id)
        function_handler_specs.append(function_handler_spec)
    return function_handler_specs


def get_function_type(function_cls: Type[Function]) -> FunctionType:
    if issubclass(function_cls, Test):
        return FunctionType.Test
    elif issubclass(function_cls, Metric):
        return FunctionType.Metric
    elif issubclass(function_cls, RecordTransform):
        return FunctionType.RecordTransform
    else:
        raise ValueError(f"unexpected function type: {function_cls}")


def get_function_handler_spec(handler_id: str) -> FunctionHandlerSpec:
    function_cls = get_function_cls(handler_id)
    config_spec: ConfigSpec = function_cls.config_spec
    function_type = get_function_type(function_cls)

    if not issubclass(function_cls, RecordFunction):
        raise ValueError(f"unexpected function: {function_cls}")

    base_spec = FunctionSpec(
        name=handler_id,
        description=config_spec.description,
        input_spec=function_cls.input_spec,
        output_spec=function_cls.output_spec,
    )
    function_handler_spec = FunctionHandlerSpec(
        name=handler_id,
        description=base_spec.description,
        type=function_type.value,
        config_spec=config_spec,
        base_spec=base_spec,
    )
    return function_handler_spec


def get_config_spec(function_id: str) -> ConfigSpec:
    return convert_to_config_spec(functions[function_id].config_spec)


def get_function_cls(handler_id: str) -> Type[Function]:
    function_cls: Type[Function] = functions[handler_id]
    return function_cls


def load_function(function_id: str, arguments: Dict[str, Any]) -> Function:
    function_cls = get_function_cls(function_id)
    # noinspection PyArgumentList
    function = function_cls(**arguments)
    return function
