from __future__ import annotations

from typing import Iterable, Iterator, Optional, Tuple, cast

from django.contrib.postgres.indexes import GinIndex
from django.contrib.postgres.search import SearchVector
from django.db import models, transaction
from django.db.models import Q, QuerySet, Subquery

from bench.models.utils import UUIDModel
from bench.models.versioning import VersionedBlob, VersionedObject, VersionedTree
from bench.utils.record import Record
from bench.utils.spec import FieldValue


class DbRecordManager(models.Manager):
    def create_record(self, data: dict, metadata: Optional[dict]) -> DbRecord:
        content_hash = VersionedObject.hash_content({"data": data})
        return super().create(data=data, metadata=metadata, content_hash=content_hash)


class DbRecord(UUIDModel, VersionedBlob):
    """
    An individual immutable record of a dataset-like Artifact.

    Data represents the in-DB part of the record's data corresponding to the artifact's
    schema, while metadata is additional data derived from the data, added by a user
    or function or any other information not strictly part of the data.
    Both data and metadata may include pointers to the artifact storage.
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


class DbRecordTree(UUIDModel, VersionedTree):
    """
    A tree referring to other records or subtrees with contiguous indices.
    """

    max_index = models.IntegerField(default=-1)


# TODO @Storage: change RecordTreeReference.id from UUID to int
class DbRecordTreeReference(UUIDModel):
    """
    A reference from a 'tree' to either a record or a subtree at a relative index.
    """

    tree = models.ForeignKey(DbRecordTree, on_delete=models.CASCADE, related_name="references")
    index = models.IntegerField()
    record = models.ForeignKey(
        DbRecord, on_delete=models.RESTRICT, null=True, blank=True, related_name="+"
    )
    subtree = models.ForeignKey(
        DbRecordTree, on_delete=models.CASCADE, null=True, blank=True, related_name="+"
    )

    class Meta:
        ordering = ["tree", "index"]
        constraints = [
            models.UniqueConstraint(
                name="bench_record_tree_reference_index_ak", fields=["tree", "index"]
            ),
            models.CheckConstraint(
                name="bench_record_tree_reference_set",
                check=Q(record__isnull=True, subtree__isnull=False)
                | Q(record__isnull=False, subtree__isnull=True),
            ),
        ]


def get_parent_tree(tree: DbRecordTree, index: int) -> Tuple[DbRecordTree, DbRecordTreeReference]:
    """
    Gets the immediate parent tree for the record at the index (relative to this tree)

    Record trees are .. trees, so we may need to dig down to get a parent tree:
     - If index exists exactly on this tree and is a record
       -> return record
     - If index doesn't exist or points to a subtree
       -> recurse and subtract relative index
    """

    if index < 0 or index > tree.max_index:
        raise LookupError(f"tree [0, {tree.max_index}] does not contain index {index}")
    reference: Optional[DbRecordTreeReference] = (
        DbRecordTreeReference.objects.filter(tree=tree, index__gte=index)
        .order_by("index")
        .select_related("subtree")
        .select_related("record")
        .first()
    )
    if reference is None:
        # Tree is not contiguous, this should never happen.
        raise RuntimeError(f"tree [0, {tree.max_index}] is missing {index}")
    elif reference.subtree:
        return get_parent_tree(reference.subtree, index - reference.index)
    else:
        return tree, reference


def get_parent_trees(
    tree: DbRecordTree, start: int, end: int
) -> Tuple[DbRecordTree, Iterable[DbRecordTreeReference]]:
    """
    Gets the immediate parent tree for the record at the index (relative to this tree)

    Unlike get_parent_tree, this method does not currently deal with nested trees.
    """

    if start < 0 or end > (tree.max_index + 1):
        raise LookupError(f"tree [0, {tree.max_index}] does not contain range [{start}:{end}]")
    references: QuerySet[DbRecordTreeReference] = (
        DbRecordTreeReference.objects.filter(tree=tree, index__gte=start, index__lt=end)
        .order_by("index")
        .select_related("record")[: (end - start + 1)]
    )
    return tree, references


def get_record(tree: DbRecordTree, index: int) -> DbRecord:
    """
    Gets the Record at the given index within the tree
    """
    parent_tree, reference = get_parent_tree(tree, index)
    # must be non-null because of get_parent_tree
    return cast(DbRecord, reference.record)


def get_records_slice(tree: DbRecordTree, start: int, end: int) -> list[DbRecord]:
    """
    Gets the Records in the given range within the tree
    TODO @Performance: optimise get_records_slice to remove redundant queries
    """
    # map start/stop to bounds
    start = max(start, 0)
    end = min(end, tree.max_index + 1)
    _, references = get_parent_trees(tree, start, end)
    # ref.record must be non-null due to get_parent_trees
    return [cast(DbRecord, ref.record) for ref in references]


def get_records_field(tree: DbRecordTree, field: str) -> list[FieldValue]:
    """
    Gets specific fields of the given tree within
    """
    # TODO @Performance: optimise get_records_field to perform query in DB
    field_values: list[FieldValue] = []
    for record in iter_record_tree(tree):
        field_values.append(record.data[field])
    return field_values


def append_record(tree: DbRecordTree, record: DbRecord) -> int:
    """
    Appends the given record to the "end" of the given tree.
    Changes are immediately persisted to the DB.
    """
    with transaction.atomic():
        insert_index = tree.max_index + 1
        tree.references.create(index=insert_index, record=record)
        tree.max_index = insert_index
        tree.save()
    return insert_index


def append_records(tree: DbRecordTree, records: list[DbRecord]) -> tuple[int, int]:
    """
    Appends the given records to the "end" of the given tree in order.
    Changes are immediately persisted to the DB.
    """
    with transaction.atomic():
        insert_offset = tree.max_index + 1
        references = []
        for i in range(0, len(records)):
            references.append(
                DbRecordTreeReference(tree_id=tree.id, index=insert_offset + i, record=records[i])
            )
        DbRecordTreeReference.objects.bulk_create(references)
        tree.max_index = tree.max_index + len(records)
        tree.save()
    return insert_offset, tree.max_index + 1


def update_record(
    tree: DbRecordTree, index: int, data: Record, metadata: Optional[Record]
) -> DbRecord:
    """
    Updates the record at the given index (by replacing it with a new one with the given data)
    """
    ds_record = get_record(tree, index)
    ds_record = copy_record_with(ds_record, data=data, metadata=metadata)
    # replace current record reference with new record reference
    replace_record(tree, index, new_record=ds_record)
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


def replace_record(tree: DbRecordTree, index: int, new_record: DbRecord):
    """
    Replaces the current record at the given index in the tree with the new record
    """
    parent_tree, reference = get_parent_tree(tree, index)
    with transaction.atomic():
        old_record: DbRecord = cast(DbRecord, reference.record)
        reference.record = new_record
        reference.save()

    gc_unused_records(
        records=DbRecord.objects.filter(id=old_record.id),
        references=DbRecordTreeReference.objects.all(),
    )


def delete_record(tree: DbRecordTree, index: int):
    raise NotImplementedError


def clear_record_tree(tree: DbRecordTree):
    """
    Clears the given record tree (non-recursively)
    """
    DbRecordTreeReference.objects.filter(tree=tree).delete()
    # TODO @Performance: gc only recently de-referenced records when deleting record tree
    gc_unused_records_all()
    tree.max_index = -1
    tree.save()


def gc_unused_records_all():
    """
    Deletes unused records, considering all records and references
    """
    gc_unused_records(
        records=DbRecord.objects.all(), references=DbRecordTreeReference.objects.all()
    )


def gc_unused_records(
    records: models.QuerySet[DbRecord], references: models.QuerySet[DbRecordTreeReference]
):
    """
    Deletes unused records not referenced in the given references
    """
    used_records_ids = references.values_list("record_id")
    records.exclude(id__in=Subquery(used_records_ids)).delete()


def iter_record_tree(tree: DbRecordTree) -> Iterator[DbRecord]:
    """
    Lazily iterates through all records in the given tree (and its subtrees).
    """
    for reference in tree.references.all():
        if reference.subtree:
            yield from iter_record_tree(reference.subtree)
        elif reference.record:
            yield reference.record
        else:
            raise RuntimeError(f"malformed reference: {reference}")
