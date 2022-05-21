from typing import Iterator, Optional, Tuple, cast

from django.contrib.postgres.indexes import GinIndex
from django.db import models, transaction
from django.db.models import Q, Subquery

from bench.models.utils import UUIDModel
from bench.models.versioning import VersionedBlob, VersionedTree
from bench.utils.spec import FieldType


class Record(UUIDModel, VersionedBlob):
    """
    An individual record of a dataset-like Artifact.

    Data represents the in-DB part of the record's data corresponding to the artifact's
    schema, while metadata is additional data derived from the data, added by a user
    or function or any other information not strictly part of the data.
    Both data and metadata may include pointers to the artifact storage.
    """

    data = models.JSONField()
    metadata = models.JSONField(null=True, blank=True)
    committed = models.BooleanField(default=True)

    class Meta:
        indexes = [GinIndex(name="bench_record_metadata", fields=["metadata"])]


class RecordTree(UUIDModel, VersionedTree):
    """
    A tree referring to other records or subtrees with contiguous indices.
    """

    max_index = models.IntegerField(default=-1)


class RecordTreeReference(UUIDModel):
    """
    A reference from a 'tree' to either a record or a subtree at a relative index.
    """

    tree = models.ForeignKey(RecordTree, on_delete=models.CASCADE, related_name="references")
    index = models.IntegerField()
    record = models.ForeignKey(
        Record, on_delete=models.RESTRICT, null=True, blank=True, related_name="+"
    )
    subtree = models.ForeignKey(
        RecordTree, on_delete=models.CASCADE, null=True, blank=True, related_name="+"
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


def get_parent_tree(tree: RecordTree, index: int) -> Tuple[RecordTree, RecordTreeReference]:
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
    reference: Optional[RecordTreeReference] = (
        RecordTreeReference.objects.filter(tree=tree, index__gte=index)
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


def get_record(tree: RecordTree, index: int) -> Record:
    """
    Gets the Record at the given index within the tree
    """
    parent_tree, reference = get_parent_tree(tree, index)
    # must be non-null because of get_parent_tree
    return cast(Record, reference.record)


def get_records_slice(tree: RecordTree, start: int, stop: int) -> list[Record]:
    """
    Gets the Records in the given range within the tree
    TODO @Performance: optimise get_records_slice to remove redundant queries
    """
    # map start/stop to bounds
    start = max(start, 0)
    stop = min(stop, tree.max_index + 1)
    records = []
    for i in range(start, stop):
        records.append(get_record(tree, i))
    return records


def get_records_field(tree: RecordTree, index: str) -> list[FieldType]:
    """
    Gets specific fields of the given tree within
    """
    raise NotImplementedError


def append_record(tree: RecordTree, record: Record):
    """
    Appends the given record to the "end" of the given tree.
    Changes are immediately persisted to the DB.
    """
    with transaction.atomic():
        insert_index = tree.max_index + 1
        record.save()
        tree.references.create(index=insert_index, record=record)
        tree.max_index = insert_index
        tree.save()


def append_records(tree: RecordTree, records: list[Record]):
    """
    Appends the given records to the "end" of the given tree in order.
    Changes are immediately persisted to the DB.
    """
    with transaction.atomic():
        insert_offset = tree.max_index + 1
        references = []
        for i in range(0, len(records)):
            references.append(
                RecordTreeReference(tree=tree, index=insert_offset + i, record=records[i])
            )
        RecordTreeReference.objects.bulk_create(references)
        tree.max_index = insert_offset
        tree.save()


def delete_record(tree: RecordTree, index: int):
    raise NotImplementedError


def clear_record_tree(tree: RecordTree):
    """
    Clears the given record tree (non-recursively)
    """
    RecordTreeReference.objects.filter(tree=tree).delete()
    # TODO @Performance: gc only recently de-referenced records when deleting record tree
    gc_unused_records_all()


def gc_unused_records_all():
    """
    Deletes unused records, considering all records and references
    """
    gc_unused_records(records=Record.objects.all(), references=RecordTreeReference.objects.all())


def gc_unused_records(
    records: models.QuerySet[Record], references: models.QuerySet[RecordTreeReference]
):
    """
    Deletes unused records not referenced in the given references
    """
    used_records_ids = references.values_list("record_id")
    records.exclude(id__in=Subquery(used_records_ids)).delete()


def iter_record_tree(tree: RecordTree) -> Iterator[Record]:
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
