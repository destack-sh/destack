from datetime import datetime
from typing import Optional
from uuid import UUID

import pytz
from strawberry.scalars import JSON
from strawberry.types import Info
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID
from strawberry_django_plus.types import OperationInfo
from strawberry_django_plus.utils.resolvers import async_safe

from bench import models
from bench.api.auth import check_can_read_project
from bench.api.sync import BatchMutationInput, check_can_write_thing, tracked_os_mutation
from bench.api.type import MMT
from bench.api.utils import CrudModel, Revisioned, ThingBatch, to_global_id
from bench.opensearch import mirror
from bench.opensearch.client import os_client
from bench.opensearch.core import IndexType
from bench.opensearch.index import batch_update_records, create_record, delete_record, update_record


@gql.django.type(models.Dataset)
class Dataset(gql.Node):
    backend: str
    backend_id: str
    versioned: bool


@gql.type
class Record(CrudModel, Revisioned):
    id: GlobalID
    dataset_id: str
    order_key: str
    data: JSON

    @staticmethod
    def from_os(record: mirror.Record) -> "Record":
        return Record(
            id=to_global_id("Record", record.id),
            dataset_id=record.dataset_id,
            order_key=record.order_key,
            data=record.data,
            revision=record.revision,
            created_at=record.created_at,
            created_by=None,
            updated_at=record.updated_at,
            deleted_at=record.deleted_at if record.deleted_at != "-" else None,
            last_edited_at=record.last_edited_at,
            last_edited_by=None,
        )


@gql.type
class RecordBatch(ThingBatch):
    records: list[Record]

    @property
    def things(self):
        return self.records

    @staticmethod
    def from_os(batch: "RecordBatch") -> "RecordBatch":
        return RecordBatch(
            records=[Record.from_os(r) for r in batch.records],
        )


@gql.input
class RecordInput:
    statement_id: GlobalID


@gql.input
class RecordCreateInput(RecordInput, gql.NodeInput):
    data: JSON
    order_key: str


@gql.input
class RecordUpdateInput(RecordInput, gql.NodeInput):
    data: JSON


@gql.input
class RecordUpdatePathInput(RecordInput, gql.NodeInput):
    path: str
    data: Optional[JSON] = None


@gql.input
class RecordMoveInput(RecordInput, gql.NodeInput):
    order_key: str


@gql.input
class RecordDeleteInput(RecordInput, gql.NodeInput):
    pass


@gql.input
class RecordRestoreInput(RecordInput, gql.NodeInput):
    pass


@gql.input
class RecordBatchSoftDeleteInput(RecordInput, BatchMutationInput):
    ids: list[GlobalID]

    def unbatch(self) -> list:
        return [RecordDeleteInput(id=i) for i in self.ids]


@gql.input
class RecordBatchRestoreInput(RecordInput, BatchMutationInput):
    ids: list[GlobalID]

    def unbatch(self) -> list:
        return [RecordRestoreInput(id=i) for i in self.ids]


def _prep_write_dataset(
    info: Info, input: RecordInput
) -> tuple[datetime, models.ProjectVersion, models.Statement]:
    statement = models.Statement.objects.select_related("dataset").get(
        id=input.statement_id.node_id
    )
    project_v = check_can_write_thing(info, statement)
    now = datetime.utcnow().replace(tzinfo=pytz.utc)
    return now, project_v, statement


