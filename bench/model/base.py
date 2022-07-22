import abc
from dataclasses import dataclass
from typing import AbstractSet, Any, List, Optional, Type, Union

from fsspec import AbstractFileSystem

from bench.artifact.base import ArtifactHandler
from bench.dataset.accessor import get_file_system
from bench.utils.record import Record, RecordBatch, RecordList
from bench.utils.registry import Registry
from bench.utils.spec import (
    ConfigSpec,
    ModelSpec,
    RecordSpec,
    convert_to_config_spec,
    convert_to_record_spec,
    infer_config_spec,
    infer_config_type,
    infer_description,
)


class ModelHandler(ArtifactHandler):
    """
    Base for model implementations that can load and run a model from some source.
    """

    # Known base model spec for all models of this handler.
    base_spec: Optional[ModelSpec] = None
    spec: ModelSpec

    def __init__(
        self,
        fs: Optional[AbstractFileSystem] = None,
        path: Optional[str] = None,
        version: Optional[str] = None,
        spec: Optional[ModelSpec] = None,
    ):
        super().__init__(fs=fs, path=path, version=version)
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
    A naive ModelBase.forward_batch implementation that just iterates over predict.
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


def map_to_model_cls(model_cls: Type[ModelHandler], *args) -> Type[ModelHandler]:
    if hasattr(model_cls, "config_spec"):
        declared_config_spec = convert_to_config_spec(model_cls.config_spec)
    else:
        declared_config_spec = None
    # TODO @Robustness: check declared_config_spec against inferred_config_spec
    inferred_config_spec = infer_config_spec(model_cls.__init__)  # noqa

    # overwrite config spec with clean config
    config_spec = declared_config_spec or inferred_config_spec
    model_cls.config_spec = config_spec
    return model_cls


models: Registry[Type[ModelHandler]] = Registry(("models",), mapper=map_to_model_cls)
# TODO @Feature: figure out better registration mechanism for registered objects
import bench.model.huggingface  # noqa
import bench.model.openai  # noqa
import bench.model.spacy_  # noqa


@dataclass
class ModelHandlerSpec:
    name: str
    description: str
    base_spec: Optional[ModelSpec]
    config_spec: ConfigSpec


def get_model_handler_specs() -> List[ModelHandlerSpec]:
    model_handler_specs = []
    for handler_id in models.names():
        model_handler_spec = get_model_handler_spec(handler_id)
        model_handler_specs.append(model_handler_spec)
    return model_handler_specs


def get_model_handler_spec(handler_id: str):
    model_cls = get_model_cls(handler_id)
    base_spec = get_model_base_spec(handler_id)
    model_handler_spec = ModelHandlerSpec(
        name=base_spec.name,
        description=base_spec.description,
        base_spec=base_spec,
        config_spec=model_cls.config_spec,
    )
    return model_handler_spec


def get_model_base_spec(handler_id: str) -> ModelSpec:
    model_cls = get_model_cls(handler_id)
    inferred_description = infer_description(model_cls) or ""

    # if available, use defined base spec, else use blank input/output spec
    if model_cls.base_spec is not None:
        input_spec = convert_to_record_spec(model_cls.base_spec.input_spec)
        output_spec = convert_to_record_spec(model_cls.base_spec.output_spec)
    else:
        input_spec = RecordSpec(name="", description="", type={})
        output_spec = RecordSpec(name="", description="", type={})

    model_spec = ModelSpec(
        name=handler_id,
        description=inferred_description,
        input_spec=input_spec,
        output_spec=output_spec,
    )
    return model_spec


def get_model_cls(handler_id: str) -> Type[ModelHandler]:
    model_cls: Type[ModelHandler] = models[handler_id]
    return model_cls


def get_variable_config_keys(handler_id: str) -> AbstractSet[str]:
    model_cls = get_model_cls(handler_id)
    config_spec = convert_to_config_spec(model_cls.config_spec)
    all_keys = config_spec.type.keys()
    static_keys = model_cls.config_static_keys or set()
    variable_keys = all_keys - static_keys
    return variable_keys


def get_static_config_keys(handler_id: str) -> set[str]:
    model_cls = get_model_cls(handler_id)
    return model_cls.config_static_keys


def load_model(
    handler_id: str,
    storage_uri: Optional[str],
    version: Optional[str],
    arguments: dict[str, Any],
    spec: Optional[ModelSpec],
) -> ModelHandler:
    model_cls = get_model_cls(handler_id)
    if storage_uri:
        fs, path = get_file_system(storage_uri)
    else:
        fs, path = None, None

    config_type = infer_config_type(model_cls)
    arguments = dict(**arguments, fs=fs, path=path)
    # filter arguments to only those listed
    arguments = {key: value for key, value in arguments.items() if key in config_type}
    model = model_cls(spec=spec, version=version, **arguments)  # noqa
    return model
