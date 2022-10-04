from __future__ import annotations

from typing import Iterable, Iterator, Optional, Tuple, cast

from django.contrib.postgres.indexes import GinIndex
from django.contrib.postgres.search import SearchVector
from django.db import models, transaction
from django.db.models import QuerySet, Subquery

from bench.models.utils import UUIDModel
from bench.models.versioning import VersionedBlob, VersionedObject, VersionedTree
from bench.utils.record import Record
from bench.utils.spec import FieldValue


class DbRecordManager(models.Manager):
    def create_record(self, data: dict, metadata: Optional[dict]) -> DbRecord:
        content_hash = VersionedObject.hash_content(data)
        return super().create(data=data, metadata=metadata, content_hash=content_hash)


class DbRecord(UUIDModel, VersionedBlob):
    """
    An individual immutable record of a dataset-like Artifact.

    Data represents the in-DB part of the record's data corresponding to the artifact's
    schema, while metadata is additional data derived from or relevant to the data.
    Both data and metadata may include pointers to outside-DB storage.
    """

    data = models.JSONField()
    metadata = models.JSONField(null=True, blank=True)

    def is_committed(self) -> bool:
        return True

    def save(self, *args, **kwargs):
        # set content hash if not yet set
        if not self.content_hash and self._state.adding:
            self.content_hash = VersionedObject.hash_content(self.data)
        super().save(*args, **kwargs)

    class Meta:
        indexes = [
            GinIndex(SearchVector("data", config="simple"), name="bench_record_data"),
            GinIndex(SearchVector("metadata", config="simple"), name="bench_record_metadata"),
        ]


class DbRecordList(UUIDModel, VersionedTree):
    """
    A rlist referring to other records with contiguous indices.
    """

    max_index = models.IntegerField(default=-1)


# TODO @Storage: change RecordrlistReference.id from UUID to int
class DbRecordListReference(UUIDModel):
    """
    A reference from a 'rlist' to a record at a relative index.
    """

    rlist = models.ForeignKey(DbRecordList, on_delete=models.CASCADE, related_name="references")
    index = models.IntegerField()
    record = models.ForeignKey(DbRecord, on_delete=models.RESTRICT, related_name="+")

    class Meta:
        ordering = ["rlist", "index"]
        constraints = [
            models.UniqueConstraint(
                name="bench_record_rlist_reference_index_ak", fields=["rlist", "index"]
            ),
        ]


def get_record_reference(rlist: DbRecordList, index: int) -> DbRecordListReference:
    """
    Gets the reference to a specific record index in a record list
    """

    if index < 0 or index > rlist.max_index:
        raise LookupError(f"rlist [0, {rlist.max_index}] does not contain index {index}")
    reference: Optional[DbRecordListReference] = (
        DbRecordListReference.objects.filter(rlist=rlist, index__gte=index)
        .order_by("index")
        .select_related("record")
        .first()
    )
    if reference is None:
        # rlist is not contiguous, this should never happen.
        raise RuntimeError(f"rlist [0, {rlist.max_index}] is missing {index}")
    else:
        return reference


def get_parent_rlists(
    rlist: DbRecordList, start: int, end: int
) -> Tuple[DbRecordList, Iterable[DbRecordListReference]]:
    """
    Gets the list for the record at the index (relative to this rlist)
    """

    if start < 0 or end > (rlist.max_index + 1):
        raise LookupError(f"rlist [0, {rlist.max_index}] does not contain range [{start}:{end}]")
    references: QuerySet[DbRecordListReference] = (
        DbRecordListReference.objects.filter(rlist=rlist, index__gte=start, index__lt=end)
        .order_by("index")
        .select_related("record")[: (end - start + 1)]
    )
    return rlist, references


def get_record(rlist: DbRecordList, index: int) -> DbRecord:
    """
    Gets the Record at the given index within the rlist
    """
    reference = get_record_reference(rlist, index)
    return reference.record


