import abc
from typing import Any, Dict, List, Optional, Union

import catalogue

from bench.utils.record import ListRecordBatch, Record, RecordBatch
from bench.utils.spec import ConfigSpec, ModelSpec


class ModelBase(abc.ABC):
    """
    Base for other model implementations that can load and run a model from some source.
    """

    spec: Optional[ModelSpec]
    config_spec: ConfigSpec

    def get_spec(self) -> ModelSpec:
        """
        Dynamic model spec if model schema can only be fully determined after loading.
        """
        if self.spec is None:
            raise ValueError(
                "model must define either Model.model_spec or dynamic property model.model_spec"
            )

        return self.spec

    def forward(self, record: Record) -> Union[Record, RecordBatch]:
        raise NotImplementedError

    def forward_batch(self, records: RecordBatch) -> RecordBatch:
        raise NotImplementedError


class BatchModelBase(ModelBase, abc.ABC):
    """
    A naive ModelBase.forward_batch implementation that just iterates over forward.
    """

    def forward_batch(self, records: RecordBatch) -> RecordBatch:
        output_records: List[Record] = []
        for record in records:
            output = self.forward(record)
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
    model_base_id: str,
    storage_uri: Optional[str],
    arguments: Dict[str, Any],
    model_spec: Optional[ModelSpec],
) -> ModelBase:
    model_cls = models.get(model_base_id)
    model = model_cls(**arguments)
    return model
