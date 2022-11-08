import abc
from dataclasses import dataclass
from typing import AbstractSet, Any, List, Mapping, Optional, Type, cast
from uuid import UUID

from bench.utils.record import Record
from bench.utils.registry import Registry
from bench.utils.spec import (
    FieldSpec,
    ModelType,
    RecordSpec,
    convert_to_config_type_spec,
    convert_to_record_spec,
)
from bench.utils.validate import cast_config_arguments, validate_config_type


@dataclass
class ModelHandlerMetadata:
    name: str
    description: str
    tags: list[str]


class ModelHandler(abc.ABC):
    """
    Base for model implementations that can load and run a model from some source.
    """

    # Config spec to configure this handler.
    config_spec: Mapping[str, FieldSpec]

    async def complete(self, prompt: str) -> tuple[str, list[float]]:
        raise NotImplementedError

    async def classify(
        self, text: str, labels: list[str], examples: list[tuple[str, str]]
    ) -> Record:
        raise NotImplementedError

    async def embed(self, text: str) -> bytes:
        raise NotImplementedError


SPECIAL_MODEL_CONFIG_KEYS = {"artifact_id", "version", "spec"}


def map_to_model_cls(model_cls: Type[ModelHandler], *args) -> Type[ModelHandler]:
    if hasattr(model_cls, "config_spec"):
        declared_config_spec = convert_to_config_type_spec(model_cls.config_spec)
    else:
        declared_config_spec = None
    # TODO @Robustness: check declared_config_spec against inferred_config_spec
    inferred_config_spec = infer_config_type(model_cls.__init__)  # noqa

    # overwrite config spec with clean config
    config_spec = declared_config_spec or inferred_config_spec
    # filter to exclude ignored keys
    config_spec = {
        key: typ for key, typ in config_spec.items() if key not in SPECIAL_MODEL_CONFIG_KEYS
    }
    model_cls.config_spec = config_spec
    return model_cls


models: Registry[Type[ModelHandler]] = Registry(("models",), mapper=map_to_model_cls)


def _import_models():
    # TODO @Feature: figure out better registration mechanism for registered objects
    import bench.model.huggingface  # noqa
    import bench.model.openai  # noqa


def get_model_cls(handler_id: str) -> Type[ModelHandler]:
    _import_models()

    model_cls: Type[ModelHandler] = models[handler_id]
    return model_cls


@dataclass
class ModelHandlerSpec:
    id: str
    name: str
    description: str
    tags: list[str]
    base_spec: Optional[ModelType]
    config_spec: Mapping[str, FieldSpec]


def get_model_handler_specs() -> List[ModelHandlerSpec]:
    _import_models()

    model_handler_specs = []
    for handler_id in models.names():
        model_handler_spec = get_model_handler_spec(handler_id)
        model_handler_specs.append(model_handler_spec)
    return model_handler_specs


def get_model_handler_spec(handler_id: str):
    model_cls = get_model_cls(handler_id)
    base_spec = get_model_base_spec(handler_id)
    model_handler_spec = ModelHandlerSpec(
        id=handler_id,
        name=model_cls.metadata.name,
        description=model_cls.metadata.description,
        tags=model_cls.metadata.tags,
        base_spec=base_spec,
        config_spec=model_cls.config_spec,
    )
    return model_handler_spec


def get_model_base_spec(handler_id: str) -> ModelType:
    model_cls = get_model_cls(handler_id)

    # if available, use defined base spec, else use blank input/output spec
    if model_cls.base_spec is not None:
        input_spec = convert_to_record_spec(model_cls.base_spec.input_spec)
        output_spec = convert_to_record_spec(model_cls.base_spec.output_spec)
    else:
        input_spec = RecordSpec(name="input", description="unspecified record (blank)", type={})
        output_spec = RecordSpec(name="output", description="unspecified record (blank)", type={})

    model_spec = ModelType(input_spec=input_spec, output_spec=output_spec)
    return model_spec


def get_variable_config_keys(handler_id: str) -> AbstractSet[str]:
    model_cls = get_model_cls(handler_id)
    config_spec = convert_to_record_spec(model_cls.config_spec)
    # all config types for models are assumed to be dicts
    all_keys = cast(dict, config_spec.type).keys()
    static_keys = model_cls.config_static_keys or set()
    variable_keys = all_keys - static_keys
    return variable_keys


def get_static_config_keys(handler_id: str) -> set[str]:
    model_cls = get_model_cls(handler_id)
    return model_cls.config_static_keys


def load_model(
    handler_id: str,
    artifact_id: UUID,
    arguments: dict[str, Any],
) -> ModelHandler:
    raise NotImplementedError

    model_cls = get_model_cls(handler_id)
    config_type = model_cls.config_spec
    arguments = cast_config_arguments(arguments, config_type)
    validate_config_type(arguments, config_type, ignore_extraneous=True)

    # first add optional special arguments (also see SPECIAL_DATASET_CONFIG_KEYS)
    arguments = dict(**arguments, fs=fs, path=path, artifact_id=artifact_id)
    # filter arguments to only those listed
    arguments = {key: value for key, value in arguments.items() if key in config_type}
    # all handlers must take id, version & spec, so add mandatory arguments last
    model = model_cls(spec=spec, artifact_id=artifact_id, version=version, **arguments)  # noqa
    return model
