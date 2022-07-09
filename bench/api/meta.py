from drf_spectacular.utils import extend_schema
from rest_framework import serializers
from rest_framework.decorators import api_view
from rest_framework.request import Request
from rest_framework.response import Response

from bench.model.base import models
from bench.utils.spec import convert_to_config_spec, convert_to_model_spec


class SpecSerializer(serializers.Serializer):
    name = serializers.CharField()
    description = serializers.CharField(required=False)


class FieldTypeSerializer(serializers.Serializer):
    pass


class FieldSpecSerializer(SpecSerializer):
    type = FieldTypeSerializer()


class ConfigSpecSerializer(SpecSerializer):
    pass


class RecordSpecSerializer(SpecSerializer):
    pass


class DatasetSpecSerializer(SpecSerializer):
    record_spec = RecordSpecSerializer()


class ModelSpecSerializer(SpecSerializer):
    input_spec = RecordSpecSerializer()


class RecordFunctionSerializer(serializers.Serializer):
    name = serializers.CharField()
    description = serializers.CharField(required=False)
    config_spec = ConfigSpecSerializer()
    input_spec = RecordSpecSerializer()
    output_spec = RecordSpecSerializer()


class DatasetHandlerSerializer(serializers.Serializer):
    name = serializers.CharField()
    description = serializers.CharField(required=False)
    base_spec = RecordSpecSerializer()


class ModelHandlerSerializer(serializers.Serializer):
    name = serializers.CharField()
    description = serializers.CharField(required=False)
    input_spec = RecordSpecSerializer()
    output_spec = RecordSpecSerializer()


@extend_schema(responses=ModelHandlerSerializer(many=True))
@api_view(["GET"])
def list_model_handlers(request: Request):
    model_handlers = []
    for model_handler_type in models:
        base_spec = None
        if model_handler_type.base_spec is not None:
            base_spec = convert_to_model_spec(model_handler_type.base_spec)
        model_handler = {
            "config_spec": convert_to_config_spec(model_handler_type.config_spec),
            "base_spec": base_spec,
        }
        model_handlers.append(model_handler)

    serialized_models = ModelHandlerSerializer(model_handlers, many=True).data
    return Response(serialized_models)
