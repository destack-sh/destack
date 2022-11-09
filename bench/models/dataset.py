from __future__ import annotations

import dataclasses
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Iterable, Iterator, Optional, Sequence

from django.contrib.postgres.indexes import GinIndex
from django.contrib.postgres.search import SearchVector
from django.db import connection, models, transaction
from django.db.models import QuerySet

from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH, UUIDModel, proxies
from bench.models.versioning import VersionedBlob, VersionedTree

if TYPE_CHECKING:
    from bench.models import Organization, Project

from bench.models.tag import TaggableMixin


class DatasetManager(models.Manager):
    def create_dataset_version(
        self,
        name: str,
        organization: Organization,
        project: Project,
        **kwargs,
    ) -> DatasetVersion:
        """Creates dataset version and corresponding dataset if it doesn't exist"""
        with transaction.atomic():
            dataset, _ = Dataset.objects.get_or_create(
                organization=organization, project=project, name=name
            )
            return DatasetVersion.objects.create(dataset=dataset, **kwargs)


class Dataset(TaggableMixin, UUIDModel):
    """
    A dataset of JSON records.

    Datasets include machine learning datasets, function inputs/outputs, lexicons.
    """

    type = models.CharField(max_length=64)
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, null=True, blank=True)
    created_at = models.DateTimeField(auto_now_add=True)

    organization: models.ForeignKey = models.ForeignKey(
        "bench.Organization", on_delete=models.CASCADE, related_name="datasets"
    )
    project = models.ForeignKey("bench.Project", on_delete=models.CASCADE, related_name="datasets")
    objects = DatasetManager()

    def __str__(self):
        return f"{self.organization.slug}/{self.name}"

    class Meta:
        indexes = [
            models.Index(name="bench_dataset_type_idx", fields=["type"]),
            models.Index(name="bench_dataset_name_idx", fields=["name"]),
        ]
        constraints = [
            # check that the name is unique within the project
            models.UniqueConstraint(
                name="bench_dataset_project_name_ak", fields=["project_id", "name"]
            ),
        ]


@dataclasses.dataclass
class DatasetSearch:
    text_like: Optional[str] = None


