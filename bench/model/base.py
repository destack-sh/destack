import abc
from typing import Any, Dict, List, Optional, Union

import catalogue

from bench.utils.record import ListRecordBatch, Record, RecordBatch
from bench.utils.spec import ConfigSpec, ModelSpec


# TODO @Cleanup: Model"Handler" is not a great name not descriptive enough
class ModelHandler(abc.ABC):
    """
    Base for other model implementations that can load and run a model from some source.
    TODO @Feature: specify model affordances for different tasks
    TODO @Feature: define common task/model specs
    """

    spec: Optional[ModelSpec] = None
    config_spec: ConfigSpec

    def __init__(self, spec: Optional[ModelSpec] = None, version: Optional[str] = None):
        if spec is None and self.spec is None:
            raise ValueError(
                "ModelHandler must define either static `spec` or dynamic `get_spec`"
            )
        elif spec is not None:
            self.spec = spec
        self.version = version

    def predict(self, record: Record) -> Union[Record, RecordBatch]:
        raise NotImplementedError

    def predict_batch(self, records: RecordBatch) -> RecordBatch:
        raise NotImplementedError


class BatchModelHandler(ModelHandler, abc.ABC):
    """
    A naive ModelBase.forward_batch implementation that just iterates over forward.
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


def load_model(
    handler_id: str,
    storage_uri: Optional[str],
    version: Optional[str],
    arguments: Dict[str, Any],
    spec: Optional[ModelSpec],
) -> ModelHandler:
    model_cls = models.get(handler_id)
    model = model_cls(version=version, spec=spec, **arguments)
    return model
