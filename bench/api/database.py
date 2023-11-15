from typing import Optional
from uuid import UUID

import strawberry
import strawberry_django
from strawberry import relay
from strawberry.relay import GlobalID
from strawberry.scalars import JSON
from strawberry.types import Info
from strawberry_django.fields.types import OperationInfo

from bench import language as lang
from bench import models
from bench.api.auth import check_module_node_access
from bench.api.sync import BatchEditInput, db_edit
from bench.api.type import MET
from bench.api.utils import (
    Conditional,
    HasCrud,
    ListConnectionWithTotalCount,
    Revisioned,
    Sort,
    ThingBatch,
)
from bench.language import ConditionalOp
from bench.models import ModuleAccessLevel
from bench.search import mirror
from bench.search.client import os_client_sync
from bench.search.core import DocumentType
from bench.search.mapping import encode_cursor, prepare_os_query


@strawberry_django.type(models.Record)
class Record(HasCrud, Revisioned, relay.Node):
    ck: UUID
    value: JSON

    @staticmethod
    def from_os(record: mirror.Record) -> models.Record:
        return models.Record(
            id=record.id,
            ck=record.ck,
            value=record.value,
            statement_id=record.statement_id,
            statement_ck=record.statement_ck,
            revision=record.revision,
            created_at=record.created_at,
            created_by=None,
            updated_at=record.updated_at,
            deleted_at=record.deleted_at if record.deleted_at != "-" else None,
            last_edited_at=record.last_edited_at,
            last_edited_by=None,
        )


@strawberry.type
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


@strawberry.input
class RecordInput:
    statement_id: GlobalID


@strawberry.input
class RecordCreateInput(RecordInput, strawberry_django.NodeInput):
    ck: UUID
    value: JSON
    statement_ck: UUID
    statement_key: str


@strawberry.input
class RecordUpdateInput(RecordInput, strawberry_django.NodeInput):
    value: JSON


@strawberry.input
class RecordUpdatePathInput(RecordInput, strawberry_django.NodeInput):
    path: str
    value: Optional[JSON] = None


@strawberry.input
class RecordDeleteInput(RecordInput, strawberry_django.NodeInput):
    pass


@strawberry.input
class RecordRestoreInput(RecordInput, strawberry_django.NodeInput):
    pass


@strawberry.input
class RecordBatchSoftDeleteInput(RecordInput, BatchEditInput):
    ids: list[GlobalID]

    def unbatch(self) -> list:
        return [RecordDeleteInput(statement_id=self.statement_id, id=i) for i in self.ids]


@strawberry.input
class RecordBatchRestoreInput(RecordInput, BatchEditInput):
    ids: list[GlobalID]

    def unbatch(self) -> list:
        return [RecordRestoreInput(statement_id=self.statement_id, id=i) for i in self.ids]


@strawberry.type
class RecordMutation:
    # nocheckin: reroute db_edits to runtime host
    @db_edit(MET.CREATE_RECORD)
    def create_record(self, input: RecordCreateInput) -> Record | OperationInfo:
        record = models.Record(
            id=UUID(input.id.node_id),
            ck=input.ck,
            statement_id=UUID(input.statement_id.node_id),
            statement_ck=input.statement_ck,
            statement_key=input.statement_key,
            value=input.value,
        )
        return record

    @db_edit(MET.UPDATE_RECORD)
    def update_record(self, input: RecordUpdateInput) -> Record | OperationInfo:
        record = models.Record.objects.get(id=UUID(input.id.node_id))
        record.value = input.value
        return record

    @db_edit(MET.SOFT_DELETE_RECORD)
    def soft_delete_record(self, input: RecordDeleteInput) -> Record | OperationInfo:
        record = models.Record.objects.get(id=UUID(input.id.node_id))
        record.soft_delete()
        return record

    @db_edit(MET.RESTORE_RECORD)
    def restore_record(self, input: RecordRestoreInput) -> Record | OperationInfo:
        record = models.Record._base_manager.get(id=UUID(input.id.node_id))
        record.restore()
        return record

    @db_edit(MET.DELETE_RECORD)
    def delete_record(self, input: RecordDeleteInput) -> Record | OperationInfo:
        record = models.Record.objects.get(id=UUID(input.id.node_id))
        record.delete()
        return record


RECORDS_LIMIT = 100


@strawberry.type
class RecordQuery:  # avoid name conflict with DatabaseQuery
    @strawberry_django.field
    def search_records(
        self,
        info: Info,
        statement_id: GlobalID,
        query: Optional[Conditional] = None,
        sort: Optional[list[Sort]] = None,
        after: Optional[str] = None,
        limit: Optional[int] = None,
        count: Optional[bool] = None,
    ) -> ListConnectionWithTotalCount[Record]:
        # nocheckin: reroute api record search to runtime host
        statement = models.Statement.objects.get(id=statement_id.node_id)
        access = check_module_node_access(info, statement, ModuleAccessLevel.Read)

        query = query.to_dsl() if query else None
        query = lang.Conditional.and_if_set(
            lang.C(ConditionalOp.EQUALS, "statement_key", value=statement.key), query
        )
        effective_limit = min(limit or RECORDS_LIMIT, RECORDS_LIMIT)
        search = prepare_os_query(
            type=DocumentType.RECORD,  # already limited by database
            project_version_id=None,  # already limited by database
            limit=effective_limit + 1,  # +1 to determine if there is a next page
            count=count or False,
            after=after,
            sort=[s.to_dsl() for s in sort] if sort else None,
            query=query,
        )

        results = os_client_sync.search(index=access.project.os_name, body=search)

        edges = []
        for i, r in enumerate(results["hits"]["hits"][0:effective_limit]):
            doc = mirror.Record.from_dict(r["_source"], r["_id"])
            node = Record.from_os(doc)
            cursor = encode_cursor(r, after, i)
            edge = relay.Edge(node=node, cursor=cursor)
            edges.append(edge)
        page_info = relay.PageInfo(
            start_cursor=edges[0].cursor if edges else None,
            end_cursor=edges[-1].cursor if edges else None,
            has_next_page=len(results["hits"]["hits"]) > effective_limit,
            has_previous_page=False,
        )
        total_count = results["hits"]["total"]["value"] if count else None
        return ListConnectionWithTotalCount(
            edges=edges, page_info=page_info, total_count=total_count
        )