class DatasetVersion(VersionedTree, TaggableMixin, UUIDModel):
    """
    A version of a JSON dataset.
    """

    dataset = models.ForeignKey(Dataset, on_delete=models.CASCADE, related_name="versions")
    project_version = models.ForeignKey(
        "ProjectVersion", on_delete=models.CASCADE, related_name="dataset_versions"
    )
    # records from DatasetRecord.dataset
    record_schema = models.JSONField()
    length = models.IntegerField(default=0)

    def __str__(self):
        return f"{self.organization.slug}/{self.name_version}"

    @property
    def organization(self):
        return self.dataset.organization

    @property
    def name_version(self) -> str:
        return f"{self.dataset.name}@{self.content_hash}"

    def search_records(
        self, search: DatasetSearch, limit: int, offset: int
    ) -> Sequence[DatasetRecord]:
        raise NotImplementedError

    def get_record(self, index: int) -> DatasetRecord:
        return DatasetRecord.objects.get(dataset=self, index=index)

    def get_records_view(self, view_data: Optional[DatasetViewData]) -> list[DatasetRecord]:
        if view_data is None:
            return self.get_records_slice(0, len(self))
        else:
            # note that this only works with slice-based views
            return self.get_records_slice(view_data.apply(0), view_data.apply(len(self)))

    def get_records_slice(self, start: int, stop: Optional[int] = None) -> list[DatasetRecord]:
        stop = stop if stop is not None else 0
        return DatasetRecord.objects.filter(dataset=self)[start:stop]

    def get_records_data_field(self, key: str) -> list[Any]:
        return DatasetRecord.objects.filter(dataset=self).values_list("data__" + key, flat=True)

    def __iter__(self) -> Iterator[DatasetRecord]:
        for record in DatasetRecord.objects.filter(dataset=self):
            yield record

    def append(self, record: DatasetRecord) -> int:
        with transaction.atomic():
            DatasetRecord.objects.create(
                dataset=self, index=self.length, data=record.data, metadata=record.metadata
            )
            self.length += 1
            self.save()
        return self.length

    def extend(self, records: Iterable[DatasetRecord]) -> tuple[int, int]:
        start_length = self.length
        db_records = []
        # create db_records with incrementing index
        with transaction.atomic():
            for record in records:
                db_records.append(
                    DatasetRecord(
                        dataset=self, index=self.length, data=record.data, metadata=record.metadata
                    )
                )
                self.length += 1
            DatasetRecord.objects.bulk_create(db_records)
            self.save()

        return start_length, self.length

    def update(self, index: int, record: DatasetRecord):
        with transaction.atomic():
            db_record = self.get_record(index)
            db_record.data = record.data
            db_record.metadata = record.metadata
            db_record.save()

    def delete_(self, index: int):
        with transaction.atomic():
            db_record = self.get_record(index)
            db_record.delete()
            # update the index of all records indices after the deleted one
            with connection.cursor() as cursor:
                cursor.execute(
                    "update bench_datasetrecord"
                    " set index = index - 1"
                    " where dataset_id = %s and index > %s",
                    [self.dataset.id, index],
                )
            self.length -= 1
            self.save()

    def clear(self):
        with transaction.atomic():
            DatasetRecord.objects.filter(dataset=self).delete()
            self.length = 0
            self.save()

    def __len__(self) -> int:
        return self.length

    @staticmethod
    def checkout_from_parents(dataset: DatasetVersion):
        parents: QuerySet[DatasetVersion] = proxies(dataset.parents.all(), DatasetVersion)
        if parents:
            # this should be caught in DatasetVersionSerializer validation
            if len(parents) != 1:
                raise RuntimeError("creating versions with multiple parents is not supported yet")

            # note: parent metadata is copied in ArtifactVersionViewSet.create prior to actual create
            # create a new version of the dataset state based on the parent
            # TODO @Architecture: creating new version logic should be elsewhere (DatasetAccessor?)
            #  because it is a shared concern and needs to drill down into (partial) sub-datasets.
            parent = parents[0]
            parent.checkout(dataset.version)
        return dataset

    def commit(self):
        if self.committed:
            raise ValueError(f"cannot commit dataset, already committed: {self.dataset}")

        self.committed = True
        self.save()

    class Meta:
        constraints = [
            models.UniqueConstraint(
                name="bench_dataset_version_dataset_content_hash_ak",
                fields=["dataset", "content_hash"],
            ),
        ]


class DatasetRecord(UUIDModel, VersionedBlob):
    """
    An individual immutable record of a dataset-like Artifact.
    """

    dataset = models.ForeignKey("DatasetVersion", on_delete=models.CASCADE, related_name="records")
    index = models.IntegerField()
    data = models.JSONField()
    metadata = models.JSONField(null=True, blank=True)

    def is_committed(self) -> bool:
        return True

    class Meta:
        # order by index ascending by default
        ordering = ["index"]
        constraints = [
            models.UniqueConstraint(
                fields=["dataset", "index"], name="bench_record_dataset_index_ak"
            ),
        ]
        indexes = [
            GinIndex(SearchVector("data", config="simple"), name="bench_record_data"),
            GinIndex(SearchVector("metadata", config="simple"), name="bench_record_metadata"),
        ]


class DatasetView(TaggableMixin, UUIDModel):
    """
    A view of a Dataset.

    If this view works only with specific versions, then it must specify the compatible
    versions in 'compatible_versions'. If empty, we assume it works with all versions.
    """

    type = models.CharField(max_length=64)
    dataset = models.ForeignKey(Dataset, on_delete=models.CASCADE, related_name="views")
    compatible_versions = models.ManyToManyField(DatasetVersion, related_name="views")
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH, null=True, blank=True)
    created_at = models.DateTimeField(auto_now_add=True)
    data = models.JSONField()

    def __str__(self):
        return f"{self.name}[{self.name}]"


# TODO @Feature: support more complex dataset views (e.g. filters)
@dataclass
class DatasetViewData:
    start: Optional[int] = None
    end: Optional[int] = None

    @property
    def asdict(self) -> dict:
        return dataclasses.asdict(self)

    @staticmethod
    def empty() -> DatasetViewData:
        return DatasetViewData.from_slice((0, 0))

    @staticmethod
    def from_slice(slice: tuple[int, int]) -> DatasetViewData:
        return DatasetViewData(start=slice[0], end=slice[1])

    def apply(self, index: int) -> int:
        if self.start is not None:
            index += self.start
        if self.end is not None and index >= self.end:
            return self.end
        return index
