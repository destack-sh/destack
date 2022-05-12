import abc
from typing import AbstractSet, Any, Dict, List, Optional, Type, Union

from fsspec import AbstractFileSystem

from bench.artifact.base import ArtifactHandler
from bench.utils.record import Record, RecordBatch, RecordList
from bench.utils.registry import Registry
from bench.utils.spec import ModelSpec, ModelType, convert_to_config_spec


class ModelHandler(ArtifactHandler):
    """
    Base for model implementations that can load and run a model from some source.
    """

    # Known base model spec for all models of this handler.
    base_spec: Union[None, ModelType, ModelSpec] = None

    def __init__(
        self,
        fs: Optional[AbstractFileSystem] = None,
        path: Optional[str] = None,
        version: Optional[str] = None,
        spec: Union[None, ModelType, ModelSpec] = None,
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


models: Registry[Type[ModelHandler]] = Registry(("models",))
# TODO @Feature: figure out better registration mechanism for registered objects
import bench.model.huggingface  # noqa
import bench.model.openai  # noqa
import bench.model.spacy_  # noqa


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
    arguments: Dict[str, Any],
    spec: Optional[ModelSpec],
) -> ModelHandler:
    model_cls = get_model_cls(handler_id)
    model = model_cls(spec=spec, version=version, **arguments)  # noqa
    return model
