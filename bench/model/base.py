import abc
from dataclasses import dataclass
from typing import (
    AbstractSet,
    Any,
    List,
    Mapping,
    Optional,
    Type,
    Union,
    cast,
)
from uuid import UUID

from fsspec import AbstractFileSystem

from bench.artifact.base import ArtifactHandler
from bench.artifact.utils import map_to_artifact_cls
from bench.dataset.accessor import get_file_system
from bench.utils.record import Record, RecordBatch, RecordList
from bench.utils.registry import Registry
from bench.utils.spec import FieldSpec, ModelType, RecordSpec, convert_to_record_spec
from bench.utils.validate import cast_config_arguments, validate_config_type


@dataclass
class ModelHandlerMetadata:
    name: str
    description: str
    tags: list[str]


class ModelHandler(ArtifactHandler):
    """
    Base for model implementations that can load and run a model from some source.
    """

    # Known base model spec for all models of this handler.
    metadata: ModelHandlerMetadata
    base_spec: Optional[ModelType] = None
    spec: ModelType

    def __init__(
        self,
        artifact_id: UUID,
        version: Optional[str] = None,
        fs: Optional[AbstractFileSystem] = None,
        path: Optional[str] = None,
        spec: Optional[ModelType] = None,
    ):
        super().__init__(artifact_id=artifact_id, version=version, fs=fs, path=path)
        if spec is None:
            if self.base_spec is None:
                raise ValueError("ModelHandler must define `base_spec` or get `spec` argument")
            else:
                self.spec = self.base_spec
        else:
            self.spec = spec

    def update_config(self, **kwargs):
        pass

    def predict(self, record: Record) -> Union[Record, RecordBatch]:
        raise NotImplementedError

    def predict_batch(self, records: RecordBatch) -> RecordBatch:
        raise NotImplementedError


class UnbatchedModelHandler(ModelHandler, abc.ABC):
    """
    A naive ModelHandler.predict_batch implementation that just iterates over predict.
    """

    def predict_batch(self, records: RecordBatch) -> RecordBatch:
        output_records: List[Record] = []
        for record in records:
            output = self.predict(record)
            # if we're getting batches, flatten them into output
            if isinstance(output, RecordBatch):
                output_records.extend(output)
            else:
                output_records.append(output)
        return RecordList(output_records)


class BatchedModelHandler(ModelHandler, abc.ABC):
    """
    A naive ModelHandler.predict implementation that just aggregates into lists.
    """

    def predict(self, record: Record) -> Union[Record, RecordBatch]:
        input_records = RecordList([record])
        output_records = self.predict_batch(input_records)
        if len(output_records) == 1:
            return output_records[0]
        else:
            return output_records


SPECIAL_MODEL_CONFIG_KEYS = {"fs", "path", "artifact_id", "version", "spec"}


def map_to_model_cls(model_cls: Type[ModelHandler], *args) -> Type[ModelHandler]:
    return map_to_artifact_cls(model_cls, ignore_keys=SPECIAL_MODEL_CONFIG_KEYS)


models: Registry[Type[ModelHandler]] = Registry(("models",), mapper=map_to_model_cls)


def _import_models():
    # TODO @Feature: figure out better registration mechanism for registered objects
    import bench.model.huggingface  # noqa
    import bench.model.openai  # noqa
    import bench.model.spacy_  # noqa


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
    storage_uri: Optional[str],
    version: Optional[str],
    arguments: dict[str, Any],
    spec: Optional[ModelType],
) -> ModelHandler:
    model_cls = get_model_cls(handler_id)
    if storage_uri:
        fs, path = get_file_system(storage_uri)
    else:
        fs, path = None, None

    config_type = model_cls.config_spec
    validate_config_type(arguments, config_type, ignore_extraneous=True)
    arguments = cast_config_arguments(arguments, config_type)

    # first add optional special arguments (also see SPECIAL_DATASET_CONFIG_KEYS)
    arguments = dict(**arguments, fs=fs, path=path, artifact_id=artifact_id)
    # filter arguments to only those listed
    arguments = {key: value for key, value in arguments.items() if key in config_type}
    # all handlers must take id, version & spec, so add mandatory arguments last
    model = model_cls(spec=spec, artifact_id=artifact_id, version=version, **arguments)  # noqa
    return model
