from typing import Optional
from uuid import UUID

import strawberry
import strawberry_django
from asgiref.sync import sync_to_async
from strawberry import relay
from strawberry.relay import GlobalID, PageInfo
from strawberry.scalars import JSON
from strawberry.types import Info
from strawberry_django.fields.types import OperationInfo

from bench import models
from bench.api.auth import check_module_node_access
from bench.api.sync import bench_edit
from bench.api.type import MET
from bench.api.utils import (
    Conditional,
    HasCrud,
    ListConnectionWithTotalCount,
    Revisioned,
    Sort,
    ThingBatch,
)
from bench.language import wire
from bench.models import ModuleAccessLevel, packer
from bench.msg import NMessage
from bench.msg.core import request
from bench.msg.messages import NMessageType, RepSearchRecordsPayload, ReqSearchRecordsPayload
from bench.search import mirror
from bench.utils.dt import utcnow_with_tz


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


@strawberry.type
class RecordMutation:
    @bench_edit(MET.CREATE_RECORD)
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

    @bench_edit(MET.UPDATE_RECORD)
    def update_record(self, input: RecordUpdateInput) -> Record | OperationInfo:
        record = models.Record.objects.get(id=UUID(input.id.node_id))
        record.value = input.value
        return record

    @bench_edit(MET.SOFT_DELETE_RECORD)
    def soft_delete_record(self, input: RecordDeleteInput) -> Record | OperationInfo:
        record = models.Record.objects.get(id=UUID(input.id.node_id))
        record.deleted_at = utcnow_with_tz()
        return record

    @bench_edit(MET.RESTORE_RECORD)
    def restore_record(self, input: RecordRestoreInput) -> Record | OperationInfo:
        record = models.Record._base_manager.get(id=UUID(input.id.node_id))
        record.deleted_at = None
        return record

    @bench_edit(MET.DELETE_RECORD)
    def delete_record(self, input: RecordDeleteInput) -> Record | OperationInfo:
        raise NotImplementedError


RECORDS_LIMIT = 100


@strawberry.type
class RecordQuery:
    @strawberry_django.field
    async def search_records(
        self,
        info: Info,
        statement_id: GlobalID,
        query: Optional[Conditional] = None,
        sort: Optional[list[Sort]] = None,
        after: Optional[str] = None,
        limit: Optional[int] = None,
        count: Optional[bool] = None,
    ) -> ListConnectionWithTotalCount[Record]:
        statement = await models.Statement.objects.aget(id=statement_id.node_id)
        access = await sync_to_async(check_module_node_access)(
            info, statement, ModuleAccessLevel.Read
        )

        query = query.to_bench() if query else None
        sort = [s.to_bench() for s in sort] if sort else None
        effective_limit = min(limit or RECORDS_LIMIT, RECORDS_LIMIT)
        req = ReqSearchRecordsPayload(
            module_id=access.project_version.id,
            statement_id=statement.id,
            statement_ck=statement.ck,
            statement_key=statement.key,
            query=wire.pack_data(query) if query is not None else None,
            sort=[wire.pack_data(s) for s in sort] if sort is not None else None,
            limit=effective_limit + 1,
            after=after,
            count=count or False,
        )
        rep: NMessage[RepSearchRecordsPayload] = await request(NMessageType.SEARCH_RECORDS, req)
        if rep.p.records is not None:
            records = [packer.unpack_node_flat(r, statement) for r in rep.p.records]
            edges = []
            for cursor, record in zip(rep.p.cursors, records):
                node = Record.from_os(record)
                edge = relay.Edge(node=node, cursor=cursor)
                edges.append(edge)
        else:
            edges = []
        page_info = PageInfo(
            start_cursor=rep.p.cursors[0] if rep.p.cursors else None,
            end_cursor=rep.p.cursors[-1] if rep.p.cursors else None,
            has_next_page=bool(rep.p.records) and len(rep.p.records) > effective_limit,
            has_previous_page=False,
        )
        return ListConnectionWithTotalCount(
            edges=edges, page_info=page_info, total_count=rep.p.total
        )