@gql.type
class DatasetMutation:
    @tracked_os_mutation(MMT.CREATE_RECORD)
    def create_record(self, info: Info, input: RecordCreateInput) -> Record | OperationInfo:
        now, project_v, statement = _prep_write_dataset(info, input)
        record = mirror.Record(
            id=UUID(input.id.node_id),
            project_version_id=project_v.id,
            statement_id=statement.id,
            dataset_id=statement.dataset.backend_id,
            created_at=now,
            created_by_id=None,  # not handled yet
            updated_at=now,
            deleted_at=None,
            last_edited_at=now,
            last_edited_by_id=None,  # not handled yet
            order_key=input.order_key,
            data=input.data,
        )
        record = create_record(project_v, record)
        return project_v, statement, record  # noqa (will be unwrapped)

    @tracked_os_mutation(MMT.UPDATE_RECORD)
    def update_record(self, info: Info, input: RecordUpdateInput) -> Record | OperationInfo:
        now, project_v, statement = _prep_write_dataset(info, input)
        record = mirror.Record.Partial(
            id=UUID(input.id.node_id),
            project_version_id=project_v.id,
            statement_id=statement.id,
            dataset_id=statement.dataset.backend_id,
            data=input.data,
            updated_at=now,
            last_edited_at=now,
        )
        record = update_record(project_v, record)
        return project_v, statement, record  # noqa

    @tracked_os_mutation(MMT.MOVE_RECORD)
    def move_record(self, info: Info, input: RecordMoveInput) -> Record | OperationInfo:
        now, project_v, statement = _prep_write_dataset(info, input)
        record = mirror.Record.Partial(
            id=UUID(input.id.node_id),
            project_version_id=project_v.id,
            statement_id=statement.id,
            dataset_id=statement.dataset.backend_id,
            order_key=input.order_key,
            updated_at=now,
            last_edited_at=now,
        )
        record = update_record(project_v, record)
        return project_v, statement, record  # noqa

    @tracked_os_mutation(MMT.SOFT_DELETE_RECORD)
    def soft_delete_record(self, info: Info, input: RecordDeleteInput) -> Record | OperationInfo:
        now, project_v, statement = _prep_write_dataset(info, input)
        record = mirror.Record.Partial(
            id=UUID(input.id.node_id),
            project_version_id=project_v.id,
            statement_id=statement.id,
            dataset_id=statement.dataset.backend_id,
            deleted_at=now,
            updated_at=now,
        )
        record = update_record(project_v, record)
        return project_v, statement, record  # noqa

    @tracked_os_mutation(MMT.RESTORE_RECORD)
    def restore_record(self, info: Info, input: RecordRestoreInput) -> Record | OperationInfo:
        now, project_v, statement = _prep_write_dataset(info, input)
        record = mirror.Record.Partial(
            id=UUID(input.id.node_id),
            project_version_id=project_v.id,
            statement_id=statement.id,
            dataset_id=statement.dataset.backend_id,
            deleted_at="-",  # invalid value to set to null
            updated_at=now,
        )
        record = update_record(project_v, record)
        return project_v, statement, record  # noqa

    @tracked_os_mutation(MMT.DELETE_RECORD)
    def delete_record(self, info: Info, input: RecordDeleteInput) -> Record | OperationInfo:
        now, project_v, statement = _prep_write_dataset(info, input)
        delete_record(project_v, UUID(input.id.node_id))
        return project_v, statement, None  # noqa

    @tracked_os_mutation(MMT.SOFT_DELETE_RECORD, batch=True, register=False)
    def batch_soft_delete_record(
        self, info: Info, input: RecordBatchSoftDeleteInput
    ) -> RecordBatch | OperationInfo:
        # imitate soft_delete_record but for a batch
        now, project_v, statement = _prep_write_dataset(info, input)
        records = [
            mirror.Record.Partial(
                id=UUID(i.node_id),
                project_version_id=project_v.id,
                statement_id=statement.id,
                dataset_id=statement.dataset.backend_id,
                deleted_at=now,
                updated_at=now,
            )
            for i in input.unbatch()
        ]
        records = batch_update_records(project_v, records)
        return project_v, statement, records  # noqa

    @tracked_os_mutation(MMT.RESTORE_RECORD, batch=True, register=False)
    def batch_restore_record(
        self, info: Info, input: RecordBatchRestoreInput
    ) -> RecordBatch | OperationInfo:
        # imitate restore_record but for a batch
        now, project_v, statement = _prep_write_dataset(info, input)
        records = [
            mirror.Record.Partial(
                id=UUID(i.node_id),
                project_version_id=project_v.id,
                statement_id=statement.id,
                dataset_id=statement.dataset.backend_id,
                deleted_at="-",  # invalid value to set to null
                updated_at=now,
            )
            for i in input.unbatch()
        ]
        records = batch_update_records(project_v, records)
        return project_v, statement, records  # noqa


@gql.type
class DatasetQuery:
    @gql.relay.connection
    @async_safe
    def search_records(
        self,
        info: Info,
        statement_id: GlobalID,
        before: Optional[str] = None,
        after: Optional[str] = None,
        first: Optional[int] = None,
        last: Optional[int] = None,
    ) -> gql.Connection[Record]:
        statement = models.Statement.objects.select_related("dataset").get(id=statement_id.node_id)
        check_can_read_project(info, statement.project_version)

        default_limit = 25
        base_filter = [
            {"term": {"dataset_id": statement.dataset.backend_id}},
            {"bool": {"must_not": {"exists": {"field": "deleted_at"}}}},
            # pagination here is broken when using a sort
            {"range": {"order_key": {"gt": after, "lt": before}}},
        ]
        query = {
            "bool": {"filter": base_filter},
        }
        sort = [{"order_key": "asc"}]
        results = os_client.search(
            index=IndexType.BENCH.get_index_name(statement.project_version.project_id),
            body={
                "size": first or last or default_limit,
                "query": query,
                "sort": sort,
                "track_total_hits": True,
                "version": True,
            },
        )

        edges = []
        for r in results["hits"]["hits"]:
            doc = mirror.Record.from_dict(r["_source"], r["_id"], r["_version"])
            node = Record.from_os(doc)
            edge = gql.relay.Edge(node=node, cursor=doc.order_key)
            edges.append(edge)

        page_info = gql.relay.PageInfo(
            start_cursor=edges[0].cursor if edges else None,
            end_cursor=edges[-1].cursor if edges else None,
            has_next_page=False,
            has_previous_page=False,
        )
        total_count = results["hits"]["total"]["value"]
        return gql.relay.Connection(edges=edges, page_info=page_info, total_count=total_count)
