import typing
from typing import Iterator, List, Optional, Union

import datasets as hf_datasets

from bench.dataset.base import DatasetHandler, datasets
from bench.utils.record import Record, RecordBatch, RecordList
from bench.utils.spec import (
    DatasetSpec,
    FieldSpec,
    FieldType,
    RecordSpec,
    RecordTypeStrict,
)


@datasets.register("bench.huggingface")
class HuggingFaceDatasetHandler(DatasetHandler):
    def __init__(
        self,
        dataset_name: str,
        version: Optional[str] = None,
        split: str = "train",
        use_auth_token: Optional[str] = None,
    ):
        """
        A HuggingFace datasets-backed Dataset handler.
        @param dataset_name: Path or name of the HF dataset.
        @param version: The revision to load.
        @param split: The split to load.
        @param use_auth_token: The authentication token to use.
        """

        self.dataset_name = dataset_name
        # We specify split and streaming=False, so we'll always get a Dataset instance.
        self._dataset: hf_datasets.Dataset = hf_datasets.load_dataset(  # noqa
            dataset_name, revision=version, split=split, use_auth_token=use_auth_token
        )
        record_type = _hf_features_to_record_type(self._dataset.features)
        record_spec = RecordSpec(type=record_type, name="", description="")
        dataset_spec = DatasetSpec(
            name=dataset_name,
            description=self._dataset.info.description,
            record_spec=record_spec,
        )
        super().__init__(spec=dataset_spec)

    @typing.overload
    def __getitem__(self, index: int) -> Record:
        ...

    @typing.overload
    def __getitem__(self, index: slice) -> RecordBatch:
        ...

    @typing.overload
    def __getitem__(self, index: str) -> List[FieldType]:
        ...

    def __getitem__(
        self, index: Union[int, slice, str]
    ) -> Union[Record, RecordBatch, List[FieldType]]:
        if isinstance(index, int):
            return self._dataset[index]
        elif isinstance(index, slice):
            # Convert dataset slice into expected record batch format for slices.
            # HF datasets do support slicing, but will return a dict of field lists.
            # TODO @Performance: creating record list from dataset slice is inefficient
            indices = range(index.start or 0, index.stop or len(self), index.step or 1)
            records = [self._dataset[i] for i in indices]
            return RecordList(records)
        elif isinstance(index, str):
            return self._dataset[index]
        else:
            raise TypeError(index)

    def __iter__(self) -> Iterator[Record]:
        return iter(self._dataset)

    def __len__(self) -> int:
        return len(self._dataset)


def _hf_features_to_record_type(features: hf_datasets.Features) -> RecordTypeStrict:
    record_type: dict[str, FieldSpec] = {}

    for feature_key, feature_type in features.items():
        # TODO @Feature: map HF Features to spec types
        mapped_type = FieldSpec(name=feature_key, description="", type=feature_type)
        record_type[feature_key] = mapped_type

    return record_type
