from drf_spectacular.utils import extend_schema
from rest_framework import serializers
from rest_framework.decorators import api_view
from rest_framework.request import Request
from rest_framework.response import Response

from bench.model.base import get_model_handler_specs


class SpecSerializer(serializers.Serializer):
    name = serializers.CharField()
    description = serializers.CharField(required=False)


class FieldSpecSerializer(SpecSerializer):
    type = serializers.DictField(child=serializers.DictField())
    pass


class ConfigSpecSerializer(SpecSerializer):
    type = serializers.DictField(child=serializers.DictField())
    pass


class RecordSpecSerializer(SpecSerializer):
    # type = serializers.DictField(child=serializers.DictField())
    pass


class DatasetSpecSerializer(SpecSerializer):
    record_spec = RecordSpecSerializer()


class ModelSpecSerializer(SpecSerializer):
    input_spec = RecordSpecSerializer()
    output_spec = RecordSpecSerializer()


class RecordFunctionSerializer(serializers.Serializer):
    name = serializers.CharField()
    description = serializers.CharField(required=False)
    config_spec = ConfigSpecSerializer()
    input_spec = RecordSpecSerializer()
    output_spec = RecordSpecSerializer()


class DatasetHandlerSerializer(serializers.Serializer):
    name = serializers.CharField()
    description = serializers.CharField(required=False)
    base_spec = DatasetSpecSerializer()
    config_spec = ConfigSpecSerializer()


class ModelHandlerSerializer(serializers.Serializer):
    name = serializers.CharField()
    description = serializers.CharField(required=False)
    base_spec = ModelSpecSerializer()
    config_spec = ConfigSpecSerializer()


@extend_schema(responses=ModelHandlerSerializer(many=True))
@api_view(["GET"])
def list_model_handlers(request: Request):
    specs = get_model_handler_specs()
    serialized_models = ModelHandlerSerializer(specs, many=True).data
    return Response(serialized_models)