def get_records_slice(rlist: DbRecordList, start: int, end: int) -> list[DbRecord]:
    """
    Gets the Records in the given range within the rlist
    TODO @Performance: optimise get_records_slice to remove redundant queries
    """
    # map start/stop to bounds
    start = max(start, 0)
    end = min(end, rlist.max_index + 1)
    _, references = get_parent_rlists(rlist, start, end)
    # ref.record must be non-null due to get_parent_rlists
    return [ref.record for ref in references]


def get_records_field(rlist: DbRecordList, field: str) -> list[FieldValue]:
    """
    Gets specific fields of the given rlist within
    """
    # TODO @Performance: optimise get_records_field to perform query in DB
    field_values: list[FieldValue] = []
    for record in iter_record_list(rlist):
        field_values.append(record.data[field])
    return field_values


def append_record(rlist: DbRecordList, record: DbRecord) -> int:
    """
    Appends the given record to the "end" of the given rlist.
    Changes are immediately persisted to the DB.
    """
    with transaction.atomic():
        insert_index = rlist.max_index + 1
        rlist.references.create(index=insert_index, record=record)
        rlist.max_index = insert_index
        rlist.save()
    return insert_index


def append_records(rlist: DbRecordList, records: list[DbRecord]) -> tuple[int, int]:
    """
    Appends the given records to the "end" of the given rlist in order.
    Changes are immediately persisted to the DB.
    """
    with transaction.atomic():
        insert_offset = rlist.max_index + 1
        references = []
        for i in range(0, len(records)):
            references.append(
                DbRecordListReference(rlist_id=rlist.id, index=insert_offset + i, record=records[i])
            )
        DbRecordListReference.objects.bulk_create(references)
        rlist.max_index = rlist.max_index + len(records)
        rlist.save()
    return insert_offset, rlist.max_index + 1


def update_record(
    rlist: DbRecordList, index: int, data: Record, metadata: Optional[Record]
) -> DbRecord:
    """
    Updates the record at the given index (by replacing it with a new one with the given data)
    """
    ds_record = get_record(rlist, index)
    ds_record = copy_record_with(ds_record, data=data, metadata=metadata)
    # replace current record reference with new record reference
    replace_record(rlist, index, new_record=ds_record)
    return ds_record


def copy_record_with(db_record: DbRecord, data: Record, metadata: Optional[Record]) -> DbRecord:
    # TODO @Storage: use content hashes to avoid creating duplicates?
    # "copy" record, replace data with new record data
    db_record.id = None
    db_record._state.adding = True
    db_record.data = data
    db_record.metadata = metadata
    db_record.save()
    return db_record


def replace_record(rlist: DbRecordList, index: int, new_record: DbRecord):
    """
    Replaces the current record at the given index in the rlist with the new record
    """
    parent_rlist, reference = get_record_reference(rlist, index)
    with transaction.atomic():
        old_record: DbRecord = cast(DbRecord, reference.record)
        reference.record = new_record
        reference.save()

    gc_unused_records(
        records=DbRecord.objects.filter(id=old_record.id),
        references=DbRecordListReference.objects.all(),
    )


def delete_record(rlist: DbRecordList, index: int):
    raise NotImplementedError


def clear_record_list(rlist: DbRecordList):
    """
    Clears the given record rlist (non-recursively)
    """
    DbRecordListReference.objects.filter(rlist=rlist).delete()
    # TODO @Performance: gc only recently de-referenced records when deleting record rlist
    gc_unused_records_all()
    rlist.max_index = -1
    rlist.save()


def gc_unused_records_all():
    """
    Deletes unused records, considering all records and references
    """
    gc_unused_records(
        records=DbRecord.objects.all(), references=DbRecordListReference.objects.all()
    )


def gc_unused_records(
    records: models.QuerySet[DbRecord], references: models.QuerySet[DbRecordListReference]
):
    """
    Deletes unused records not referenced in the given references
    """
    used_records_ids = references.values_list("record_id")
    records.exclude(id__in=Subquery(used_records_ids)).delete()


def iter_record_list(rlist: DbRecordList) -> Iterator[DbRecord]:
    """
    Lazily iterates through all records in the given rlist.
    """
    for reference in rlist.references.all():
        if reference.record:
            yield reference.record
        else:
            raise RuntimeError(f"malformed reference: {reference}")
