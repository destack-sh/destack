from drf_spectacular.utils import extend_schema
from rest_framework.decorators import api_view
from rest_framework.request import Request
from rest_framework.response import Response

from bench.dataset.base import DatasetHandlerSpec, get_dataset_handler_specs
from bench.function.base import FunctionHandlerSpec, get_function_handler_specs
from bench.model.base import ModelHandlerSpec, get_model_handler_specs
from bench.utils.serializer import (
    DatasetHandlerSpecSerializer,
    FunctionHandlerSpecSerializer,
    ModelHandlerSpecSerializer,
)


@extend_schema(responses=ModelHandlerSpecSerializer(many=True))
@api_view(["GET"])
def list_model_handlers(request: Request):
    specs: list[ModelHandlerSpec] = get_model_handler_specs()
    serialized_models = ModelHandlerSpecSerializer(specs, many=True).data
    return Response(serialized_models)


@extend_schema(responses=DatasetHandlerSpecSerializer(many=True))
@api_view(["GET"])
def list_dataset_handlers(request: Request):
    specs: list[DatasetHandlerSpec] = get_dataset_handler_specs()
    serialized_datasets = DatasetHandlerSpecSerializer(specs, many=True).data
    return Response(serialized_datasets)


@extend_schema(responses=FunctionHandlerSpecSerializer(many=True))
@api_view(["GET"])
def list_function_handlers(request: Request):
    specs: list[FunctionHandlerSpec] = get_function_handler_specs()
    serialized_functions = FunctionHandlerSpecSerializer(specs, many=True).data
    return Response(serialized_functions)
