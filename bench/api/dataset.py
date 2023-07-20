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
from bench.api.utils import CrudModel, Revisioned, SearchQuery, SearchSort, ThingBatch, to_global_id
from bench.language import Q, Query, QueryOp
from bench.opensearch import mirror
from bench.opensearch.client import os_client
from bench.opensearch.core import IndexType
from bench.opensearch.index import batch_update_records, create_record, delete_record, update_record
from bench.opensearch.query import encode_cursor, prepare_search


@gql.django.type(models.Dataset)
class Dataset(gql.Node):
    backend: str
    backend_id: str
    versioned: bool


@gql.type
class Record(CrudModel, Revisioned):
    id: GlobalID
    dataset_id: str
    order_key: Optional[str]
    value: JSON

    @staticmethod
    def from_os(record: mirror.Record) -> "Record":
        return Record(
            id=to_global_id("Record", record.id),
            dataset_id=record.dataset_id,
            order_key=record.order_key,
            value=record.value,
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
    value: JSON
    order_key: Optional[str] = None


@gql.input
class RecordUpdateInput(RecordInput, gql.NodeInput):
    value: JSON


@gql.input
class RecordUpdatePathInput(RecordInput, gql.NodeInput):
    path: str
    value: Optional[JSON] = None


@gql.input
class RecordMoveInput(RecordInput, gql.NodeInput):
    order_key: Optional[str] = None


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
        return [RecordDeleteInput(statement_id=self.statement_id, id=i) for i in self.ids]


@gql.input
class RecordBatchRestoreInput(RecordInput, BatchMutationInput):
    ids: list[GlobalID]

    def unbatch(self) -> list:
        return [RecordRestoreInput(statement_id=self.statement_id, id=i) for i in self.ids]


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
            value=input.value,
            revision=None,  # not set yet
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
            value=input.value,
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


RECORDS_LIMIT = 100


@gql.type
class DataQuery:  # avoid name conflict with DatasetQuery
    @gql.relay.connection
    @async_safe
    def search_dataset(
        self,
        info: Info,
        statement_id: GlobalID,
        query: Optional[SearchQuery] = None,
        sort: Optional[list[SearchSort]] = None,
        after: Optional[str] = None,
        limit: Optional[int] = None,
        count: Optional[bool] = None,
    ) -> gql.Connection[Record]:
        statement = models.Statement.objects.select_related("dataset").get(id=statement_id.node_id)
        check_can_read_project(info, statement.project_version)

        query = query.to_dsl() if query else None
        query = Query.and_if_set(
            Q(QueryOp.EQUALS, "dataset_id", statement.dataset.backend_id), query
        )
        effective_limit = min(limit or RECORDS_LIMIT, RECORDS_LIMIT)
        search = prepare_search(
            type=mirror.DocumentType.RECORD,  # already limited by dataset
            project_version_id=None,  # already limited by dataset
            limit=effective_limit + 1,  # +1 to determine if there is a next page
            count=count or False,
            after=after,
            sort=[s.to_dsl() for s in sort] if sort else None,
            query=query,
        )

        results = os_client.search(
            index=IndexType.BENCH.get_index_name(statement.project_version.project_id),
            body=search,
        )

        edges = []
        for i, r in enumerate(results["hits"]["hits"][0:effective_limit]):
            doc = mirror.Record.from_dict(r["_source"], r["_id"], r["_version"])
            node = Record.from_os(doc)
            cursor = encode_cursor(r, after, i)
            edge = gql.relay.Edge(node=node, cursor=cursor)
            edges.append(edge)
        page_info = gql.relay.PageInfo(
            start_cursor=edges[0].cursor if edges else None,
            end_cursor=edges[-1].cursor if edges else None,
            has_next_page=len(results["hits"]["hits"]) > effective_limit,
            has_previous_page=False,
        )
        total_count = results["hits"]["total"]["value"] if count else None
        return gql.relay.Connection(edges=edges, page_info=page_info, total_count=total_count)
