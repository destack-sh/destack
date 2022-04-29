import abc
from typing import AbstractSet, Any, Dict, List, Optional, Set, Type, Union

from bench.utils.record import ListRecordBatch, Record, RecordBatch
from bench.utils.registry import Registry
from bench.utils.spec import (
    ConfigSpec,
    ConfigType,
    ModelSpec,
    ModelType,
    convert_to_config_spec,
)


# TODO @Cleanup: Model"Handler" is not a great name (not descriptive enough)
class ModelHandler(abc.ABC):
    """
    Base for model implementations that can load and run a model from some source.
    TODO @Feature: specify model affordances for different tasks
    TODO @Feature: define common task/model specs
    """

    # Known base model spec for all models of this handler.
    base_spec: Union[None, ModelType, ModelSpec] = None
    # Config spec for configuring this handler.
    config_spec: Union[ConfigType, ConfigSpec]
    # Config values that are immutable after init. If not set, defaults to all keys.
    config_static_keys: Set[str]

    def __init__(
        self,
        spec: Union[None, ModelType, ModelSpec] = None,
        version: Optional[str] = None,
    ):
        if spec is None and self.base_spec is None:
            raise ValueError(
                "ModelHandler must define either `base_spec` or get `spec` argument"
            )
        elif spec is not None:
            self.base_spec = spec
        self.version = version

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
        return ListRecordBatch(output_records)


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
    model = model_cls(spec=spec, **arguments)
    return model
