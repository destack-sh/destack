import abc
from typing import Any, Dict, List, Optional, Set, Type, Union

import catalogue

from bench.utils.record import ListRecordBatch, Record, RecordBatch
from bench.utils.spec import ConfigSpec, ModelSpec


# TODO @Cleanup: Model"Handler" is not a great name (not descriptive enough)
class ModelHandler(abc.ABC):
    """
    Base for model implementations that can load and run a model from some source.
    TODO @Feature: specify model affordances for different tasks
    TODO @Feature: define common task/model specs
    """

    # Known base model spec for all models of this handler.
    base_spec: Optional[ModelSpec] = None
    # Config spec to configure this handler.
    config_spec: ConfigSpec
    # Config values that are immutable after init. If not set, defaults to all keys.
    config_static_keys: Set[str]

    def __init__(self, spec: Optional[ModelSpec] = None, version: Optional[str] = None):
        if spec is None and self.base_spec is None:
            raise ValueError(
                "ModelHandler must define either static `spec` or dynamic `get_spec`"
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


models = catalogue.create("bench", "models", entry_points=True)
# TODO @Feature: figure out better registration mechanism for models/datasets/functions
import bench.model.huggingface  # noqa
import bench.model.openai  # noqa
import bench.model.spacy_  # noqa


def get_model_cls(handler_id: str) -> Type[ModelHandler]:
    model_cls: Type[ModelHandler] = models.get(handler_id)
    return model_cls


def get_variable_config_keys(handler_id: str) -> Set[str]:
    model_cls = get_model_cls(handler_id)
    all_keys = model_cls.config_spec.keys()
    static_keys = model_cls.config_static_keys or set()
    variable_keys = all_keys - static_keys
    return variable_keys


def get_static_config_keys(handler_id: str) -> Set[str]:
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
    model = model_cls(version=version, spec=spec, **arguments)
    return model
