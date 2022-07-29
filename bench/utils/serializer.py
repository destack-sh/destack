import copy
import dataclasses
from typing import Any, Type, Union

from django.utils.translation import gettext_lazy as _
from rest_framework import serializers
from rest_framework.fields import DictField

from bench.utils.spec import (
    CONFIG_SPEC_TYPES,
    CONFIG_TYPES,
    FIELD_SPEC_TYPES,
    FIELD_TYPES,
    FieldSpec,
    FieldType,
    FieldTypeSpec,
    RecordSpec,
    _Spec,
    _Type,
)


class SpecSerializer(serializers.Serializer):
    name = serializers.CharField(allow_blank=True)
    description = serializers.CharField(allow_blank=True, required=False)


class SpecField(serializers.Field):
    default_error_messages = {
        "not_a_dict": _('Expected a dictionary of items but got type "{input_type}".'),
        "not_a_type": _('Expected a type with _type field but got "{value}"'),
        "empty": _("This dictionary may not be empty."),
    }

    def __init__(self, types: list[Union[Type[_Type], Type[_Spec]]], **kwargs):
        super().__init__(**kwargs)
        self._types = list(types)
        self._types_by_name = {cls.__name__: cls for cls in self._types}

    def to_representation(self, obj):
        # modified dataclasses.asdict to encode computed _type field
        if dataclasses.is_dataclass(obj):
            result = {}
            for f in dataclasses.fields(obj):
                value = self.to_representation(getattr(obj, f.name))
                result[f.name] = value
            result["_type"] = type(obj).__name__
            return result
        elif isinstance(obj, (list, tuple)):
            return type(obj)(self.to_representation(v) for v in obj)
        elif isinstance(obj, dict):
            return type(obj)(
                (self.to_representation(k), self.to_representation(v)) for k, v in obj.items()
            )
        else:
            return copy.deepcopy(obj)

    def to_internal_value(self, data: Any) -> Union[FieldSpec, FieldTypeSpec, FieldType]:
        if isinstance(data, (tuple, list)):
            return [self.to_internal_value(item) for item in data]  # type: ignore
        elif not isinstance(data, dict):
            self.fail("not_a_dict", input_type=type(data))
        if len(data) == 0:
            # empty is a valid type
            return {}  # type: ignore
        # recursively deserialize field
        _type = data.pop("_type", None)
        if _type is not None:
            # if it's a dataclass type, instantiate that types
            type_cls = self._types_by_name.get(_type)
            if type_cls is None:
                self.fail("not_a_type", value=data)
            child_data = {**data}
            # some types also have name and description which bypass the spec/type only deserialization
            if issubclass(type_cls, _Spec) and "type" in data:
                child_data["type"] = self.to_internal_value(data["type"])
            return type_cls(**child_data)  # type: ignore
        else:
            # otherwise simply re-construct the dict
            return {key: self.to_internal_value(value) for key, value in data.items()}  # type: ignore


class FieldSpecSerializer(SpecSerializer):
    type = SpecField(types=[*FIELD_TYPES, *FIELD_SPEC_TYPES])


class ConfigSpecSerializer(SpecSerializer):
    type = SpecField(types=[*CONFIG_TYPES, *CONFIG_SPEC_TYPES])


class RecordSpecSerializer(SpecSerializer):
    type = SpecField(types=[*FIELD_TYPES, *FIELD_SPEC_TYPES])

    def create(self, validated_data):
        return RecordSpec(**validated_data)


class DatasetSpecSerializer(SpecSerializer):
    record_spec = RecordSpecSerializer()


class ModelSpecSerializer(SpecSerializer):
    input_spec = RecordSpecSerializer()
    output_spec = RecordSpecSerializer()


class FunctionSpecSerializer(SpecSerializer):
    input_spec = DictField(child=SpecField(types=[*FIELD_TYPES, *FIELD_SPEC_TYPES]))
    output_spec = DictField(child=SpecField(types=[*FIELD_TYPES, *FIELD_SPEC_TYPES]))


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


class FunctionHandlerSerializer(serializers.Serializer):
    name = serializers.CharField()
    description = serializers.CharField(required=False)
    type = serializers.CharField()
    config_spec = ConfigSpecSerializer()
    base_spec = FunctionSpecSerializer()
